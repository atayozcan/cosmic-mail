use std::path::PathBuf;

pub const APP_ID: &str = "io.github.atayozcan.TbTray";

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .expect("no XDG_CONFIG_HOME")
        .join("tb-tray/config.toml")
}

pub fn autostart_path() -> PathBuf {
    dirs::config_dir()
        .expect("no XDG_CONFIG_HOME")
        .join("autostart/tb-tray.desktop")
}

/// The absolute path of the running binary, used for the autostart entry
/// and for re-launching ourselves in `--settings` mode.
pub fn self_exec() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "tb-tray".to_string())
}
