// Standalone settings GUI for cosmic-mail. Launched as a child
// process by the applet's "Settings…" button.

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::{Alignment, Length, Size};
use cosmic::prelude::*;
use cosmic::widget::{self, space};

use cosmic_mail::accounts::{self, Account, AccountsFile};
use cosmic_mail::secrets;
use cosmic_mail::settings::{self, Settings as MailSettings};
use cosmic_mail::{accounts_path, fl, localize, APP_ID};

#[derive(Clone, Debug, Default)]
enum SaveStatus {
    #[default]
    Idle,
    Saving,
    Saved,
    Error(String),
}

fn main() -> cosmic::iced::Result {
    localize::localize();
    let settings = Settings::default()
        .size(Size::new(760.0, 680.0))
        .exit_on_close(true);
    cosmic::app::run::<App>(settings, ())
}

#[derive(Clone, Copy, Debug)]
pub enum AccountField {
    Name,
    Server,
    Username,
    Password,
}

#[derive(Clone, Debug)]
pub enum Message {
    MailClient(String),
    IntervalText(String),
    AccountField {
        idx: usize,
        field: AccountField,
        value: String,
    },
    AddAccount,
    RemoveAccount(usize),
    TogglePasswordVisibility(usize),
    Save,
    PasswordLoaded {
        idx: usize,
        password: Option<String>,
    },
    SaveResult(Result<(), String>),
    Noop,
}

#[derive(Clone, Debug, Default)]
struct AccountDraft {
    name: String,
    server: String,
    username: String,
    password: String,
}

impl AccountDraft {
    fn from_account(a: Account) -> Self {
        Self {
            name: a.name,
            server: a.server,
            username: a.username,
            password: String::new(),
        }
    }

    fn to_account(&self) -> Account {
        Account {
            name: self.name.clone(),
            server: self.server.clone(),
            username: self.username.clone(),
        }
    }

    fn empty() -> Self {
        Self::default()
    }
}

pub struct App {
    core: Core,
    mail_client: String,
    interval_text: String,
    accounts: Vec<AccountDraft>,
    show_passwords: Vec<bool>,
    status: SaveStatus,
}

