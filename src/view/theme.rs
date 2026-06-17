//! Colors, sizes, and small formatting helpers shared across the UI.

use eframe::egui::{self, Color32};

// ---- Text ----
pub(crate) const TEXT: Color32 = Color32::from_rgb(0xec, 0xec, 0xee);
pub(crate) const MUTED: Color32 = Color32::from_rgb(0x9a, 0x9a, 0xa2);
// Faintest text tier (hints, secondary lines, future days).
pub(crate) const FAINT: Color32 = Color32::from_rgb(0x6a, 0x6a, 0x72);

// ---- Surfaces ---- (window bg, sidebar, raised cards, hover, borders)
pub(crate) const BG: Color32 = Color32::from_rgb(0x15, 0x15, 0x17);
pub(crate) const SURFACE: Color32 = Color32::from_rgb(0x1b, 0x1b, 0x1f);
pub(crate) const CARD: Color32 = Color32::from_rgb(0x20, 0x20, 0x26);
pub(crate) const HOVER_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x31);
pub(crate) const CARD_BORDER: Color32 = Color32::from_rgb(0x2c, 0x2c, 0x33);
pub(crate) const HAIRLINE: Color32 = Color32::from_rgb(0x26, 0x26, 0x2c);

// ---- Amber accent (the "sun") ---- used for brand, active state, top app.
pub(crate) const ACCENT: Color32 = Color32::from_rgb(0xF2, 0xB3, 0x3D);
pub(crate) const ACCENT_HOVER: Color32 = Color32::from_rgb(0xf7, 0xc4, 0x6a);
// A solid amber-on-dark tint for highlighted rows/controls (no alpha blending).
pub(crate) const ACCENT_TINT: Color32 = Color32::from_rgb(0x42, 0x38, 0x2a);
// Dark brown text for placing on bright amber fills (pills, bright heat cells).
pub(crate) const PILL_TEXT: Color32 = Color32::from_rgb(0x2a, 0x1d, 0x05);
// Green is reserved purely for the live "recording" indicator (sidebar footer).
pub(crate) const RECORDING: Color32 = Color32::from_rgb(0x5f, 0xd0, 0x7a);

pub(crate) const TRACK_BG: Color32 = Color32::from_rgb(0x2a, 0x2a, 0x2a);
pub(crate) const IDLE_COLOR: Color32 = Color32::from_rgb(0x6a, 0x6a, 0x72);
// Primary toggle button: green while tracking, neutral while paused.
pub(crate) const ACCENT_BG: Color32 = Color32::from_rgb(0x1f, 0x4d, 0x2e);
pub(crate) const ACCENT_FG: Color32 = Color32::from_rgb(0x8f, 0xe0, 0xa6);
pub(crate) const BTN_BG: Color32 = CARD;
// The most-used app is highlighted in amber (matching the trophy and brand).
pub(crate) const GOLD: Color32 = ACCENT;
// Mini-HUD window size and chrome. Tall enough that the 12px content margins
// hold around both rows (the toggle + two stacked rows need ~92px).
pub(crate) const HUD_SIZE: egui::Vec2 = egui::vec2(280.0, 96.0);
pub(crate) const HUD_BG: Color32 = SURFACE;
pub(crate) const HUD_BORDER: Color32 = CARD_BORDER;

// Shared toolbar control geometry, so every button in the header/nav rows keeps
// the same height and rounding.
pub(crate) const CONTROL_H: f32 = 30.0;
pub(crate) const BTN_RADIUS: f32 = 8.0;

// Apply the dark + amber palette to egui's widgets, panels, and selection, so
// stock controls (buttons, menus, separators) adopt the theme without every
// call site restyling them. Per-text colors set via RichText still win.
pub(crate) fn configure_visuals(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();

    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = BG;
    v.faint_bg_color = HOVER_BG;
    v.window_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    v.window_corner_radius = egui::CornerRadius::same(12);

    v.selection.bg_fill = ACCENT_TINT;
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    v.hyperlink_color = ACCENT;

    let radius = egui::CornerRadius::same(8);

    let n = &mut v.widgets.noninteractive;
    n.bg_fill = CARD;
    n.weak_bg_fill = CARD;
    n.bg_stroke = egui::Stroke::new(1.0, HAIRLINE); // separators / hairlines
    n.fg_stroke = egui::Stroke::new(1.0, TEXT); // default (label) text color
    n.corner_radius = radius;

    let i = &mut v.widgets.inactive;
    i.bg_fill = CARD;
    i.weak_bg_fill = CARD;
    i.bg_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    i.fg_stroke = egui::Stroke::new(1.0, TEXT);
    i.corner_radius = radius;

    let h = &mut v.widgets.hovered;
    h.bg_fill = HOVER_BG;
    h.weak_bg_fill = HOVER_BG;
    h.bg_stroke = egui::Stroke::new(1.0, ACCENT_HOVER);
    h.fg_stroke = egui::Stroke::new(1.0, TEXT);
    h.corner_radius = radius;

    let a = &mut v.widgets.active;
    a.bg_fill = ACCENT_TINT;
    a.weak_bg_fill = ACCENT_TINT;
    a.bg_stroke = egui::Stroke::new(1.0, ACCENT);
    a.fg_stroke = egui::Stroke::new(1.0, TEXT);
    a.corner_radius = radius;

    let o = &mut v.widgets.open;
    o.bg_fill = CARD;
    o.weak_bg_fill = CARD;
    o.bg_stroke = egui::Stroke::new(1.0, CARD_BORDER);
    o.corner_radius = radius;

    ctx.set_visuals(v);
}

// Apply the medium-weight Inter face to a RichText. egui has no synthetic bold,
// so emphasis is a distinct registered font family (see main.rs).
pub(crate) fn medium(text: egui::RichText) -> egui::RichText {
    text.family(egui::FontFamily::Name("medium".into()))
}

// Render a Phosphor icon glyph through the icon-only font family, so Inter's
// few colliding private-use glyphs can't shadow it.
pub(crate) fn glyph(g: &str) -> egui::RichText {
    egui::RichText::new(g).family(egui::FontFamily::Name("icons".into()))
}

// Human-friendly duration: "6h 47m", "1m 58s", or "2s". The smaller unit is
// dropped when it's zero, so whole values stay clean ("6h", not "6h 0m").
pub(crate) fn fmt_duration(secs: u64) -> String {
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        if m > 0 { format!("{h}h {m}m") } else { format!("{h}h") }
    } else if m > 0 {
        if s > 0 { format!("{m}m {s}s") } else { format!("{m}m") }
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

// Calendar heatmap cell colors for a day's active seconds: (fill, text). A warm
// "sun" ramp deepens from a dark surface through amber with tracked hours; text
// flips to dark on the bright cells.
pub(crate) fn heat_cell(secs: u64) -> (Color32, Color32) {
    let hours = secs as f64 / 3600.0;
    if secs == 0 {
        (CARD, Color32::from_rgb(0x55, 0x55, 0x5c))
    } else if hours < 1.0 {
        (
            Color32::from_rgb(0x3d, 0x34, 0x22),
            Color32::from_rgb(0xb8, 0xb8, 0xbd),
        )
    } else if hours < 3.0 {
        (
            Color32::from_rgb(0x6b, 0x53, 0x20),
            Color32::from_rgb(0xec, 0xec, 0xee),
        )
    } else if hours < 6.0 {
        (
            Color32::from_rgb(0xb0, 0x7f, 0x24),
            Color32::from_rgb(0x2a, 0x1d, 0x05),
        )
    } else {
        (ACCENT, Color32::from_rgb(0x2a, 0x1d, 0x05))
    }
}
