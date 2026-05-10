//! Non-secret cosmic-mail settings, stored via `cosmic_config`.
//!
//! Lives at `~/.config/cosmic/io.github.atayozcan.CosmicMail/v1/<field>`,
//! one RON-encoded file per field. cosmic_config gives us cross-process
//! live reload via inotify and matches what the rest of the COSMIC
//! ecosystem uses. JMAP credentials are *not* here — see
//! [`crate::accounts`] for the 0600 file that holds those.

use cosmic_config::{Config, CosmicConfigEntry};
// Derive macro lives in the sibling `cosmic-config-derive` crate, re-exported
// by `cosmic_config` as a sub-module. Macros and traits inhabit different
// namespaces, so this second `use` doesn't shadow the trait import above.
use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

use crate::APP_ID;

pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, CosmicConfigEntry)]
#[version = 1]
pub struct Settings {
    /// Path to the mail client to launch when the user clicks the
    /// notification's Open action. Empty string disables the launch.
    pub mail_client: String,
    /// Seconds between JMAP polls. Floored to 10 at runtime so a
    /// user-typo of `1` doesn't hammer the server.
    pub interval_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // freedesktop-standard mail composer launcher; no
            // hardcoding to a particular client. Users who want their
            // inbox to open instead can set this to e.g.
            // `/usr/bin/thunderbird` in settings.
            mail_client: "xdg-email".into(),
            interval_secs: 60,
        }
    }
}

pub fn handler() -> Result<Config, cosmic_config::Error> {
    Config::new(APP_ID, CONFIG_VERSION)
}

pub fn load() -> Settings {
    let Ok(h) = handler() else {
        return Settings::default();
    };
    match Settings::get_entry(&h) {
        Ok(s) => s,
        Err((errs, s)) => {
            for e in errs {
                eprintln!("cosmic-mail: settings: {e}");
            }
            s
        }
    }
}

pub fn save(s: &Settings) -> Result<(), cosmic_config::Error> {
    let h = handler()?;
    s.write_entry(&h)
}
