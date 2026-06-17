//! The sessions manager page: create, rename, archive, and restore sessions.

use super::theme::*;
use super::widgets::{icon_button, text_button};
use crate::app::TrackerApp;
use crate::db;
use crate::state::{DEFAULT_SESSION_ID, SessionId};
use eframe::egui::{self, Ui};
use egui_phosphor::regular;
use std::sync::atomic::Ordering;

const ICONS: &str = "icons";
const MEDIUM: &str = "medium";

impl TrackerApp {
    pub(crate) fn sessions_page(&mut self, ui: &mut Ui) {
        let active = self.state.current_session.load(Ordering::Relaxed);
        let renaming = self.renaming;
        // Work off a clone so the row closures can edit `self` (rename buffer,
        // focus) without also holding a borrow of the list they're iterating.
        let sessions = self.manage_sessions.clone();
        let (live, archived): (Vec<db::Session>, Vec<db::Session>) =
            sessions.into_iter().partition(|s| !s.archived);

        // Actions are collected here and applied after the UI closures return.
        let mut switch_to: Option<SessionId> = None;
        let mut start_rename: Option<SessionId> = None;
        let mut commit_rename = false;
        let mut cancel_rename = false;
        let mut archive_id: Option<SessionId> = None;
        let mut restore_id: Option<SessionId> = None;
        let mut open_new = false;

        // ---------- Header: title + New session ----------
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(medium(egui::RichText::new("Sessions").size(18.0)).color(TEXT));
                ui.label(
                    egui::RichText::new("Switch, rename, or archive your tracking contexts.")
                        .size(12.5)
                        .color(FAINT),
                );
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = egui::Frame::new()
                    .fill(ACCENT_TINT)
                    .stroke(egui::Stroke::new(1.0, ACCENT))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::symmetric(13, 7))
                    .show(ui, |ui| {
                        // This pill sits in a right-to-left slot, which reverses
                        // its inner row — so emit label then icon to read "＋ New
                        // session" on screen.
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.label(egui::RichText::new("New session").size(13.0).color(ACCENT));
                            ui.label(glyph(regular::PLUS).size(13.0).color(ACCENT));
                        });
                    })
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if btn.clicked() {
                    open_new = true;
                }
            });
        });

        ui.add_space(16.0);

        // ---------- Session list ----------
        egui::ScrollArea::vertical()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 5.0;

                for s in &live {
                    let id = s.id;
                    let is_active = id == active;
                    let is_default = id == DEFAULT_SESSION_ID;
                    let is_renaming = renaming == Some(id);

                    // The whole row is the click target for switching. It's
                    // allocated first so the action buttons (added afterwards via
                    // a child UI) layer on top and keep their own clicks.
                    let (rect, row_resp) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 44.0),
                        egui::Sense::click(),
                    );
                    let hovered = ui.rect_contains_pointer(rect);
                    let bg = if is_active {
                        ACCENT_TINT
                    } else if hovered && !is_renaming {
                        HOVER_BG
                    } else {
                        CARD
                    };
                    ui.painter().rect_filled(rect, egui::CornerRadius::same(9), bg);

                    // Folder icon, painted at the exact vertical centre.
                    ui.painter().text(
                        egui::pos2(rect.left() + 14.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        regular::FOLDER,
                        egui::FontId::new(16.0, egui::FontFamily::Name(ICONS.into())),
                        if is_active { ACCENT } else { MUTED },
                    );
                    let text_x = rect.left() + 39.0;

                    // Right-aligned action widgets, in a child UI layered above the
                    // row's click target.
                    let ctrl_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left() + 12.0, rect.top()),
                        egui::pos2(rect.right() - 12.0, rect.bottom()),
                    );
                    ui.scope_builder(
                        egui::UiBuilder::new()
                            .max_rect(ctrl_rect)
                            .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        |ui| {
                            if is_renaming {
                                if icon_button(ui, regular::X, "Cancel", true) {
                                    cancel_rename = true;
                                }
                                if icon_button(ui, regular::CHECK, "Save", true) {
                                    commit_rename = true;
                                }
                            } else {
                                // The active session and General can't be archived
                                // (switch away first / it's the permanent fallback).
                                if !is_active
                                    && !is_default
                                    && icon_button(ui, regular::ARCHIVE, "Archive session", true)
                                {
                                    archive_id = Some(id);
                                }
                                if icon_button(ui, regular::PENCIL_SIMPLE, "Rename session", true) {
                                    start_rename = Some(id);
                                }
                            }
                        },
                    );

                    if is_renaming {
                        // Inline edit field, vertically centred at the name slot.
                        let edit_rect = egui::Rect::from_min_max(
                            egui::pos2(text_x, rect.center().y - 13.0),
                            egui::pos2(text_x + 210.0, rect.center().y + 13.0),
                        );
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(edit_rect)
                                .layout(egui::Layout::left_to_right(egui::Align::Center)),
                            |ui| {
                                let edit = ui.add(
                                    egui::TextEdit::singleline(&mut self.rename_buf)
                                        .desired_width(200.0)
                                        .margin(egui::Margin::symmetric(8, 4)),
                                );
                                if self.focus_rename {
                                    edit.request_focus();
                                    self.focus_rename = false;
                                }
                                if edit.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                {
                                    commit_rename = true;
                                }
                                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                                    cancel_rename = true;
                                }
                            },
                        );
                    } else {
                        // Name, painted at the exact vertical centre to sit level
                        // with the folder icon.
                        let name_font = if is_active {
                            egui::FontId::new(14.0, egui::FontFamily::Name(MEDIUM.into()))
                        } else {
                            egui::FontId::proportional(14.0)
                        };
                        let galley =
                            ui.painter().layout_no_wrap(s.name.clone(), name_font, TEXT);
                        let name_w = galley.size().x;
                        ui.painter().galley(
                            egui::pos2(text_x, rect.center().y - galley.size().y / 2.0),
                            galley,
                            TEXT,
                        );

                        // Active pill / default tag, just after the name.
                        let tag_x = text_x + name_w + 9.0;
                        if is_active {
                            paint_pill(ui, tag_x, rect.center().y);
                        } else if is_default {
                            ui.painter().text(
                                egui::pos2(tag_x, rect.center().y),
                                egui::Align2::LEFT_CENTER,
                                "default",
                                egui::FontId::proportional(11.0),
                                FAINT,
                            );
                        }
                    }

                    if row_resp.clicked() && !is_renaming {
                        switch_to = Some(id);
                    }
                    if hovered && !is_active && !is_renaming {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }

                // ---------- Archived ----------
                if !archived.is_empty() {
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.label(egui::RichText::new("ARCHIVED").size(11.0).color(FAINT));
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 1.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().hline(
                            rect.x_range(),
                            rect.center().y,
                            egui::Stroke::new(1.0, HAIRLINE),
                        );
                    });
                    ui.add_space(4.0);

                    for s in &archived {
                        let id = s.id;
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 38.0),
                            egui::Sense::hover(),
                        );
                        ui.painter().text(
                            egui::pos2(rect.left() + 14.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            regular::FOLDER,
                            egui::FontId::new(15.0, egui::FontFamily::Name(ICONS.into())),
                            FAINT,
                        );
                        let galley = ui.painter().layout_no_wrap(
                            s.name.clone(),
                            egui::FontId::proportional(13.5),
                            MUTED,
                        );
                        ui.painter().galley(
                            egui::pos2(rect.left() + 39.0, rect.center().y - galley.size().y / 2.0),
                            galley,
                            MUTED,
                        );
                        let ctrl_rect = egui::Rect::from_min_max(
                            egui::pos2(rect.left() + 12.0, rect.top()),
                            egui::pos2(rect.right() - 12.0, rect.bottom()),
                        );
                        ui.scope_builder(
                            egui::UiBuilder::new()
                                .max_rect(ctrl_rect)
                                .layout(egui::Layout::right_to_left(egui::Align::Center)),
                            |ui| {
                                if text_button(ui, "Restore", "Restore session", 78.0) {
                                    restore_id = Some(id);
                                }
                            },
                        );
                    }
                }
            });

        // ---------- Apply collected actions ----------
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
        if let Some(id) = start_rename {
            self.rename_buf = self
                .manage_sessions
                .iter()
                .find(|s| s.id == id)
                .map(|s| s.name.clone())
                .unwrap_or_default();
            self.renaming = Some(id);
            self.focus_rename = true;
        }
        if commit_rename {
            if let Some(id) = self.renaming {
                let name = self.rename_buf.trim().to_string();
                // A blank name is a no-op; keep the old one.
                if !name.is_empty() {
                    db::or_warn(db::rename_session(&self.conn, id, &name), "rename session");
                    self.refresh_sessions();
                    self.refresh_manage_sessions();
                }
            }
            self.renaming = None;
        }
        if cancel_rename {
            self.renaming = None;
        }
        if let Some(id) = archive_id {
            db::or_warn(db::set_archived(&self.conn, id, true), "archive session");
            if self.renaming == Some(id) {
                self.renaming = None;
            }
            self.refresh_sessions();
            self.refresh_manage_sessions();
        }
        if let Some(id) = restore_id {
            db::or_warn(db::set_archived(&self.conn, id, false), "restore session");
            self.refresh_sessions();
            self.refresh_manage_sessions();
        }
        if open_new {
            self.show_new_session = true;
            self.new_session_name.clear();
            self.focus_new_session = true;
        }

        self.new_session_popup(ui);
    }
}

// The amber "Active" pill marking the session being recorded into, painted so it
// sits level with the row's name.
fn paint_pill(ui: &Ui, x: f32, cy: f32) {
    let galley = ui.painter().layout_no_wrap(
        "Active".to_string(),
        egui::FontId::new(10.5, egui::FontFamily::Name(MEDIUM.into())),
        PILL_TEXT,
    );
    let rect = egui::Rect::from_min_size(
        egui::pos2(x, cy - 9.0),
        egui::vec2(galley.size().x + 14.0, 18.0),
    );
    ui.painter().rect_filled(rect, egui::CornerRadius::same(5), ACCENT);
    ui.painter().galley(
        egui::pos2(rect.left() + 7.0, rect.center().y - galley.size().y / 2.0),
        galley,
        PILL_TEXT,
    );
}
