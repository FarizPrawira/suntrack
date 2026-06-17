// The dashboard UI, split by concern. `theme` and `widgets` are shared building
// blocks; the page/HUD modules each add an `impl TrackerApp` block holding one
// screen's rendering, so app.rs only owns the struct and the eframe glue.
pub(crate) mod theme;
pub(crate) mod widgets;

mod calendar_page;
mod hud;
mod nav;
mod sessions_page;
mod tracker_page;
