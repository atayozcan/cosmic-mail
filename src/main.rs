// tb-tray: minimal Cosmic-native (StatusNotifierItem) IMAP mail notifier.
//
// - Polls each configured IMAP folder every N seconds (default 60).
// - Fires a desktop notification when the unread count grows.
// - Shows a tray icon with the total unread count across accounts.
// - Click tray (or "Open Thunderbird" menu) → spawns /usr/bin/thunderbird.
// - Does NOT keep Thunderbird running. TB launches only when you ask for it.
//
// Config: ~/.config/tb-tray/config.toml — see sample at end of this file's
// directory after first run.

use ksni::{menu::*, Tray, TrayMethods};
use notify_rust::{Hint, Notification, Urgency};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[derive(Deserialize, Clone, Debug)]
struct Account {
    name: String,
    server: String,
    #[serde(default = "default_port")]
    port: u16,
    username: String,
    password: String,
    #[serde(default = "default_folder")]
    folder: String,
}

fn default_port() -> u16 {
    993
}
fn default_folder() -> String {
    "INBOX".into()
}
fn default_mail_client() -> String {
    "/usr/bin/thunderbird".into()
}
fn default_interval() -> u64 {
    60
}

#[derive(Deserialize)]
struct Config {
    #[serde(default = "default_mail_client")]
    mail_client: String,
    #[serde(default = "default_interval")]
    interval_secs: u64,
    #[serde(rename = "account")]
    accounts: Vec<Account>,
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("no XDG_CONFIG_HOME")
        .join("tb-tray/config.toml")
}

fn write_sample_config(path: &PathBuf) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let sample = r#"# tb-tray config — fill in then restart the service.
# `mail_client` runs when you click the tray icon.

mail_client    = "/usr/bin/thunderbird"
interval_secs  = 60

[[account]]
name     = "Personal"
server   = "imap.example.com"
port     = 993
username = "you@example.com"
password = "REPLACE_ME"
folder   = "INBOX"

# add more [[account]] blocks for more mailboxes
"#;
    std::fs::write(path, sample)?;
    Ok(())
}

struct State {
    unread: AtomicUsize,
    mail_client: String,
}

#[derive(Clone)]
struct TbTray {
    state: Arc<State>,
}

impl Tray for TbTray {
    fn id(&self) -> String {
        "tb-tray".into()
    }
    fn title(&self) -> String {
        "Mail".into()
    }
    fn icon_name(&self) -> String {
        if self.state.unread.load(Ordering::Relaxed) > 0 {
            "mail-unread-symbolic".into()
        } else {
            "mail-read-symbolic".into()
        }
    }
    fn tool_tip(&self) -> ksni::ToolTip {
        let n = self.state.unread.load(Ordering::Relaxed);
        let title = if n == 0 {
            "Mail: no unread".into()
        } else {
            format!("Mail: {n} unread")
        };
        ksni::ToolTip {
            title,
            description: "Click to open Thunderbird".into(),
            icon_name: "mail-unread-symbolic".into(),
            icon_pixmap: vec![],
        }
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = Command::new(&self.state.mail_client).spawn();
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        let client = self.state.mail_client.clone();
        let cfg = config_path();
        let cfg_for_edit = cfg.clone();
        let cfg_dir = cfg.parent().map(|p| p.to_path_buf()).unwrap_or(cfg);
        vec![
            StandardItem {
                label: "Open Thunderbird".into(),
                icon_name: "mail-message-new-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = Command::new(&client).spawn();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Edit Config".into(),
                icon_name: "document-edit-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = Command::new("xdg-open").arg(&cfg_for_edit).spawn();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Open Config Folder".into(),
                icon_name: "folder-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = Command::new("xdg-open").arg(&cfg_dir).spawn();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit Tray".into(),
                icon_name: "application-exit-symbolic".into(),
                activate: Box::new(|_| std::process::exit(0)),
                ..Default::default()
            }
            .into(),
        ]
    }
}

/// Block on a sync IMAP fetch of UNSEEN-uid count for one account.
fn fetch_unread_blocking(acc: &Account) -> Result<usize, String> {
    let tls = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| format!("tls build: {e}"))?;
    let client = imap::connect((acc.server.as_str(), acc.port), &acc.server, &tls)
        .map_err(|e| format!("connect: {e}"))?;
    let mut session = client
        .login(&acc.username, &acc.password)
        .map_err(|e| format!("login: {}", e.0))?;
    session
        .examine(&acc.folder)
        .map_err(|e| format!("examine: {e}"))?;
    let uids = session
        .uid_search("UNSEEN")
        .map_err(|e| format!("search: {e}"))?;
    let _ = session.logout();
    Ok(uids.len())
}

fn notify_new_mail(account: &str, delta: usize, total: usize, mail_client: &str) {
    let body = if delta == 1 {
        format!("1 new message in {account}\n({total} unread total)")
    } else {
        format!("{delta} new messages in {account}\n({total} unread total)")
    };
    let result = Notification::new()
        .summary("New mail")
        .body(&body)
        .icon("mail-unread-symbolic")
        .appname("tb-tray")
        .urgency(Urgency::Normal)
        .hint(Hint::Category("email.arrived".into()))
        // "default" fires when the user clicks the notification body itself.
        // Adding an explicit "open" action gives a button on daemons that
        // render them (e.g. KDE); on Cosmic the body-click is enough.
        .action("default", "Open")
        .action("open", "Open")
        .show();

    // Block this (blocking-pool) thread until the user clicks or the
    // notification is closed. Spawn the mail client only on a real click.
    let mc = mail_client.to_string();
    if let Ok(handle) = result {
        handle.wait_for_action(|action| {
            if matches!(action, "default" | "open") {
                let _ = std::process::Command::new(&mc).spawn();
            }
        });
    }
}