impl App {
    fn build_settings(&self) -> MailSettings {
        MailSettings {
            mail_client: self.mail_client.clone(),
            interval_secs: self.interval_text.parse().unwrap_or(60).max(10),
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
        let s = settings::load();
        let accounts_file = accounts::read(&accounts_path()).unwrap_or_default();
        let accounts: Vec<AccountDraft> = accounts_file
            .accounts
            .into_iter()
            .map(AccountDraft::from_account)
            .collect();
        let show_passwords = vec![false; accounts.len()];
        let app = App {
            core,
            mail_client: s.mail_client,
            interval_text: s.interval_secs.to_string(),
            accounts,
            show_passwords,
            status: SaveStatus::Idle,
        };

        let fetches: Vec<Task<Message>> = app
            .accounts
            .iter()
            .enumerate()
            .map(|(idx, a)| {
                let username = a.username.clone();
                let server = a.server.clone();
                Task::perform(
                    async move { secrets::fetch(&username, &server).await.ok() },
                    move |password| {
                        cosmic::Action::App(Message::PasswordLoaded { idx, password })
                    },
                )
            })
            .collect();

        (app, Task::batch(fetches))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MailClient(s) => self.mail_client = s,
            Message::IntervalText(s) => {
                if s.chars().all(|c| c.is_ascii_digit()) && s.len() <= 6 {
                    self.interval_text = s;
                }
            }
            Message::AccountField { idx, field, value } => {
                if let Some(a) = self.accounts.get_mut(idx) {
                    match field {
                        AccountField::Name => a.name = value,
                        AccountField::Server => a.server = value,
                        AccountField::Username => a.username = value,
                        AccountField::Password => a.password = value,
                    }
                }
            }
            Message::AddAccount => {
                self.accounts.push(AccountDraft::empty());
                self.show_passwords.push(false);
            }
            Message::RemoveAccount(i) => {
                if i >= self.accounts.len() {
                    return Task::none();
                }
                let removed = self.accounts.remove(i);
                self.show_passwords.remove(i);
                let username = removed.username;
                let server = removed.server;
                return Task::perform(
                    async move {
                        let _ = secrets::delete(&username, &server).await;
                    },
                    |()| cosmic::Action::App(Message::Noop),
                );
            }
            Message::TogglePasswordVisibility(i) => {
                if let Some(b) = self.show_passwords.get_mut(i) {
                    *b = !*b;
                }
            }
            Message::Save => {
                let drafts = self.accounts.clone();
                let new_settings = self.build_settings();
                let path = accounts_path();
                self.status = SaveStatus::Saving;
                return Task::perform(
                    async move {
                        for d in &drafts {
                            if !d.password.is_empty() {
                                secrets::store(&d.username, &d.server, &d.password)
                                    .await
                                    .map_err(|e| format!("secrets-{e}"))?;
                            }
                        }
                        settings::save(&new_settings).map_err(|e| format!("settings-{e}"))?;
                        let af = AccountsFile {
                            accounts: drafts.iter().map(AccountDraft::to_account).collect(),
                        };
                        accounts::write(&path, &af).map_err(|e| format!("accounts-{e}"))?;
                        Ok::<(), String>(())
                    },
                    |result| cosmic::Action::App(Message::SaveResult(result)),
                );
            }
            Message::PasswordLoaded { idx, password } => {
                if let Some(draft) = self.accounts.get_mut(idx) {
                    if let Some(p) = password {
                        if draft.password.is_empty() {
                            draft.password = p;
                        }
                    }
                }
            }
            Message::SaveResult(result) => {
                self.status = match result {
                    Ok(()) => SaveStatus::Saved,
                    Err(e) => SaveStatus::Error(e),
                };
            }
            Message::Noop => {}
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let general = widget::settings::section()
            .title(fl!("settings-section-general"))
            .add(widget::settings::item(
                fl!("settings-mail-client"),
                widget::text_input(fl!("settings-mail-client-placeholder"), &self.mail_client)
                    .on_input(Message::MailClient)
                    .width(Length::Fixed(300.0)),
            ))
            .add(widget::settings::item(
                fl!("settings-interval"),
                widget::text_input("60", &self.interval_text)
                    .on_input(Message::IntervalText)
                    .width(Length::Fixed(80.0)),
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
            widget::button::standard(fl!("account-add"))
                .on_press(Message::AddAccount)
                .into(),
            space::horizontal().into(),
        ]);
        sections.push(add_row.into());

        let body = widget::settings::view_column(sections);
        let scroll = widget::scrollable(body).height(Length::Fill);

        let status_widget: Element<Message> = match &self.status {
            SaveStatus::Idle => widget::Space::new().into(),
            SaveStatus::Saving => widget::text(fl!("settings-saving")).into(),
            SaveStatus::Saved => widget::text(fl!("settings-saved")).into(),
            SaveStatus::Error(e) => {
                let msg = if let Some(rest) = e.strip_prefix("settings-") {
                    fl!("settings-error-settings", error = rest.to_string())
                } else if let Some(rest) = e.strip_prefix("accounts-") {
                    fl!("settings-error-accounts", error = rest.to_string())
                } else if let Some(rest) = e.strip_prefix("secrets-") {
                    fl!("settings-error-secrets", error = rest.to_string())
                } else {
                    fl!("settings-error", error = e.clone())
                };
                widget::text(msg).into()
            }
        };

        let footer = widget::row::with_children(vec![
            status_widget,
            space::horizontal().into(),
            widget::button::suggested(fl!("settings-save"))
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
        fl!("account-fallback-title", index = ((idx + 1) as u32))
    } else {
        acc.name.clone()
    };

    let header = widget::row::with_children(vec![
        widget::text::heading(title).into(),
        space::horizontal().into(),
        widget::button::destructive(fl!("account-remove"))
            .on_press(Message::RemoveAccount(idx))
            .into(),
    ])
    .spacing(8)
    .align_y(Alignment::Center);

    widget::settings::section()
        .header(header)
        .add(widget::settings::item(
            fl!("account-display-name"),
            widget::text_input(fl!("account-display-name-placeholder"), &acc.name)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Name,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            fl!("account-server"),
            widget::text_input(fl!("account-server-placeholder"), &acc.server)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Server,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            fl!("account-username"),
            widget::text_input(fl!("account-username-placeholder"), &acc.username)
                .on_input(move |s| Message::AccountField {
                    idx,
                    field: AccountField::Username,
                    value: s,
                })
                .width(Length::Fixed(300.0)),
        ))
        .add(widget::settings::item(
            fl!("account-password"),
            widget::secure_input(
                fl!("account-password-placeholder"),
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
        .into()
}

