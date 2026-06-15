//! The collapsed, always-on-top mini-HUD and its window transitions.

use super::theme::*;
use super::widgets::{color_dot, icon_button, toggle_button};
use crate::app::TrackerApp;
use crate::state::IDLE;
use eframe::egui::{self, Ui};
use egui_phosphor::regular;
use std::sync::atomic::Ordering;

impl TrackerApp {
    // Collapse the main window into the always-on-top mini-HUD.
    pub(crate) fn enter_mini(&mut self, ctx: &egui::Context) {
        self.is_mini = true;

        let pos = self.hud_pos.unwrap_or_else(|| {
            let monitor = ctx
                .input(|i| i.viewport().monitor_size)
                .unwrap_or(egui::vec2(1280.0, 720.0));
            // Bottom-left corner, clear of the taskbar.
            egui::pos2(24.0, monitor.y - HUD_SIZE.y - 60.0)
        });

        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(HUD_SIZE));
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
    }

    // Restore the full dashboard window.
    pub(crate) fn exit_mini(&mut self, ctx: &egui::Context) {
        self.is_mini = false;
        ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
            egui::WindowLevel::Normal,
        ));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(self.full_size));
        if let Some(pos) = self.full_pos {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(pos));
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
    }

    pub(crate) fn mini_hud(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let is_tracking = self.state.tracking.load(Ordering::Relaxed);

        // The live figures the HUD shows, reusing the shared per-app aggregation
        // so the HUD total can't drift from the dashboard's.
        let active_app = self.state.current_app.lock().unwrap().clone();
        let totals = self.state.app_totals();
        let total_active: u64 = totals
            .iter()
            .filter(|r| r.name != IDLE)
            .map(|r| r.secs)
            .sum();
        let active_secs: u64 = active_app
            .as_ref()
            .and_then(|a| totals.iter().find(|r| &r.name == a))
            .map(|r| r.secs)
            .unwrap_or(0);

        let mut toggle = false;
        let mut expand = false;

        egui::Frame::new()
            .fill(HUD_BG)
            .stroke(egui::Stroke::new(0.5, HUD_BORDER))
            .inner_margin(egui::Margin::same(12))
            .show(ui, |ui| {
                // Drag anywhere on the HUD to move it; right-click for the menu.
                let drag = ui.interact(
                    ui.max_rect(),
                    egui::Id::new("suntrack_hud_drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                drag.context_menu(|ui| {
                    if ui.button("Open Suntrack").clicked() {
                        expand = true;
                        ui.close();
                    }
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        ui.close();
                    }
                });

                // Top row: play/pause + big total + expand.
                ui.horizontal(|ui| {
                    if toggle_button(ui, is_tracking, true) {
                        toggle = true;
                    }

                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            egui::RichText::new(fmt_duration(total_active))
                                .size(24.0)
                                .strong()
                                .color(egui::Color32::WHITE),
                        );
                        ui.label(egui::RichText::new("active today").size(11.0).color(MUTED));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, regular::ARROWS_OUT, "Expand to full window", true) {
                            expand = true;
                        }
                    });
                });

                ui.add_space(8.0);

                // Bottom row: the app you're currently in (or the paused state).
                ui.horizontal(|ui| {
                    if !is_tracking {
                        color_dot(ui, IDLE_COLOR, 4.5);
                        ui.label(egui::RichText::new("Paused").size(13.0).color(MUTED));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new("not tracking").size(12.0).color(MUTED));
                        });
                    } else if let Some(app) = &active_app {
                        color_dot(ui, app_color(app), 4.5);
                        ui.label(egui::RichText::new(app).size(13.0).color(TEXT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(fmt_duration(active_secs))
                                    .size(13.0)
                                    .color(TEXT),
                            );
                        });
                    } else {
                        color_dot(ui, IDLE_COLOR, 4.5);
                        ui.label(egui::RichText::new("Waiting…").size(13.0).color(MUTED));
                    }
                });
            });

        if toggle {
            self.state.tracking.store(!is_tracking, Ordering::Relaxed);
        }
        if expand {
            self.exit_mini(ctx);
        }

        // Once a drag is released (pointer up), snap the HUD to the nearest
        // screen edge so it can only ever rest on an edge.
        let (outer, monitor, pointer_down) = ctx.input(|i| {
            (
                i.viewport().outer_rect,
                i.viewport().monitor_size,
                i.pointer.any_down(),
            )
        });
        if let (Some(rect), Some(monitor), false) = (outer, monitor, pointer_down) {
            let snapped = snap_to_edge(rect, monitor);
            if snapped.distance(rect.min) > 1.0 {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(snapped));
            }
            self.hud_pos = Some(snapped);
        }
    }
}

// Snap the HUD's top-left so it sits against the nearest screen edge, while
// staying slidable along that edge — it can never rest free-floating.
fn snap_to_edge(rect: egui::Rect, monitor: egui::Vec2) -> egui::Pos2 {
    const MARGIN: f32 = 16.0;
    const BOTTOM_MARGIN: f32 = 48.0; // clear of the taskbar

    let size = rect.size();
    let left = rect.min.x;
    let right = monitor.x - rect.max.x;
    let top = rect.min.y;
    let bottom = monitor.y - rect.max.y;
    let nearest = left.min(right).min(top).min(bottom);

    // Position along the docked edge, clamped within the monitor.
    let x = rect
        .min
        .x
        .clamp(MARGIN, (monitor.x - size.x - MARGIN).max(MARGIN));
    let y = rect
        .min
        .y
        .clamp(MARGIN, (monitor.y - size.y - BOTTOM_MARGIN).max(MARGIN));

    if nearest == left {
        egui::pos2(MARGIN, y)
    } else if nearest == right {
        egui::pos2(monitor.x - size.x - MARGIN, y)
    } else if nearest == top {
        egui::pos2(x, MARGIN)
    } else {
        egui::pos2(x, monitor.y - size.y - BOTTOM_MARGIN)
    }
}
