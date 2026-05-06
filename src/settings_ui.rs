// libcosmic settings GUI for tb-tray. Loaded only when the binary is
// invoked with `--settings`. Edits are kept in memory until the user clicks
// Save; the Autostart toggle is the exception — it writes/removes
// ~/.config/autostart/tb-tray.desktop immediately.

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::prelude::*;
use cosmic::widget::{self, space};
use cosmic::Action;

use tb_tray::autostart;
use tb_tray::config::{self, Account, Config};
use tb_tray::paths::{config_path, APP_ID};

pub fn run() -> cosmic::iced::Result {
    let settings = Settings::default()
        .size(Size::new(760.0, 680.0))
        .exit_on_close(true);
    cosmic::app::run::<App>(settings, ())
}

#[derive(Clone, Copy, Debug)]
pub enum AccountField {
    Name,
    Server,
    Port,
    Username,
    Password,
    Folder,
}

#[derive(Clone, Debug)]
pub enum Message {
    MailClient(String),
    IntervalText(String),
    Autostart(bool),
    AccountField {
        idx: usize,
        field: AccountField,
        value: String,
    },
    AddAccount,
    RemoveAccount(usize),
    TogglePasswordVisibility(usize),
    Save,
    SaveResult(Result<(), String>),
}

#[derive(Clone, Debug, Default)]
struct AccountDraft {
    name: String,
    server: String,
    port_text: String,
    username: String,
    password: String,
    folder: String,
}

impl AccountDraft {
    fn from_account(a: Account) -> Self {
        Self {
            name: a.name,
            server: a.server,
            port_text: a.port.to_string(),
            username: a.username,
            password: a.password,
            folder: a.folder,
        }
    }

    fn to_account(&self) -> Account {
        Account {
            name: self.name.clone(),
            server: self.server.clone(),
            port: self.port_text.parse().unwrap_or(993),
            username: self.username.clone(),
            password: self.password.clone(),
            folder: if self.folder.is_empty() {
                "INBOX".into()
            } else {
                self.folder.clone()
            },
        }
    }

