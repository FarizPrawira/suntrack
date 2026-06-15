//! The month heatmap calendar page.

use super::theme::*;
use super::widgets::{icon_button, text_button};
use crate::app::{Page, TrackerApp};
use crate::db;
use chrono::{Datelike, Local, Months, NaiveDate};
use eframe::egui::{self, Ui};
use egui_phosphor::regular;

impl TrackerApp {
    pub(crate) fn calendar_page(&mut self, ui: &mut Ui) {
        // HEADER — one centered row: Back | ◀ Month Year ▶ | Today.
        // Closures only set local flags so they never borrow self.
        let today = Local::now().date_naive();
        let month_label = self.view_month.format("%B %Y").to_string();
        let mut go_back = false;
        let mut go_today = false;
        let mut prev_month = false;
        let mut next_month = false;

        ui.horizontal(|ui| {
            let gap = ui.spacing().item_spacing.x;
            let back_w = 74.0;
            let today_w = 66.0;
            let label_w = 110.0;
            let arrow_w = CONTROL_H; // matches the icon_button width for centering math
            let nav_w = arrow_w * 2.0 + label_w + gap * 2.0;
            let row_width = ui.available_width();

            // Back — far left.
            if text_button(
                ui,
                &format!("{}  Back", regular::CARET_LEFT),
                "Back to tracker",
                back_w,
            ) {
                go_back = true;
            }

            // Month nav — centered across the full row.
            let pad_left = ((row_width - nav_w) * 0.5 - back_w - gap).max(0.0);
            ui.add_space(pad_left);
            if icon_button(ui, regular::CARET_LEFT, "Previous month", true) {
                prev_month = true;
            }
            ui.add_sized(
                egui::vec2(label_w, CONTROL_H),
                egui::Label::new(month_label),
            );
            if icon_button(ui, regular::CARET_RIGHT, "Next month", true) {
                next_month = true;
            }

            // Today — far right.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if text_button(ui, "Today", "Jump to current month", today_w) {
                    go_today = true;
                }
            });
        });

        if prev_month {
            self.view_month = self.view_month.checked_sub_months(Months::new(1)).unwrap();
        }
        if next_month {
            self.view_month = self.view_month.checked_add_months(Months::new(1)).unwrap();
        }
        if go_today {
            // Recenter the calendar on the current month (stay here).
            self.view_month = today.with_day(1).unwrap();
        }
        if go_back {
            self.page = Page::Tracker;
        }

        // Load this month's per-day totals for the heatmap (only when it changes).
        if self.loaded_month != Some(self.view_month) {
            let prefix = self.view_month.format("%Y-%m").to_string();
            self.month_totals = db::or_warn(
                db::totals_for_month(&self.conn, &prefix),
                "load monthly totals",
            );
            self.loaded_month = Some(self.view_month);
        }

        // BODY — reserve a small strip at the bottom for the legend.
        let avail_size = ui.available_size();
        let cell_size = egui::Vec2::new(avail_size.x / 8.0, (avail_size.y - 40.0).max(24.0) / 6.0);

        ui.vertical_centered(|ui| {
            let spacing_x = ui.spacing().item_spacing.x;
            let grid_width = cell_size.x * 7.0 + spacing_x * 6.0;

            ui.allocate_ui_with_layout(
                egui::vec2(grid_width, 0.0),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    egui::Grid::new("calendar").show(ui, |grid_ui| {
                        for name in ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"] {
                            grid_ui.label(name);
                        }
                        grid_ui.end_row();

                        let first_weekday = self.view_month.weekday().num_days_from_monday();
                        let year = self.view_month.year();
                        let month = self.view_month.month();

                        let mut col = 0;

                        for _ in 0..first_weekday {
                            grid_ui.label("");
                            col += 1;
                        }

                        for day in 1..=31 {
                            let date = match NaiveDate::from_ymd_opt(year, month, day) {
                                Some(date) => date,
                                None => break,
                            };

                            let is_today = today == date;
                            let is_future = today < date;
                            let secs = self
                                .month_totals
                                .get(&db::date_key(date))
                                .copied()
                                .unwrap_or(0);
                            let (fill, text_color) = heat_cell(secs);

                            let mut button = egui::Button::new(
                                egui::RichText::new(day.to_string()).color(text_color),
                            )
                            .min_size(cell_size)
                            .corner_radius(6.0)
                            .fill(fill);
                            // Keep today distinct with a gold outline.
                            if is_today {
                                button = button.stroke(egui::Stroke::new(2.0, GOLD));
                            }

                            let hover = if secs == 0 {
                                format!("{} — no activity", date.format("%d %b"))
                            } else {
                                format!("{} — {} active", date.format("%d %b"), fmt_duration(secs))
                            };

                            if grid_ui
                                .add_enabled(!is_future, button)
                                .on_hover_text(hover)
                                .clicked()
                            {
                                self.view_date(date, today);
                                self.page = Page::Tracker;
                            }

                            col += 1;
                            if col % 7 == 0 {
                                grid_ui.end_row();
                            }
                        }
                    });
                },
            );
        });

        // Heatmap legend, pinned to the bottom-right (bottom_up + Align::Max
        // anchors it to the bottom edge and the right of the remaining space).
        ui.with_layout(egui::Layout::bottom_up(egui::Align::Max), |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.label(egui::RichText::new("Less").size(11.0).color(MUTED));
                for sample in HEAT_SAMPLES {
                    let (fill, _) = heat_cell(sample);
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(13.0, 13.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, fill);
                }
                ui.label(egui::RichText::new("More").size(11.0).color(MUTED));
            });
        });
    }
}
