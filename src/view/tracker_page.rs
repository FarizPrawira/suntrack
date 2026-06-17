//! The dashboard page: top bar, hero summary, and the per-app list with drill-down.

use super::theme::*;
use super::widgets::{
    Step, accent_button, app_row, date_stepper, ghost_button, idle_row, segmented, stat_chip,
    title_row, tracking_button,
};
use crate::app::{Page, TrackerApp};
use crate::db;
use crate::state::{IDLE, SessionId, UsageRow};
use chrono::Local;
use eframe::egui::{self, Ui};
use egui_phosphor::regular;
use std::sync::atomic::Ordering;

impl TrackerApp {
    pub(crate) fn tracker_page(&mut self, ui: &mut Ui) {
        let today = Local::now().date_naive();

        // ---------- Top bar (may change the active session / scope / date) ----------
        self.dashboard_top_bar(ui, today);

        ui.add_space(10.0);

        // ---------- Gather + split data ----------
        let is_live = self.selected_date.is_none();
        let viewed = self.selected_date.unwrap_or(today);
        let active = self.state.current_session.load(Ordering::Relaxed);
        let is_tracking = self.state.tracking.load(Ordering::Relaxed);
        // The live in-memory map is used only for the active session today; every
        // other view (a past day, or "All sessions") reads from the database.
        let live = self.is_live_view();

        if !live {
            let key = (self.selected_date, self.view_all, active);
            let key_changed = self.loaded_usage_key != Some(key);
            if key_changed && is_live {
                // Today's all-sessions view: flush the active session's live tail
                // so it's reflected in the combined read.
                db::or_warn(
                    db::flush_usage(&self.conn, &db::date_key(today), active, &self.state.usage),
                    "flush before combined view",
                );
            }
            if key_changed || is_live {
                self.day_usage = db::or_warn(
                    db::usage_for_date(&self.conn, &db::date_key(viewed), self.view_filter()),
                    "load day usage",
                );
                self.loaded_usage_key = Some(key);
            }
            if key_changed {
                self.expanded_app = None;
                self.expanded_titles.clear();
            }
        }

        let snapshot: Vec<UsageRow> = if live {
            self.state.app_totals()
        } else {
            self.day_usage.clone()
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
            ui.add_space(70.0);
            let mut start = false;
            ui.vertical_centered(|ui| {
                if !is_live {
                    ui.label(medium(egui::RichText::new("No activity tracked").size(15.0)).color(TEXT));
                    ui.label(
                        egui::RichText::new("Nothing was recorded on this day.")
                            .size(12.5)
                            .color(FAINT),
                    );
                } else if is_tracking {
                    ui.label(medium(egui::RichText::new("No activity yet").size(15.0)).color(TEXT));
                    ui.label(
                        egui::RichText::new("Tracking is on — it'll appear as you switch apps.")
                            .size(12.5)
                            .color(FAINT),
                    );
                } else {
                    // First open / paused: an explicit invitation to start.
                    ui.label(
                        medium(egui::RichText::new("Ready to track your day").size(16.0)).color(TEXT),
                    );
                    ui.label(
                        egui::RichText::new("Suntrack records where your time goes, automatically.")
                            .size(12.5)
                            .color(FAINT),
                    );
                    ui.add_space(16.0);
                    start = tracking_button(ui, false, true);
                }
            });
            if start {
                self.state.tracking.store(true, Ordering::Relaxed);
            }
            return;
        }

        // ---------- Hero summary ----------
        let mut toggle = false;
        ui.horizontal(|ui| {
            ui.label(
                medium(egui::RichText::new(fmt_duration(total_active)).size(40.0))
                    .color(egui::Color32::WHITE),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(if is_live { "active today" } else { "active" })
                    .size(14.0)
                    .color(FAINT),
            );
            // Primary control, where the eye lands — only for the live "today" view.
            if is_live {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if tracking_button(ui, is_tracking, false) {
                        toggle = true;
                    }
                });
            }
        });
        if toggle {
            self.state.tracking.store(!is_tracking, Ordering::Relaxed);
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(top) = apps.first() {
                stat_chip(ui, regular::TROPHY, &top.name, true);
            }
            if idle_secs > 0 {
                stat_chip(ui, regular::COFFEE, &format!("{} idle", fmt_duration(idle_secs)), false);
            }
            stat_chip(ui, regular::SQUARES_FOUR, &format!("{} apps", apps.len()), false);
        });

        ui.add_space(16.0);

        // ---------- App list (scrollable; click a row to drill down) ----------
        let load_from_db = !live;
        let viewed_key = db::date_key(viewed);
        let filter = self.view_filter();
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                for row in &apps {
                    let color = app_color(&row.name);
                    let expanded = self.expanded_app.as_deref() == Some(row.name.as_str());

                    if app_row(ui, &row.name, row.secs, total_active, bar_max, color, expanded) {
                        if expanded {
                            self.expanded_app = None;
                        } else {
                            self.expanded_app = Some(row.name.clone());
                            if load_from_db {
                                self.expanded_titles = db::or_warn(
                                    db::titles_for(&self.conn, &viewed_key, &row.name, filter),
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
                    idle_row(ui, idle_secs, bar_max);
                }
            });
    }

    // The dashboard top bar: session picker + view-scope segmented on the left,
    // grouped date stepper on the right. (Pause/resume lives in the sidebar.)
    fn dashboard_top_bar(&mut self, ui: &mut Ui, today: chrono::NaiveDate) {
        let active = self.state.current_session.load(Ordering::Relaxed);
        let active_name = self
            .sessions
            .iter()
            .find(|s| s.id == active)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "General".to_string());
        let is_live = self.selected_date.is_none();
        let viewed = self.selected_date.unwrap_or(today);

        let mut switch_to: Option<SessionId> = None;
        let mut open_new = false;
        let mut new_view_all = self.view_all;
        let mut step = Step::None;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;

            // Active-session picker: a custom pill matching the segmented and
            // stepper, with a menu popup (switch into another, or create one).
            let pill = egui::Frame::new()
                .fill(CARD)
                .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                .corner_radius(9.0)
                .inner_margin(egui::Margin::symmetric(11, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.label(glyph(regular::FOLDER).size(13.0).color(MUTED));
                        ui.label(egui::RichText::new(&active_name).size(13.0).color(TEXT));
                        ui.label(glyph(regular::CARET_DOWN).size(13.0).color(MUTED));
                    });
                })
                .response
                .interact(egui::Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);

            egui::Popup::menu(&pill).show(|ui| {
                for s in &self.sessions {
                    if ui
                        .selectable_label(s.id == active, s.name.as_str())
                        .clicked()
                    {
                        switch_to = Some(s.id);
                        ui.close();
                    }
                }
                ui.separator();
                if ui
                    .button(format!("{}  New session…", regular::PLUS))
                    .clicked()
                {
                    open_new = true;
                    ui.close();
                }
            });

            // View scope.
            new_view_all = segmented(ui, "This session", "All", self.view_all);

            // Grouped date stepper, right-aligned.
            let label = if is_live {
                "Today".to_string()
            } else {
                viewed.format("%a, %d %b").to_string()
            };
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                step = date_stepper(ui, &label, !is_live);
            });
        });

        // Apply the bar's actions after its closure, so they can mutate `self`.
        if new_view_all != self.view_all {
            self.view_all = new_view_all;
        }
        match step {
            Step::Prev => {
                if let Some(d) = viewed.pred_opt() {
                    self.view_date(d, today);
                }
            }
            Step::Next if !is_live => {
                if let Some(d) = viewed.succ_opt() {
                    self.view_date(d, today);
                }
            }
            Step::Open => {
                self.loaded_month = None;
                self.page = Page::Calendar;
            }
            _ => {}
        }
        if let Some(id) = switch_to
            && id != active
        {
            self.set_active_session(id);
            // Show the live "today" view of the session you switched into.
            self.selected_date = None;
            self.view_all = false;
            self.expanded_app = None;
            self.expanded_titles.clear();
        }
        if open_new {
            self.show_new_session = true;
            self.new_session_name.clear();
            self.focus_new_session = true;
        }

        self.new_session_popup(ui);
    }

    // The modal for naming and creating a session. Creating one makes it active
    // and drops back to its live view. Shared by the dashboard top bar and the
    // sessions manager page.
    pub(crate) fn new_session_popup(&mut self, ui: &mut Ui) {
        if !self.show_new_session {
            return;
        }
        let mut create = false;
        let mut cancel = false;

        egui::Window::new("New session")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .frame(
                egui::Frame::new()
                    .fill(CARD)
                    .stroke(egui::Stroke::new(1.0, CARD_BORDER))
                    .corner_radius(12.0)
                    .inner_margin(egui::Margin::same(16)),
            )
            .show(ui.ctx(), |ui| {
                ui.set_width(264.0);
                ui.label(medium(egui::RichText::new("New session").size(14.0)).color(TEXT));
                ui.add_space(12.0);
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.new_session_name)
                        .hint_text("e.g. Work, Learning")
                        .margin(egui::Margin::symmetric(10, 8))
                        .desired_width(f32::INFINITY),
                );
                // Focus the field the first frame the popup opens.
                if self.focus_new_session {
                    edit.request_focus();
                    self.focus_new_session = false;
                }
                // Enter in a single-line field surrenders focus on submit.
                let submitted = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if accent_button(ui, "Create") || submitted {
                        create = true;
                    }
                    if ghost_button(ui, "Cancel") {
                        cancel = true;
                    }
                });
            });

        if create {
            let name = self.new_session_name.trim().to_string();
            // A blank name keeps the popup open for another try.
            if !name.is_empty() {
                match db::create_session(&self.conn, &name) {
                    Ok(id) => {
                        self.refresh_sessions();
                        self.refresh_manage_sessions();
                        self.set_active_session(id);
                        self.view_all = false;
                        self.selected_date = None;
                        self.expanded_app = None;
                        self.expanded_titles.clear();
                    }
                    Err(err) => eprintln!("suntrack: could not create session '{name}': {err}"),
                }
                self.show_new_session = false;
                self.new_session_name.clear();
            }
        }
        if cancel {
            self.show_new_session = false;
            self.new_session_name.clear();
        }
    }
}
