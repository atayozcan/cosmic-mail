// cosmic-mail: Wayland-native COSMIC panel applet for JMAP unread-mail
// notifications. Replaces the v0.6/v0.7 SNI tray daemon.
//
// The applet IS the long-running process — cosmic-panel spawns the
// binary when the user adds it to a panel and keeps it alive. There's
// no separate daemon, no autostart entry. The panel button shows the
// unread count (icon swaps between the read/unread variants); click
// to open a popover listing per-account unread totals plus a
// "Settings…" button.
//
// JMAP push (Server-Sent Events) drives the per-account watcher
// inside an iced Subscription channel; on disconnect we reconnect
// with exponential backoff and fall back to interval-only polling
// if the server doesn't expose event_source.

use cosmic::app::{Core, Task};
use cosmic::iced::futures::channel::mpsc;
use cosmic::iced::platform_specific::shell::wayland::commands::popup::{destroy_popup, get_popup};
use cosmic::iced::{event, keyboard, stream, window, Event, Length, Subscription};
use cosmic::prelude::*;
use cosmic::widget;
use std::collections::HashMap;
use std::process::Command;
use std::time::Duration;

use cosmic_mail::accounts::{self, Account};
use cosmic_mail::secrets;
use cosmic_mail::settings;
use cosmic_mail::{accounts_path, fl, localize, APP_ID};

const ICON_READ: &str = "cosmic-mail-symbolic";
const ICON_UNREAD: &str = "cosmic-mail-unread-symbolic";

fn main() -> cosmic::iced::Result {
    localize::localize();
    cosmic::applet::run::<App>(())
}

#[derive(Clone, Debug)]
pub enum Message {
    TogglePopup,
    PopupClosed(window::Id),
    OpenSettings,
    /// Per-account unread tally landed from the watcher.
    UnreadUpdated { account: String, count: usize },
    /// Per-account watcher hit a fatal error — surface via a toast/log.
    AccountError { account: String, error: String },
    Noop,
}

pub struct App {
    core: Core,
    popup: Option<window::Id>,
    /// Per-account unread tally, keyed by display name. Total
    /// across accounts drives the icon variant.
    unread: HashMap<String, usize>,
    /// Most recent error per account, to render in the popover.
    errors: HashMap<String, String>,
    /// Account list snapshot loaded at init.
    accounts: Vec<Account>,
}

impl App {
    fn total_unread(&self) -> usize {
        self.unread.values().sum()
    }

