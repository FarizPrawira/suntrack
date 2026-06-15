//! The dashboard page: hero summary and the per-app list with drill-down.

use super::theme::*;
use super::widgets::{app_row, icon_button, idle_row, title_row, toggle_button};
use crate::app::{Page, TrackerApp};
use crate::db;
use crate::state::{IDLE, UsageRow};
use chrono::Local;
use eframe::egui::{self, Ui};
use egui_phosphor::regular;
use std::sync::atomic::Ordering;

impl TrackerApp {
    pub(crate) fn tracker_page(&mut self, ui: &mut Ui) {
        let today = Local::now().date_naive();
        let is_live = self.selected_date.is_none();
        let viewed = self.selected_date.unwrap_or(today);
        let is_tracking = self.state.tracking.load(Ordering::Relaxed);

        ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);

        // ---------- Header ----------
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;

            // Primary toggle: the icon and color carry the state, so no separate
            // Live/Paused indicator is needed.
            if toggle_button(ui, is_tracking, false) {
                self.state.tracking.store(!is_tracking, Ordering::Relaxed);
            }

            // Right-aligned day stepper: ◀ [ date ] ▶, where the date is itself a
            // button that opens the calendar. All three share the nav button
            // style, so they read as one control group.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, regular::CARET_RIGHT, "Next day", !is_live)
                    && let Some(d) = viewed.succ_opt()
                {
                    self.view_date(d, today);
                }

                let label = if is_live {
                    "Today".to_string()
                } else {
                    viewed.format("%a, %d %b").to_string()
                };
                let date_btn = egui::Button::new(
                    egui::RichText::new(format!("{}  {label}", regular::CALENDAR)).size(14.0),
                )
                .corner_radius(BTN_RADIUS)
                .min_size(egui::vec2(132.0, CONTROL_H));
                if ui.add(date_btn).on_hover_text("Open calendar").clicked() {
                    // Force the heatmap to refetch on open.
                    self.loaded_month = None;
                    self.page = Page::Calendar;
                }

                if icon_button(ui, regular::CARET_LEFT, "Previous day", true)
                    && let Some(d) = viewed.pred_opt()
                {
                    self.view_date(d, today);
                }
            });
        });

        ui.separator();

        // ---------- Gather + split data ----------
        // App-level totals: live aggregation for today, grouped DB rows for a
        // past day.
        let snapshot: Vec<UsageRow> = match self.selected_date {
            None => self.state.app_totals(),
            Some(_) => self.day_usage.clone(),
        };

        let idle_secs: u64 = snapshot
            .iter()
            .filter(|r| r.name == IDLE)
            .map(|r| r.secs)
            .sum();
        let mut apps: Vec<UsageRow> = snapshot.into_iter().filter(|r| r.name != IDLE).collect();
        apps.sort_by(UsageRow::by_usage_desc);

        let total_active: u64 = apps.iter().map(|r| r.secs).sum();
        // Scale the app bars to the most-used app so it fills the row; Idle is a
        // separate, de-emphasized category and must not shrink the app bars.
        let bar_max = apps.first().map(|r| r.secs).unwrap_or(0);

        if apps.is_empty() && idle_secs == 0 {
            ui.add_space(40.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("No activity tracked")
                        .size(15.0)
                        .color(MUTED),
                );
                let hint = if is_live {
                    "Tracking records time as soon as you switch apps."
                } else {
                    "Nothing was recorded on this day."
                };
                ui.label(egui::RichText::new(hint).size(12.0).color(MUTED));
            });
            return;
        }

        // ---------- Hero summary ----------
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(fmt_duration(total_active))
                    .size(32.0)
                    .strong()
                    .color(egui::Color32::WHITE),
            );
            ui.label(
                egui::RichText::new(if is_live { "active today" } else { "active" })
                    .size(14.0)
                    .color(MUTED),
            );
        });

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 14.0;
            if let Some(top) = apps.first() {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    ui.label(egui::RichText::new(regular::TROPHY).size(12.0).color(GOLD));
                    ui.label(egui::RichText::new(&top.name).size(12.0).color(MUTED));
                });
            }
            if idle_secs > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "{}  {}",
                        regular::COFFEE,
                        fmt_duration(idle_secs)
                    ))
                    .size(12.0)
                    .color(MUTED),
                );
            }
            ui.label(
                egui::RichText::new(format!("{}  {} apps", regular::SQUARES_FOUR, apps.len()))
                    .size(12.0)
                    .color(MUTED),
            );
        });

        ui.add_space(14.0);

        // ---------- App list (scrollable; click a row to drill down) ----------
        let date_str = self.selected_date.map(db::date_key);
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                for row in &apps {
                    let color = app_color(&row.name);
                    let expanded = self.expanded_app.as_deref() == Some(row.name.as_str());

                    if app_row(
                        ui,
                        &row.name,
                        row.secs,
                        total_active,
                        bar_max,
                        color,
                        expanded,
                    ) {
                        if expanded {
                            self.expanded_app = None;
                        } else {
                            self.expanded_app = Some(row.name.clone());
                            if let Some(ref ds) = date_str {
                                self.expanded_titles = db::or_warn(
                                    db::titles_for(&self.conn, ds, &row.name),
                                    "load title breakdown",
                                );
                            }
                        }
                    }

                    // Title breakdown beneath the expanded app.
                    if self.expanded_app.as_deref() == Some(row.name.as_str()) {
                        let titles = self.titles_for(&row.name);
                        let title_max = titles.iter().map(|t| t.secs).max().unwrap_or(0);
                        if titles.is_empty() {
                            title_row(ui, "(no window titles recorded)", 0, 0, color);
                        } else {
                            for t in &titles {
                                title_row(ui, &t.name, t.secs, title_max, color);
                            }
                        }
                    }
                }

                // Idle (de-emphasized), scrolls with the list.
                if idle_secs > 0 {
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(2.0);
                    idle_row(ui, idle_secs, bar_max);
                }
            });
    }
}
