//! Configuration loaded from a TOML file in the data dir.

use crate::paths;
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    pub idle_timeout_secs: u64,
    pub save_interval_secs: u64,
    pub refresh_rate_secs: u64,
    // Whether to begin tracking the moment the app opens. False = start paused.
    pub start_tracking_on_launch: bool,
    // Session to record into on launch. Empty = continue the last one used;
    // otherwise the named session (created if missing).
    pub default_session: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timeout_secs: 60,
            save_interval_secs: 15,
            refresh_rate_secs: 1,
            start_tracking_on_launch: false,
            default_session: String::new(),
        }
    }
}

impl Config {
    // Guard against zero/invalid values that would break timing math.
    fn sanitized(mut self) -> Self {
        self.idle_timeout_secs = self.idle_timeout_secs.max(1);
        self.save_interval_secs = self.save_interval_secs.max(1);
        self.refresh_rate_secs = self.refresh_rate_secs.max(1);
        self
    }
}

// The self-documenting file written when no config exists. Built from
// `Config::default()` so the seeded values never drift from the real defaults.
fn default_config_toml() -> String {
    let d = Config::default();
    format!(
        "# Suntrack configuration. Edit and restart to apply.\n\n\
         # Seconds of inactivity before time is logged as \"Idle\".\n\
         idle_timeout_secs = {}\n\n\
         # How often (seconds) accumulated time is written to the database.\n\
         save_interval_secs = {}\n\n\
         # Tracker sampling interval (seconds). Lower = more responsive, slightly more CPU.\n\
         refresh_rate_secs = {}\n\n\
         # Begin tracking the moment the app opens. false = start paused.\n\
         start_tracking_on_launch = {}\n\n\
         # Session to record into on launch. Empty continues the last session you\n\
         # used; a name (e.g. \"Work\") always starts there, creating it if needed.\n\
         default_session = \"{}\"\n",
        d.idle_timeout_secs,
        d.save_interval_secs,
        d.refresh_rate_secs,
        d.start_tracking_on_launch,
        d.default_session
    )
}

// Missing fields fall back to defaults; a missing file is seeded with them; a
// malformed file falls back to defaults with a warning.
pub fn load() -> Config {
    let path = paths::config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => {
            // Tolerate a leading UTF-8 BOM (Notepad and some editors add one),
            // which would otherwise make the first key unparseable and silently
            // reset every setting to its default.
            let contents = contents.strip_prefix('\u{feff}').unwrap_or(&contents);
            match toml::from_str::<Config>(contents) {
                Ok(config) => config.sanitized(),
                Err(err) => {
                    eprintln!("suntrack: invalid {} ({err}); using defaults", path.display());
                    Config::default()
                }
            }
        }
        Err(_) => {
            if let Err(err) = fs::write(&path, default_config_toml()) {
                eprintln!("suntrack: could not write {} ({err})", path.display());
            }
            Config::default()
        }
    }
}