    fn close_popup_task(&mut self) -> Task<Message> {
        if let Some(p) = self.popup.take() {
            destroy_popup(p)
        } else {
            Task::none()
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
        let accounts = accounts::read(&accounts_path())
            .map(|f| f.accounts)
            .unwrap_or_default();
        (
            App {
                core,
                popup: None,
                unread: HashMap::new(),
                errors: HashMap::new(),
                accounts,
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::TogglePopup => {
                if let Some(p) = self.popup.take() {
                    return destroy_popup(p);
                }
                let new_id = window::Id::unique();
                self.popup = Some(new_id);
                let popup_settings = self.core.applet.get_popup_settings(
                    self.core.main_window_id().expect("applet has main window"),
                    new_id,
                    None,
                    None,
                    None,
                );
                get_popup(popup_settings)
            }
            Message::PopupClosed(id) => {
                if Some(id) == self.popup {
                    self.popup = None;
                }
                Task::none()
            }
            Message::OpenSettings => {
                if let Ok(exe) = std::env::current_exe() {
                    let settings_bin = exe
                        .parent()
                        .map(|p| p.join("cosmic-mail-settings"))
                        .unwrap_or_else(|| std::path::PathBuf::from("cosmic-mail-settings"));
                    let _ = Command::new(settings_bin).spawn();
                }
                self.close_popup_task()
            }
            Message::UnreadUpdated { account, count } => {
                self.errors.remove(&account);
                self.unread.insert(account, count);
                Task::none()
            }
            Message::AccountError { account, error } => {
                self.errors.insert(account, error);
                Task::none()
            }
            Message::Noop => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let icon = if self.total_unread() > 0 {
            ICON_UNREAD
        } else {
            ICON_READ
        };
        self.core
            .applet
            .icon_button(icon)
            .on_press(Message::TogglePopup)
            .into()
    }

    fn view_window(&self, _id: window::Id) -> Element<'_, Message> {
        let header = if self.accounts.is_empty() {
            widget::container(widget::text(fl!("popup-no-accounts")))
                .padding(16)
                .width(Length::Fill)
        } else {
            widget::container(widget::text::heading(if self.total_unread() == 0 {
                fl!("popup-no-unread")
            } else {
                fl!("popup-total-unread", count = self.total_unread())
            }))
            .padding(8)
            .width(Length::Fill)
        };

        let mut sections: Vec<Element<Message>> = vec![header.into()];

        if !self.accounts.is_empty() {
            let mut col = widget::column::with_capacity(self.accounts.len()).spacing(2);
            for acc in &self.accounts {
                let count = self.unread.get(&acc.name).copied().unwrap_or(0);
                let err = self.errors.get(&acc.name).cloned();
                let label = if let Some(e) = err {
                    fl!(
                        "popup-account-error",
                        account = acc.name.clone(),
                        error = e
                    )
                } else if count == 0 {
                    fl!(
                        "popup-account-empty",
                        account = acc.name.clone()
                    )
                } else {
                    fl!(
                        "popup-account-unread",
                        account = acc.name.clone(),
                        count = count
                    )
                };
                col = col.push(widget::text(label));
            }
            sections.push(col.into());
        }

        let footer = widget::row::with_children(vec![
            widget::button::standard(fl!("settings"))
                .on_press(Message::OpenSettings)
                .into(),
            widget::space::horizontal().into(),
        ])
        .spacing(8)
        .align_y(cosmic::iced::Alignment::Center);

        let content = widget::column::with_children(vec![
            widget::column::with_children(sections).spacing(6).into(),
            widget::space::vertical().height(Length::Fixed(6.0)).into(),
            footer.into(),
        ])
        .spacing(8)
        .padding(8);

        self.core.applet.popup_container(content).into()
    }

    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::PopupClosed(id))
    }

    fn subscription(&self) -> Subscription<Message> {
        // One subscription per account, plus an Esc-to-close hook.
        // Each per-account stream connects, runs the JMAP push +
        // heartbeat loop, and emits UnreadUpdated / AccountError as
        // it learns things. Reconnect with exponential backoff lives
        // inside the closure.
        let mut subs: Vec<Subscription<Message>> = Vec::with_capacity(self.accounts.len() + 1);
        for acc in &self.accounts {
            subs.push(account_subscription(acc.clone()));
        }
        subs.push(escape_subscription());
        Subscription::batch(subs)
    }
}

/// Per-account JMAP watcher subscription. Connects, does an initial
/// fetch, subscribes to event_source for Email + EmailDelivery, and
/// emits UnreadUpdated on each change. Reconnects with backoff on
/// any fault.
///
/// `Subscription::run_with` takes a fn-pointer for the builder, so
/// `account_stream` is a top-level fn that takes the account by
/// reference and clones it into the stream's future.
fn account_subscription(acc: Account) -> Subscription<Message> {
    Subscription::run_with(acc, account_stream)
}

fn account_stream(
    acc: &Account,
) -> std::pin::Pin<Box<dyn cosmic::iced::futures::Stream<Item = Message> + Send + 'static>> {
    let acc = acc.clone();
    Box::pin(stream::channel(
        8,
        move |output: mpsc::Sender<Message>| async move {
            run_account(acc, output).await
        },
    ))
}