fn run_configure() -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, BufRead, Write};

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut prompt = |msg: &str, default: &str| -> io::Result<String> {
        if default.is_empty() {
            print!("{msg}: ");
        } else {
            print!("{msg} [{default}]: ");
        }
        stdout.flush()?;
        let mut s = String::new();
        stdin.lock().read_line(&mut s)?;
        let s = s.trim().to_string();
        Ok(if s.is_empty() { default.into() } else { s })
    };

    println!("tb-tray configure: enter IMAP credentials. Press enter to accept defaults.\n");
    let name = prompt("Account name", "Personal")?;
    let server = prompt("IMAP server", "")?;
    let port: u16 = prompt("IMAP port", "993")?.parse().unwrap_or(993);
    let username = prompt("Username (full email)", "")?;
    let password = rpassword::prompt_password("Password (hidden): ")?;
    let folder = prompt("Folder", "INBOX")?;
    let mail_client = prompt("Mail client to launch on click", "/usr/bin/thunderbird")?;
    let interval: u64 = prompt("Poll interval seconds", "60")?.parse().unwrap_or(60);

    let toml = format!(
        r#"# Generated by `tb-tray --configure`. Stored in plaintext — chmod 600 advised.

mail_client    = "{mail_client}"
interval_secs  = {interval}

[[account]]
name     = "{name}"
server   = "{server}"
port     = {port}
username = "{username}"
password = "{}"
folder   = "{folder}"
"#,
        password.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, toml)?;

    // Restrict to user-only since password is plaintext
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(&path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(&path, perms)?;

    println!("\ntb-tray: wrote {} (mode 0600)", path.display());
    println!("Start the tray with: tb-tray");
    Ok(())
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // CLI handling — no clap to keep it small.
    let mut args = std::env::args().skip(1);
    if let Some(cmd) = args.next() {
        match cmd.as_str() {
            "--configure" | "-c" => return run_configure(),
            "--help" | "-h" => {
                println!("tb-tray: minimal IMAP mail notifier with Cosmic-native tray.");
                println!();
                println!("Usage:");
                println!("  tb-tray              run the tray service");
                println!("  tb-tray --configure  interactively write ~/.config/tb-tray/config.toml");
                println!("  tb-tray --help       this help");
                return Ok(());
            }
            other => {
                eprintln!("tb-tray: unknown argument {other:?}");
                std::process::exit(2);
            }
        }
    }

    // Load config or generate a sample on first run.
    let cfg_path = config_path();
    if !cfg_path.exists() {
        write_sample_config(&cfg_path)?;
        eprintln!("tb-tray: wrote sample config to {}", cfg_path.display());
        eprintln!("tb-tray: run `tb-tray --configure` for an interactive setup,");
        eprintln!("         or edit the file manually, then restart.");
        return Ok(());
    }
    let cfg_str = std::fs::read_to_string(&cfg_path)?;
    let cfg: Config = toml::from_str(&cfg_str)?;
    if cfg.accounts.is_empty() {
        eprintln!("tb-tray: no [[account]] blocks in config — nothing to poll.");
        return Ok(());
    }

    let state = Arc::new(State {
        unread: AtomicUsize::new(0),
        mail_client: cfg.mail_client.clone(),
    });

    // Register the SNI tray.
    let handle = TbTray {
        state: state.clone(),
    }
    .spawn()
    .await?;

    // One poll loop per account, sharing `last_unread_per_account` so we can
    // recompute the global unread total on each tick.
    let last = Arc::new(Mutex::new(HashMap::<String, usize>::new()));
    let interval = Duration::from_secs(cfg.interval_secs.max(10));
    let accounts = cfg.accounts;

    for acc in accounts {
        let last = last.clone();
        let state = state.clone();
        let handle = handle.clone();
        let acc_name = acc.name.clone();
        tokio::spawn(async move {
            loop {
                let acc2 = acc.clone();
                let result =
                    tokio::task::spawn_blocking(move || fetch_unread_blocking(&acc2)).await;

                match result {
                    Ok(Ok(n)) => {
                        let mut map = last.lock().await;
                        let prev = map.get(&acc_name).copied();
                        map.insert(acc_name.clone(), n);
                        let total: usize = map.values().sum();
                        state.unread.store(total, Ordering::Relaxed);
                        drop(map);

                        // Notify only if unread count grew (not on first poll).
                        // notify-rust's .show() calls block_on internally for the
                        // D-Bus call, which panics inside a tokio runtime — so we
                        // run it on the blocking pool.
                        if let Some(p) = prev {
                            if n > p {
                                let acc_n = acc_name.clone();
                                let delta = n - p;
                                let mc = state.mail_client.clone();
                                // detached: wait_for_action blocks for a long
                                // time, we don't want to hold the poll loop.
                                tokio::task::spawn_blocking(move || {
                                    notify_new_mail(&acc_n, delta, total, &mc);
                                });
                            }
                        }

                        // Refresh tray icon/tooltip.
                        let _ = handle
                            .update(|_: &mut TbTray| {
                                // No mutation needed; update() triggers re-fetch.
                            })
                            .await;
                    }
                    Ok(Err(e)) => eprintln!("tb-tray[{acc_name}]: {e}"),
                    Err(e) => eprintln!("tb-tray[{acc_name}]: join: {e}"),
                }
                sleep(interval).await;
            }
        });
    }

    // Park forever; tasks drive the work.
    std::future::pending::<()>().await;
    Ok(())
}