    fn empty() -> Self {
        Self {
            port_text: "993".into(),
            folder: "INBOX".into(),
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default)]
enum SaveStatus {
    #[default]
    Idle,
    Saving,
    Saved,
    Error(String),
}

pub struct App {
    core: Core,
    mail_client: String,
    interval_text: String,
    autostart_enabled: bool,
    accounts: Vec<AccountDraft>,
    show_passwords: Vec<bool>,
    status: SaveStatus,
}

impl App {
    fn build_config(&self) -> Config {
        Config {
            mail_client: self.mail_client.clone(),
            interval_secs: self.interval_text.parse().unwrap_or(60).max(10),
            accounts: self.accounts.iter().map(AccountDraft::to_account).collect(),
        }
    }
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _: ()) -> (Self, Task<Message>) {
        let path = config_path();
        let cfg = config::read(&path).unwrap_or_default();
        let accounts: Vec<AccountDraft> = cfg
            .accounts
            .into_iter()
            .map(AccountDraft::from_account)
            .collect();
        let show_passwords = vec![false; accounts.len()];
        let mut app = App {
            core,
            mail_client: cfg.mail_client,
            interval_text: cfg.interval_secs.to_string(),
            autostart_enabled: autostart::is_enabled(),
            accounts,
            show_passwords,
            status: SaveStatus::Idle,
        };
        let title = app.set_window_title("tb-tray Settings".into());
        (app, title)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MailClient(s) => self.mail_client = s,
            Message::IntervalText(s) => {
                if s.chars().all(|c| c.is_ascii_digit()) && s.len() <= 6 {
                    self.interval_text = s;
                }
            }
            Message::Autostart(on) => {
                let res = if on {
                    autostart::enable()
                } else {
                    autostart::disable()
                };
                if let Err(e) = res {
                    self.status = SaveStatus::Error(format!("autostart: {e}"));
                }
                self.autostart_enabled = autostart::is_enabled();
            }
            Message::AccountField { idx, field, value } => {
                if let Some(a) = self.accounts.get_mut(idx) {
                    match field {
                        AccountField::Name => a.name = value,
                        AccountField::Server => a.server = value,
                        AccountField::Port => {
                            if value.chars().all(|c| c.is_ascii_digit()) && value.len() <= 5 {
                                a.port_text = value;
                            }
                        }
                        AccountField::Username => a.username = value,
                        AccountField::Password => a.password = value,
                        AccountField::Folder => a.folder = value,
                    }
                }
            }
            Message::AddAccount => {
                self.accounts.push(AccountDraft::empty());
                self.show_passwords.push(false);
            }
            Message::RemoveAccount(i) => {
                if i < self.accounts.len() {
                    self.accounts.remove(i);
                    self.show_passwords.remove(i);
                }
            }
            Message::TogglePasswordVisibility(i) => {
                if let Some(b) = self.show_passwords.get_mut(i) {
                    *b = !*b;
                }
            }
            Message::Save => {
                self.status = SaveStatus::Saving;
                let cfg = self.build_config();
                let path = config_path();
                return Task::perform(
                    async move { config::write(&path, &cfg).map_err(|e| e.to_string()) },
                    |r| Action::App(Message::SaveResult(r)),
                );
            }
            Message::SaveResult(Ok(())) => self.status = SaveStatus::Saved,
            Message::SaveResult(Err(e)) => self.status = SaveStatus::Error(e),
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let general = widget::settings::section()
            .title("General")
            .add(widget::settings::item(
                "Mail client",
                widget::text_input("/usr/bin/thunderbird", &self.mail_client)
                    .on_input(Message::MailClient)
                    .width(Length::Fixed(300.0)),
            ))
            .add(widget::settings::item(
                "Poll interval (seconds)",
                widget::text_input("60", &self.interval_text)
                    .on_input(Message::IntervalText)
                    .width(Length::Fixed(80.0)),
            ))
            .add(widget::settings::item(
                "Start tb-tray on login",
                widget::toggler(self.autostart_enabled).on_toggle(Message::Autostart),
            ));

        let mut sections: Vec<Element<Message>> = vec![general.into()];

        for (idx, acc) in self.accounts.iter().enumerate() {
            sections.push(account_section(
                idx,
                acc,
                self.show_passwords.get(idx).copied().unwrap_or(false),
            ));
        }

        let add_row = widget::row::with_children(vec![
            widget::button::standard("Add account")
                .on_press(Message::AddAccount)
                .into(),
            space::horizontal().into(),
        ]);
        sections.push(add_row.into());

        let body = widget::settings::view_column(sections);
        let scroll = widget::scrollable(body).height(Length::Fill);

        let status_widget: Element<Message> = match &self.status {
            SaveStatus::Idle => widget::Space::new().into(),
            SaveStatus::Saving => widget::text("Saving…").into(),
            SaveStatus::Saved => widget::text("Saved.").into(),
            SaveStatus::Error(e) => widget::text(format!("Error: {e}")).into(),
        };

        let footer = widget::row::with_children(vec![
            status_widget,
            space::horizontal().into(),
            widget::button::suggested("Save")
                .on_press(Message::Save)
                .into(),
        ])
        .spacing(8)
        .align_y(Alignment::Center);

        let content = widget::column::with_children(vec![scroll.into(), footer.into()])
            .spacing(16)
            .padding(16);

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

fn account_section<'a>(
    idx: usize,
    acc: &'a AccountDraft,
    show_password: bool,
) -> Element<'a, Message> {
    let title = if acc.name.is_empty() {
        format!("Account #{}", idx + 1)
    } else {
        acc.name.clone()
    };

    let header = widget::row::with_children(vec![
        widget::text::heading(title).into(),
        space::horizontal().into(),
        widget::button::destructive("Remove")
            .on_press(Message::RemoveAccount(idx))
            .into(),
    ])
    .spacing(8)
    .align_y(Alignment::Center);

    widget::settings::section()
        .header(header)
        .add(widget::settings::item(
            "Display name",
            widget::text_input("Personal", &acc.name)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Name,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            "IMAP server",
            widget::text_input("imap.example.com", &acc.server)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Server,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            "Port",
            widget::text_input("993", &acc.port_text)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Port,
                    value: s,
                })
                .width(Length::Fixed(80.0)),
        ))
        .add(widget::settings::item(
            "Username",
            widget::text_input("you@example.com", &acc.username)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Username,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            "Password",
            widget::secure_input(
                "••••••",
                acc.password.clone(),
                Some(Message::TogglePasswordVisibility(idx)),
                !show_password,
            )
            .on_input(move |s| Message::AccountField {
                idx,
                field: AccountField::Password,
                value: s,
            })
            .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            "Folder",
            widget::text_input("INBOX", &acc.folder)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Folder,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .into()
}