async fn run_account(acc: Account, mut output: mpsc::Sender<Message>) {
    use cosmic::iced::futures::SinkExt;

    let mut backoff = Duration::from_secs(5);
    let max_backoff = Duration::from_secs(300);
    loop {
        match account_session(&acc, &mut output).await {
            Ok(()) => {
                backoff = Duration::from_secs(5);
            }
            Err(e) => {
                let _ = output
                    .send(Message::AccountError {
                        account: acc.name.clone(),
                        error: e,
                    })
                    .await;
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        }
    }
}

async fn fetch_unread(client: &jmap_client::client::Client) -> Result<usize, String> {
    use jmap_client::core::query::Filter;
    use jmap_client::email::query::Filter as EmailFilter;
    use jmap_client::mailbox::{query::Filter as MailboxFilter, Role};

    let inbox_id = client
        .mailbox_query(MailboxFilter::role(Role::Inbox).into(), None::<Vec<_>>)
        .await
        .map_err(|e| format!("mailbox_query: {e}"))?
        .take_ids()
        .into_iter()
        .next()
        .ok_or_else(|| "no Inbox mailbox on this account".to_string())?;
    let ids = client
        .email_query(
            Filter::and([
                EmailFilter::in_mailbox(&inbox_id),
                EmailFilter::not_keyword("$seen"),
            ])
            .into(),
            None::<Vec<_>>,
        )
        .await
        .map_err(|e| format!("email_query: {e}"))?
        .take_ids();
    Ok(ids.len())
}

async fn account_session(
    acc: &Account,
    output: &mut mpsc::Sender<Message>,
) -> Result<(), String> {
    use cosmic::iced::futures::SinkExt;
    use futures_util::StreamExt;
    use jmap_client::client::Client;
    use jmap_client::DataType;

    let password = secrets::fetch(&acc.username, &acc.server).await?;
    let base = acc.base_url();
    // jmap-client's custom redirect policy aborts redirects whose target
    // host isn't on the trusted list, and the list defaults to empty.
    // Many JMAP providers (Fastmail among them) redirect from
    // /.well-known/jmap to the actual session URL, so without this
    // every connect would fail with a transport error. Trust the host
    // the user typed in plus its apex domain — covers
    // `api.fastmail.com` ↔ `fastmail.com`-style redirects.
    let trusted = trusted_hosts_for(&base);
    let client = Client::new()
        .credentials((acc.username.as_str(), password.as_str()))
        .follow_redirects(trusted)
        .connect(&base)
        .await
        .map_err(|e| format!("connect: {e}"))?;

    // Initial fetch.
    let n = fetch_unread(&client).await?;
    let _ = output
        .send(Message::UnreadUpdated {
            account: acc.name.clone(),
            count: n,
        })
        .await;

    match client
        .event_source(
            Some([DataType::Email, DataType::EmailDelivery]),
            false,
            Some(60),
            None,
        )
        .await
    {
        Ok(mut stream) => loop {
            let interval = Duration::from_secs(settings::load().interval_secs.max(10));
            tokio::select! {
                note = stream.next() => match note {
                    Some(Ok(_)) => {
                        let n = fetch_unread(&client).await?;
                        let _ = output.send(Message::UnreadUpdated {
                            account: acc.name.clone(),
                            count: n,
                        }).await;
                    }
                    Some(Err(e)) => return Err(format!("event_source: {e}")),
                    None => return Ok(()),
                },
                _ = tokio::time::sleep(interval) => {
                    let n = fetch_unread(&client).await?;
                    let _ = output.send(Message::UnreadUpdated {
                        account: acc.name.clone(),
                        count: n,
                    }).await;
                }
            }
        },
        Err(_) => loop {
            let interval = Duration::from_secs(settings::load().interval_secs.max(10));
            tokio::time::sleep(interval).await;
            let n = fetch_unread(&client).await?;
            let _ = output
                .send(Message::UnreadUpdated {
                    account: acc.name.clone(),
                    count: n,
                })
                .await;
        },
    }
}

/// Build the redirect-trust list jmap-client expects.
///
/// Includes the URL's literal host plus a one-level apex (so
/// `api.fastmail.com` also trusts `fastmail.com` and vice versa).
/// Anything more elaborate would need a public-suffix list, which
/// isn't worth the dep weight for the redirect lane.
fn trusted_hosts_for(url: &str) -> Vec<String> {
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host_end = stripped.find(['/', ':']).unwrap_or(stripped.len());
    let host = &stripped[..host_end];
    if host.is_empty() {
        return Vec::new();
    }
    let mut out = vec![host.to_string()];
    if let Some((_, parent)) = host.split_once('.') {
        if parent.contains('.') && parent != host {
            out.push(parent.to_string());
        }
    }
    out
}

fn escape_subscription() -> Subscription<Message> {
    event::listen_with(|evt, _status, _id| match evt {
        Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
            if matches!(key, keyboard::Key::Named(keyboard::key::Named::Escape)) {
                Some(Message::Noop)
            } else {
                None
            }
        }
        _ => None,
    })
}
