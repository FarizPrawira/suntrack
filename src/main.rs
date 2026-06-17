//! Suntrack — a lightweight desktop time tracker.

// On Windows a console-subsystem binary opens a terminal window alongside the
// GUI. Mark release builds as a GUI app so only the dashboard window appears;
// debug builds keep the console so `eprintln!` diagnostics stay visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod db;
mod icon;
mod paths;
mod state;
mod tracker;
mod view;

use crate::state::SharedState;
use eframe::egui;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::Duration;

fn main() -> eframe::Result {
    let config = config::load();
    let conn = db::init_db();

    // Resolve the session to start in (named default, or the last one used) and
    // persist it so "continue last" survives even an immediate, untracked close.
    let session = db::resolve_start_session(&conn, &config.default_session);
    db::or_warn(
        db::set_meta(&conn, db::ACTIVE_SESSION_KEY, &session.to_string()),
        "persist active session",
    );

    let state = SharedState::new(config.start_tracking_on_launch, session);

    let tracker_state = state.clone();
    let tracker_config = config.clone();
    thread::spawn(move || tracker::run_tracker(tracker_state, tracker_config));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 640.0])
            // Allow shrinking down to the mini-HUD size.
            .with_min_inner_size([280.0, 88.0])
            .with_icon(Arc::new(icon::sun_icon())),
        ..Default::default()
    };
    eframe::run_native(
        "Suntrack",
        options,
        Box::new(move |cc| {
            // Bundle Inter (the UI typeface) and the Phosphor icon font.
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "Inter".to_owned(),
                Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/Inter-Regular.ttf"
                ))),
            );
            fonts.font_data.insert(
                "Inter-Medium".to_owned(),
                Arc::new(egui::FontData::from_static(include_bytes!(
                    "../assets/Inter-Medium.ttf"
                ))),
            );
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            // Inter is the primary proportional text face (phosphor stays in the
            // chain as a fallback for any un-tagged icon).
            if let Some(prop) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                prop.retain(|f| f != "Inter");
                prop.insert(0, "Inter".to_owned());
            }
            // Icons render through this Phosphor-only family (see theme::glyph) so
            // Inter's few colliding private-use glyphs (folder, carets) can't
            // shadow them — while Latin text still uses Inter.
            fonts.families.insert(
                egui::FontFamily::Name("icons".into()),
                vec!["phosphor".to_owned()],
            );
            // A named family for medium-weight headings/emphasis (egui has no
            // synthetic bold, so the weight is a separate face).
            fonts.families.insert(
                egui::FontFamily::Name("medium".into()),
                vec!["Inter-Medium".to_owned(), "Inter".to_owned()],
            );
            cc.egui_ctx.set_fonts(fonts);

            // Dark + amber palette for all stock widgets.
            crate::view::theme::configure_visuals(&cc.egui_ctx);

            // eframe suspends the update loop while minimized, so the minimize
            // can't be caught in the UI. A side thread holding a clone of the
            // context watches for it, undoes it, and signals the app to dock.
            let watch_ctx = cc.egui_ctx.clone();
            let wants_mini = Arc::clone(&state.wants_mini);
            thread::spawn(move || {
                let mut was_minimized = false;
                loop {
                    thread::sleep(Duration::from_millis(120));
                    let minimized = watch_ctx.input(|i| i.viewport().minimized) == Some(true);
                    if minimized && !was_minimized {
                        wants_mini.store(true, Ordering::Relaxed);
                        watch_ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                        watch_ctx.request_repaint();
                    }
                    was_minimized = minimized;
                }
            });

            Ok(Box::new(app::TrackerApp::new(state, conn, config)))
        }),
    )
}
