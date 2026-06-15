//! Types shared between the UI thread and the background tracker thread.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

// Reserved app names. Single source of truth for the active/idle split that the
// tracker, UI, and DB all key off, so a stray literal can't quietly break it.
pub const IDLE: &str = "Idle";
// Recorded when the foreground window can't be identified.
pub const UNKNOWN: &str = "Unknown";

// A slice of tracked time, keyed by app and window title.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ActivityKey {
    pub app: String,
    pub title: String,
}

impl ActivityKey {
    pub fn new(app: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            app: app.into(),
            title: title.into(),
        }
    }

    // Idle time is tracked under a reserved app name and excluded from totals.
    pub fn idle() -> Self {
        Self {
            app: IDLE.to_string(),
            title: String::new(),
        }
    }
}

// In-memory accumulation of tracked seconds, keyed by (app, title).
pub type UsageMap = HashMap<ActivityKey, u64>;

// A display row: a label (app name, or window title in the drill-down) and its
// accumulated seconds.
#[derive(Clone, Debug)]
pub struct UsageRow {
    pub name: String,
    pub secs: u64,
}

impl UsageRow {
    pub fn new(name: impl Into<String>, secs: u64) -> Self {
        Self {
            name: name.into(),
            secs,
        }
    }

    // Display order: most time first, ties broken by name. Shared by the app
    // list and the title drill-down so they sort identically.
    pub fn by_usage_desc(a: &UsageRow, b: &UsageRow) -> Ordering {
        b.secs.cmp(&a.secs).then_with(|| a.name.cmp(&b.name))
    }
}

// The handful of values the UI and tracker threads share, cloned as a unit.
#[derive(Clone)]
pub struct SharedState {
    // Today's tracked time, accumulated live by the tracker.
    pub usage: Arc<Mutex<UsageMap>>,
    // Tracker on/off switch; starts enabled.
    pub tracking: Arc<AtomicBool>,
    // App currently being recorded (None when paused), shown in the mini-HUD.
    pub current_app: Arc<Mutex<Option<String>>>,
    // Raised by the minimize watcher to ask the UI to collapse to the mini-HUD —
    // needed because the update loop is suspended while the window is minimized.
    pub wants_mini: Arc<AtomicBool>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            usage: Arc::new(Mutex::new(UsageMap::new())),
            tracking: Arc::new(AtomicBool::new(true)),
            current_app: Arc::new(Mutex::new(None)),
            wants_mini: Arc::new(AtomicBool::new(false)),
        }
    }

    // Per-app totals from the live map. The one place this aggregation lives, so
    // the dashboard and HUD agree. Idle is included; callers filter it as needed.
    pub fn app_totals(&self) -> Vec<UsageRow> {
        let map = self.usage.lock().unwrap();
        let mut totals: HashMap<String, u64> = HashMap::new();
        for (key, secs) in map.iter() {
            *totals.entry(key.app.clone()).or_insert(0) += *secs;
        }
        totals
            .into_iter()
            .map(|(name, secs)| UsageRow::new(name, secs))
            .collect()
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}
