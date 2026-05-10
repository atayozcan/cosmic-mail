//! Shared types for the cosmic-mail applet and its settings binary.
//!
//! Both depend on this lib so config schema stays in one place.
//!
//! Storage is split across three backends:
//!
//! - Non-secret settings (mail client, poll interval) live in
//!   `cosmic_config` and benefit from cross-process live reload.
//! - Account metadata (display name, server URL, username) lives in
//!   `~/.config/cosmic-mail/accounts.toml` at mode 0600.
//! - Account passwords live in the freedesktop Secret Service
//!   (gnome-keyring, kwallet, KeePassXC) via [`secrets`].

pub mod accounts;
pub mod localize;
pub mod secrets;
pub mod settings;

pub const APP_ID: &str = "io.github.atayozcan.CosmicMail";
pub const BIN_NAME: &str = "cosmic-mail";

/// Path to the 0600 accounts file, kept separate from cosmic_config.
pub fn accounts_path() -> std::path::PathBuf {
    dirs::config_dir()
        .expect("no XDG_CONFIG_HOME and no $HOME")
        .join(BIN_NAME)
        .join("accounts.toml")
}
