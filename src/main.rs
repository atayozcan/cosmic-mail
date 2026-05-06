// tb-tray: minimal Wayland-native (StatusNotifierItem) IMAP mail notifier.
//
// Single binary, three modes:
//   tb-tray              run the tray daemon (default)
//   tb-tray --settings   open the libcosmic settings GUI
//   tb-tray --configure  CLI prompt for first-run setup
//
// The daemon's "Settings…" menu item re-launches the binary with --settings
// so the GUI never blocks the polling loop.

use ksni::{menu::*, Tray, TrayMethods};
use notify_rust::{Hint, Notification, Urgency};
use std::collections::HashMap;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tb_tray::config::{self, Account};
use tb_tray::paths::{config_path, self_exec};
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

mod settings_ui;

const ICON_READ: &str = "tb-tray-symbolic";
const ICON_UNREAD: &str = "tb-tray-unread-symbolic";

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
            ICON_UNREAD.into()
        } else {
            ICON_READ.into()
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
            description: "Click to open your mail client".into(),
            icon_name: if n > 0 { ICON_UNREAD.into() } else { ICON_READ.into() },
            icon_pixmap: vec![],
        }
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = Command::new(&self.state.mail_client).spawn();
    }
    fn menu(&self) -> Vec<MenuItem<Self>> {
        let client = self.state.mail_client.clone();
        let exe = self_exec();
        vec![
            StandardItem {
                label: "Open Mail".into(),
                icon_name: "mail-message-new-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = Command::new(&client).spawn();
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Settings…".into(),
                icon_name: "preferences-system-symbolic".into(),
                activate: Box::new(move |_| {
                    let _ = Command::new(&exe).arg("--settings").spawn();
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
        .icon(ICON_UNREAD)
        .appname("tb-tray")
        .urgency(Urgency::Normal)
        .hint(Hint::Category("email.arrived".into()))
        .action("default", "Open")
        .action("open", "Open")
        .show();

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

    let cfg = config::Config {
        mail_client,
        interval_secs: interval,
        accounts: vec![Account {
            name,
            server,
            port,
            username,
            password,
            folder,
        }],
    };
    let path = config_path();
    config::write(&path, &cfg)?;

    println!("\ntb-tray: wrote {} (mode 0600)", path.display());
    println!("Start the tray with: tb-tray");
    Ok(())
}

fn print_help() {
    println!("tb-tray: minimal IMAP mail notifier with a Wayland-native tray.");
    println!();
    println!("Usage:");
    println!("  tb-tray              run the tray service");
    println!("  tb-tray --settings   open the libcosmic settings GUI");
    println!("  tb-tray --configure  interactively write ~/.config/tb-tray/config.toml");
    println!("  tb-tray --help       this help");
}

fn run_daemon() -> Result<(), Box<dyn std::error::Error>> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(daemon_main())
}

async fn daemon_main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg_path = config_path();
    if !cfg_path.exists() {
        config::write_sample(&cfg_path)?;
        eprintln!("tb-tray: wrote sample config to {}", cfg_path.display());
        eprintln!("tb-tray: run `tb-tray --settings` for the GUI editor,");
        eprintln!("         or `tb-tray --configure` for the CLI prompt,");
        eprintln!("         or edit the file manually, then restart.");
        return Ok(());
    }
    let cfg = config::read(&cfg_path).map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
    if cfg.accounts.is_empty() {
        eprintln!("tb-tray: no [[account]] blocks in config — nothing to poll.");
        return Ok(());
    }

    let state = Arc::new(State {
        unread: AtomicUsize::new(0),
        mail_client: cfg.mail_client.clone(),
    });

    let handle = TbTray {
        state: state.clone(),
    }
    .spawn()
    .await?;

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

                        if let Some(p) = prev {
                            if n > p {
                                let acc_n = acc_name.clone();
                                let delta = n - p;
                                let mc = state.mail_client.clone();
                                tokio::task::spawn_blocking(move || {
                                    notify_new_mail(&acc_n, delta, total, &mc);
                                });
                            }
                        }

                        let _ = handle.update(|_: &mut TbTray| {}).await;
                    }
                    Ok(Err(e)) => eprintln!("tb-tray[{acc_name}]: {e}"),
                    Err(e) => eprintln!("tb-tray[{acc_name}]: join: {e}"),
                }
                sleep(interval).await;
            }
        });
    }

    std::future::pending::<()>().await;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => run_daemon(),
        Some("--settings") | Some("-s") => settings_ui::run().map_err(|e| e.into()),
        Some("--configure") | Some("-c") => run_configure(),
        Some("--help") | Some("-h") => {
            print_help();
            Ok(())
        }
        Some(other) => {
            eprintln!("tb-tray: unknown argument {other:?}");
            print_help();
            std::process::exit(2);
        }
    }
}
