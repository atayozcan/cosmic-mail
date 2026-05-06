//! Shared types and helpers for the tb-tray daemon and settings GUI.
//!
//! The daemon binary (`src/main.rs`) and the libcosmic settings binary
//! (`src/bin/tb-tray-settings.rs`) both depend on this lib so config
//! parsing, paths, and autostart-file handling stay in one place.

pub mod autostart;
pub mod config;
pub mod paths;
