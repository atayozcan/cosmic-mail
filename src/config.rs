use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Account {
    pub name: String,
    pub server: String,
    #[serde(default = "default_port")]
    pub port: u16,
    pub username: String,
    pub password: String,
    #[serde(default = "default_folder")]
    pub folder: String,
}

pub fn default_port() -> u16 {
    993
}
pub fn default_folder() -> String {
    "INBOX".into()
}
pub fn default_mail_client() -> String {
    "/usr/bin/thunderbird".into()
}
pub fn default_interval() -> u64 {
    60
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "default_mail_client")]
    pub mail_client: String,
    #[serde(default = "default_interval")]
    pub interval_secs: u64,
    #[serde(rename = "account", default)]
    pub accounts: Vec<Account>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            mail_client: default_mail_client(),
            interval_secs: default_interval(),
            accounts: vec![],
        }
    }
}

pub fn read(path: &Path) -> Result<Config, String> {
    let s = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&s).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Write the config to disk with mode 0600. Renders TOML by hand instead of
/// going through `toml::to_string` so the file stays human-friendly (stable
/// key order, comment header) and round-trips cleanly with `--configure`.
pub fn write(path: &Path, cfg: &Config) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    out.push_str("# tb-tray config — passwords are plaintext, file mode is 0600.\n\n");
    out.push_str(&format!(
        "mail_client    = {}\n",
        toml_quote(&cfg.mail_client)
    ));
    out.push_str(&format!("interval_secs  = {}\n", cfg.interval_secs));
    for acc in &cfg.accounts {
        out.push_str("\n[[account]]\n");
        out.push_str(&format!("name     = {}\n", toml_quote(&acc.name)));
        out.push_str(&format!("server   = {}\n", toml_quote(&acc.server)));
        out.push_str(&format!("port     = {}\n", acc.port));
        out.push_str(&format!("username = {}\n", toml_quote(&acc.username)));
        out.push_str(&format!("password = {}\n", toml_quote(&acc.password)));
        out.push_str(&format!("folder   = {}\n", toml_quote(&acc.folder)));
    }
    std::fs::write(path, out)?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

pub fn write_sample(path: &Path) -> std::io::Result<()> {
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
    std::fs::write(path, sample)
}

fn toml_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}
