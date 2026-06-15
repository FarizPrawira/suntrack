//! Colors, sizes, and small formatting helpers shared across the UI.

use eframe::egui::{self, Color32};

pub(crate) const TEXT: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe4);
pub(crate) const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0x9a);
pub(crate) const TRACK_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
pub(crate) const IDLE_COLOR: Color32 = Color32::from_rgb(0x6a, 0x6a, 0x6a);
// Primary toggle button: green while tracking, neutral while paused.
pub(crate) const ACCENT_BG: Color32 = Color32::from_rgb(0x1f, 0x4d, 0x2e);
pub(crate) const ACCENT_FG: Color32 = Color32::from_rgb(0x8f, 0xe0, 0xa6);
pub(crate) const BTN_BG: Color32 = Color32::from_rgb(0x2c, 0x2c, 0x2c);
// The most-used app is always highlighted in gold (matching the trophy).
pub(crate) const GOLD: Color32 = Color32::from_rgb(0xF2, 0xC1, 0x4E);
// Mini-HUD window size and chrome.
pub(crate) const HUD_SIZE: egui::Vec2 = egui::vec2(280.0, 88.0);
pub(crate) const HUD_BG: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1b);
pub(crate) const HUD_BORDER: Color32 = Color32::from_rgb(0x3a, 0x3a, 0x3a);

// Shared toolbar control geometry, so every button in the header/nav rows keeps
// the same height and rounding.
pub(crate) const CONTROL_H: f32 = 30.0;
pub(crate) const BTN_RADIUS: f32 = 8.0;

// Human-friendly duration: "6h 47m", "1m 58s", or "2s".
pub(crate) fn fmt_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m}m")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

// Deterministic color per app name (FNV-1a hash into a fixed palette) so each
// app keeps the same color across sessions.
pub(crate) fn app_color(name: &str) -> Color32 {
    // Amber/gold is intentionally excluded — it is reserved for the top app.
    const PALETTE: [Color32; 7] = [
        Color32::from_rgb(0x37, 0x8A, 0xDD), // blue
        Color32::from_rgb(0xD4, 0x53, 0x7E), // pink
        Color32::from_rgb(0x1D, 0x9E, 0x75), // teal
        Color32::from_rgb(0x97, 0xC4, 0x59), // green
        Color32::from_rgb(0x7F, 0x77, 0xDD), // purple
        Color32::from_rgb(0xD8, 0x5A, 0x30), // coral
        Color32::from_rgb(0xE2, 0x4B, 0x4A), // red
    ];
    let mut hash: u32 = 2166136261;
    for byte in name.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(16777619);
    }
    PALETTE[(hash as usize) % PALETTE.len()]
}

// One representative value per `heat_cell` band (0, <1h, <3h, <6h, ≥6h). The
// legend renders its swatches from these, so it can't drift from the shading.
pub(crate) const HEAT_SAMPLES: [u64; 5] = [0, 1800, 7200, 4 * 3600, 8 * 3600];

// Calendar heatmap cell colors for a day's active seconds: (fill, text). The
// green ramp deepens with tracked hours; text flips to dark on the bright cells.
pub(crate) fn heat_cell(secs: u64) -> (Color32, Color32) {
    let hours = secs as f64 / 3600.0;
    if secs == 0 {
        (
            Color32::from_rgb(0x26, 0x26, 0x26),
            Color32::from_rgb(0x6a, 0x6a, 0x6a),
        )
    } else if hours < 1.0 {
        (
            Color32::from_rgb(0x1f, 0x3a, 0x26),
            Color32::from_rgb(0xcf, 0xcf, 0xcf),
        )
    } else if hours < 3.0 {
        (
            Color32::from_rgb(0x2f, 0x6b, 0x3f),
            Color32::from_rgb(0xf0, 0xf0, 0xf0),
        )
    } else if hours < 6.0 {
        (
            Color32::from_rgb(0x43, 0xa3, 0x5c),
            Color32::from_rgb(0x10, 0x20, 0x14),
        )
    } else {
        (
            Color32::from_rgb(0x5f, 0xd0, 0x7a),
            Color32::from_rgb(0x0e, 0x2a, 0x16),
        )
    }
}
