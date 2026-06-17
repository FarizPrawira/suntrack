//! The left navigation sidebar: brand, page nav, and the tracking-status footer.

use super::theme::*;
use super::widgets::nav_item;
use crate::app::{Page, TrackerApp};
use eframe::egui::{self, Ui};
use egui_phosphor::regular;
use std::sync::atomic::Ordering;

impl TrackerApp {
    pub(crate) fn sidebar(&mut self, ui: &mut Ui) {
        // Tracking-status footer, pinned to the bottom via a nested bottom panel
        // (reliable regardless of how the rail's content area is sized).
        egui::Panel::bottom("nav_footer")
            .frame(egui::Frame::new())
            .show_inside(ui, |ui| {
                ui.add_space(8.0);
                self.status_footer(ui);
            });

        // Brand + page nav fill the area above the footer.
        egui::CentralPanel::default()
            .frame(egui::Frame::new())
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.label(glyph(regular::SUN).size(19.0).color(ACCENT));
                    ui.add_space(2.0);
                    ui.label(medium(egui::RichText::new("Suntrack").size(15.0)));
                });
                ui.add_space(14.0);

                if nav_item(ui, regular::CLOCK, "Today", self.page == Page::Tracker) {
                    self.page = Page::Tracker;
                }
                ui.add_space(2.0);
                if nav_item(ui, regular::CALENDAR, "Calendar", self.page == Page::Calendar) {
                    // Force the heatmap to refetch when the page opens.
                    self.loaded_month = None;
                    self.page = Page::Calendar;
                }
                ui.add_space(2.0);
                if nav_item(ui, regular::FOLDER, "Sessions", self.page == Page::Sessions) {
                    self.refresh_sessions();
                    self.refresh_manage_sessions();
                    self.renaming = None;
                    self.page = Page::Sessions;
                }
            });
    }

    // Passive tracking status at the foot of the rail. The control itself lives
    // on the Today hero (and the empty-state CTA) — this is just a readout.
    fn status_footer(&mut self, ui: &mut Ui) {
        let is_tracking = self.state.tracking.load(Ordering::Relaxed);
        let active = self.state.current_session.load(Ordering::Relaxed);
        let session_name = self
            .sessions
            .iter()
            .find(|s| s.id == active)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "General".to_string());

        ui.horizontal(|ui| {
            let (dot, label) = if is_tracking {
                (RECORDING, "Recording")
            } else {
                (IDLE_COLOR, "Paused")
            };
            let (r, _) = ui.allocate_exact_size(egui::vec2(9.0, 9.0), egui::Sense::hover());
            ui.painter().circle_filled(r.center(), 3.5, dot);
            ui.add_space(2.0);
            ui.label(egui::RichText::new(label).size(12.5).color(TEXT));
        });
        ui.add_space(3.0);
        ui.label(
            egui::RichText::new(format!("Session · {session_name}"))
                .size(12.0)
                .color(FAINT),
        );
    }
}
