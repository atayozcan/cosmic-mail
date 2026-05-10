//! JMAP account metadata, stored as TOML at mode 0600.
//!
//! Passwords live in the freedesktop Secret Service (see
//! [`crate::secrets`]) — never on disk in this file. accounts.toml
//! holds only what's safe to inspect: display name, server URL,
//! username.

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct Account {
    pub name: String,
    /// Either a bare hostname (`mail.example.com`) or a full URL
    /// (`https://mail.example.com`). Normalized in [`Account::base_url`].
    pub server: String,
    pub username: String,
}

impl Account {
    /// Server URL ready to feed to `jmap_client::Client::connect`.
    /// `jmap-client` itself appends `/.well-known/jmap` for discovery.
    pub fn base_url(&self) -> String {
        if self.server.starts_with("http://") || self.server.starts_with("https://") {
            self.server.clone()
        } else {
            format!("https://{}", self.server)
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AccountsFile {
    #[serde(rename = "account", default)]
    pub accounts: Vec<Account>,
}

pub fn read(path: &Path) -> Result<AccountsFile, String> {
    if !path.exists() {
        return Ok(AccountsFile::default());
    }
    let s = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    toml::from_str(&s).map_err(|e| format!("parse {}: {e}", path.display()))
}

/// Write the accounts file at mode 0600. Renders TOML by hand
/// instead of going through `toml::to_string` so the file stays
/// human-friendly (stable key order, comment header). Passwords are
/// never written here — they live in the freedesktop Secret Service
/// (see [`crate::secrets`]).
pub fn write(path: &Path, file: &AccountsFile) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    out.push_str("# cosmic-mail accounts — passwords live in the freedesktop\n");
    out.push_str("# Secret Service (gnome-keyring, kwallet, KeePassXC, …).\n");
    out.push_str("# This file is mode 0600 because the username/server pair\n");
    out.push_str("# is the lookup key into that store.\n");
    for acc in &file.accounts {
        out.push_str("\n[[account]]\n");
        out.push_str(&format!("name     = {}\n", toml_quote(&acc.name)));
        out.push_str(&format!("server   = {}\n", toml_quote(&acc.server)));
        out.push_str(&format!("username = {}\n", toml_quote(&acc.username)));
    }
    std::fs::write(path, out)?;

    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o600);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

fn toml_quote(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_url_passes_through_https() {
        let a = Account {
            name: "x".into(),
            server: "https://mail.example.com".into(),
            username: "u".into(),
        };
        assert_eq!(a.base_url(), "https://mail.example.com");
    }

    #[test]
    fn base_url_prepends_https_for_bare_hostname() {
        let a = Account {
            name: "x".into(),
            server: "mail.example.com".into(),
            username: "u".into(),
        };
        assert_eq!(a.base_url(), "https://mail.example.com");
    }
}
