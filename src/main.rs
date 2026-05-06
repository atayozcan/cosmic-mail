// tb-tray: minimal Wayland-native (StatusNotifierItem) IMAP mail notifier.
//
// Single binary, single user-facing entry point: `tb-tray`.
//   - On first run with no config, the settings GUI opens for setup.
//   - Otherwise, the tray daemon runs.
//   - The tray menu's "Settings…" item re-execs the binary with the
//     internal `--settings` flag to open the GUI in a child process,
//     so the iced/wgpu window doesn't have to share the tokio+ksni
//     event loop.

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

fn print_help() {
    println!("tb-tray: Wayland-native IMAP mail notifier.");
    println!();
    println!("Usage: tb-tray");
    println!();
    println!("  On first run with no config, the settings window opens so");
    println!("  you can add an account. After that, running tb-tray starts");
    println!("  the tray daemon. Reach settings later from the tray icon's");
    println!("  menu.");
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
        None => {
            // First-run / smart default: if there's no config yet, open
            // the settings GUI so the user can add an account. After the
            // window closes, fall through to daemon mode if a config now
            // exists — so a fresh `tb-tray` invocation does the right
            // thing without any flags.
            if !config_path().exists() {
                settings_ui::run().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            }
            if config_path().exists() {
                run_daemon()
            } else {
                eprintln!("tb-tray: no config saved; exiting.");
                Ok(())
            }
        }
        // Internal: the tray menu re-execs itself with this flag so the
        // GUI runs in its own process. Not advertised in --help.
        Some("--settings") => settings_ui::run().map_err(|e| e.into()),
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
