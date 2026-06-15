//! Suntrack — a lightweight desktop time tracker.

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
    let state = SharedState::new();

    let tracker_state = state.clone();
    thread::spawn(move || tracker::run_tracker(tracker_state, config));

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
            // Register the Phosphor icon font so toolbar glyphs render.
            let mut fonts = egui::FontDefinitions::default();
            egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
            cc.egui_ctx.set_fonts(fonts);

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
