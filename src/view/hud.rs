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
        // Keep the session list current for the HUD's switcher.
        self.refresh_sessions();

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

    // Advance the active session to the next one. The HUD window is too small to
    // host a dropdown, so the session pill cycles through them on click.
    fn cycle_session(&mut self) {
        if self.sessions.len() < 2 {
            return;
        }
        let active = self.state.current_session.load(Ordering::Relaxed);
        let idx = self
            .sessions
            .iter()
            .position(|s| s.id == active)
            .unwrap_or(0);
        let next = self.sessions[(idx + 1) % self.sessions.len()].id;
        self.set_active_session(next);
    }

    pub(crate) fn mini_hud(&mut self, ui: &mut Ui, ctx: &egui::Context) {
        let is_tracking = self.state.tracking.load(Ordering::Relaxed);

        // The active session, so the HUD shows what's being recorded into.
        let active = self.state.current_session.load(Ordering::Relaxed);
        let session_name = self
            .sessions
            .iter()
            .find(|s| s.id == active)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "General".to_string());

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
        let mut cycle = false;

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
                    ui.add_space(6.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            medium(egui::RichText::new(fmt_duration(total_active)).size(22.0))
                                .color(egui::Color32::WHITE),
                        );
                        ui.label(egui::RichText::new("active today").size(11.0).color(FAINT));
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, regular::ARROWS_OUT, "Expand to full window", true) {
                            expand = true;
                        }
                    });
                });

                ui.add_space(8.0);

                // Bottom row: the session pill (click to switch) + current app.
                ui.horizontal(|ui| {
                    let pill = egui::Frame::new()
                        .fill(CARD)
                        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                        .corner_radius(7.0)
                        .inner_margin(egui::Margin::symmetric(8, 3))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                ui.label(glyph(regular::FOLDER).size(11.0).color(MUTED));
                                ui.label(egui::RichText::new(&session_name).size(12.0).color(TEXT));
                            });
                        })
                        .response
                        .interact(egui::Sense::click())
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text("Click to switch session");
                    if pill.clicked() {
                        cycle = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if !is_tracking {
                            ui.label(egui::RichText::new("Paused").size(12.0).color(MUTED));
                        } else if let Some(app) = &active_app {
                            ui.label(
                                egui::RichText::new(fmt_duration(active_secs))
                                    .size(12.0)
                                    .color(MUTED),
                            );
                            ui.add_space(6.0);
                            ui.add(
                                egui::Label::new(egui::RichText::new(app).size(12.5).color(TEXT))
                                    .truncate(),
                            );
                            color_dot(ui, app_color(app), 4.0);
                        } else {
                            ui.label(egui::RichText::new("Waiting…").size(12.0).color(MUTED));
                        }
                    });
                });
            });

        if toggle {
            self.state.tracking.store(!is_tracking, Ordering::Relaxed);
        }
        if cycle {
            self.cycle_session();
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
