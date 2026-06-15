//! Filesystem locations for the database and config.

use std::fs;
use std::path::PathBuf;

// The per-user data directory, created on demand. Prefers the Windows app-data
// folders, then the home dir, then the current dir.
pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("APPDATA"))
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    let dir = base.join("suntrack");
    if let Err(err) = fs::create_dir_all(&dir) {
        eprintln!(
            "suntrack: could not create data dir {}: {err}",
            dir.display()
        );
    }
    dir
}

pub fn db_path() -> PathBuf {
    data_dir().join("usage.db")
}

pub fn config_path() -> PathBuf {
    data_dir().join("config.toml")
}
