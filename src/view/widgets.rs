//! Reusable egui widgets for the dashboard rows and toolbar.

use super::theme::*;
use crate::state::IDLE;
use eframe::egui::{self, Color32, Ui};
use egui_phosphor::regular;

// Bar fill fraction in 0..=1, guarding divide-by-zero. Shared by every row.
fn frac_of(secs: u64, max: u64) -> f32 {
    if max > 0 {
        secs as f32 / max as f32
    } else {
        0.0
    }
}

// A small filled circle that participates in horizontal layout.
pub(crate) fn color_dot(ui: &mut Ui, color: Color32, radius: f32) {
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(radius * 2.0, radius * 2.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

// A full-width proportional bar: a muted track with a colored fill. The one
// place bar geometry lives, shared by the app rows and the drill-down titles.
pub(crate) fn fill_bar(ui: &mut Ui, frac: f32, color: Color32, height: f32, radius: f32) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), height),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    painter.rect_filled(rect, radius, TRACK_BG);
    let mut fill = rect;
    fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
    painter.rect_filled(fill, radius, color);
}

// The standard app-row bar.
pub(crate) fn usage_bar(ui: &mut Ui, frac: f32, color: Color32) {
    fill_bar(ui, frac, color, 7.0, 4.0);
}

// A full-width sidebar navigation row: icon + label, amber-tinted when active.
// Returns true when clicked.
pub(crate) fn nav_item(ui: &mut Ui, icon: &str, label: &str, active: bool) -> bool {
    let fg = if active { ACCENT } else { MUTED };
    egui::Frame::new()
        .fill(if active { ACCENT_TINT } else { Color32::TRANSPARENT })
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(10, 9))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(glyph(icon).size(17.0).color(fg));
                ui.add_space(5.0);
                ui.label(egui::RichText::new(label).size(13.5).color(fg));
            });
            // Force the row (and thus its click target) to span the full width.
            ui.allocate_space(egui::vec2(ui.available_width(), 0.0));
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

// A compact icon-only square button, shared by the toolbar nav controls.
pub(crate) fn icon_button(ui: &mut Ui, icon: &str, hover: &str, enabled: bool) -> bool {
    let btn = egui::Button::new(glyph(icon).size(16.0))
        .min_size(egui::vec2(CONTROL_H, CONTROL_H))
        .corner_radius(BTN_RADIUS);
    ui.add_enabled(enabled, btn).on_hover_text(hover).clicked()
}

// A labelled toolbar button matching icon_button's height and rounding.
pub(crate) fn text_button(ui: &mut Ui, label: &str, hover: &str, width: f32) -> bool {
    let btn = egui::Button::new(egui::RichText::new(label).size(13.0))
        .min_size(egui::vec2(width, CONTROL_H))
        .corner_radius(BTN_RADIUS);
    ui.add(btn).on_hover_text(hover).clicked()
}

// The play/pause toggle, shared by the header and the HUD. `compact` is the
// icon-only square the HUD uses; otherwise the labelled "Pause"/"Resume" button.
// Returns true when clicked — the caller flips the tracking flag.
pub(crate) fn toggle_button(ui: &mut Ui, is_tracking: bool, compact: bool) -> bool {
    let (icon, fill, fg) = if is_tracking {
        (regular::PAUSE, ACCENT_BG, ACCENT_FG)
    } else {
        (regular::PLAY, BTN_BG, TEXT)
    };

    if compact {
        let btn = egui::Button::new(glyph(icon).size(16.0).color(fg))
            .fill(fill)
            .corner_radius(9.0)
            .min_size(egui::vec2(38.0, 38.0));
        ui.add(btn).clicked()
    } else {
        let (label, hover) = if is_tracking {
            ("Pause", "Pause tracking")
        } else {
            ("Resume", "Resume tracking")
        };
        let btn = egui::Button::new(
            egui::RichText::new(format!("{icon}  {label}"))
                .size(14.0)
                .color(fg),
        )
        .fill(fill)
        .corner_radius(BTN_RADIUS)
        .min_size(egui::vec2(104.0, CONTROL_H));
        ui.add(btn).on_hover_text(hover).clicked()
    }
}

// One app row: an avatar, the app name, and a proportional background fill that
// reads as the bar, with right-aligned percent and time. The whole row is
// clickable to toggle the drill-down; returns true when clicked.
pub(crate) fn app_row(
    ui: &mut Ui,
    name: &str,
    secs: u64,
    total: u64,
    bar_max: u64,
    color: Color32,
    expanded: bool,
) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
    let hovered = resp.hovered();
    let frac = frac_of(secs, bar_max);
    let pct = if total > 0 {
        (secs as f64 / total as f64 * 100.0).round() as u64
    } else {
        0
    };

    {
        let painter = ui.painter();
        let radius = egui::CornerRadius::same(9);

        // Hover/expanded base, then the proportional fill that reads as the bar.
        if hovered || expanded {
            painter.rect_filled(rect, radius, HOVER_BG);
        }
        if frac > 0.0 {
            let mut fill = rect;
            fill.set_width(rect.width() * frac.clamp(0.0, 1.0));
            painter.rect_filled(fill, radius, tint(color, 30));
        }

        // Avatar: a rounded square tinted in the app color, with its initial.
        let av = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 25.0, rect.center().y),
            egui::vec2(28.0, 28.0),
        );
        painter.rect_filled(av, egui::CornerRadius::same(7), tint(color, 48));
        let initial = name
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string());
        painter.text(
            av.center(),
            egui::Align2::CENTER_CENTER,
            initial,
            egui::FontId::new(13.0, egui::FontFamily::Name("medium".into())),
            color,
        );

        // Name.
        painter.text(
            egui::pos2(av.right() + 10.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            name,
            egui::FontId::proportional(14.0),
            TEXT,
        );

        // Time at the right edge, with the percent to its left.
        let time_rect = painter.text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            fmt_duration(secs),
            egui::FontId::new(13.5, egui::FontFamily::Name("medium".into())),
            TEXT,
        );
        painter.text(
            egui::pos2(time_rect.left() - 14.0, rect.center().y),
            egui::Align2::RIGHT_CENTER,
            format!("{pct}%"),
            egui::FontId::proportional(12.5),
            FAINT,
        );
    }

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

// A translucent version of `color`, for fills painted behind content.
fn tint(color: Color32, alpha: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

// A small rounded stat chip: icon + text, amber-tinted when `accent`.
pub(crate) fn stat_chip(ui: &mut Ui, icon: &str, text: &str, accent: bool) {
    let (bg, icon_col, txt_col) = if accent {
        (ACCENT_TINT, ACCENT, Color32::from_rgb(0xe8, 0xc9, 0x87))
    } else {
        (CARD, MUTED, MUTED)
    };
    egui::Frame::new()
        .fill(bg)
        .corner_radius(7.0)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.label(glyph(icon).size(13.0).color(icon_col));
                ui.label(egui::RichText::new(text).size(12.0).color(txt_col));
            });
        });
}

// A two-option segmented control. Returns the (possibly changed) right-selected
// state; the active segment is filled.
pub(crate) fn segmented(ui: &mut Ui, left: &str, right: &str, right_selected: bool) -> bool {
    let mut selected = right_selected;
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::same(3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if segment(ui, left, !right_selected) {
                    selected = false;
                }
                if segment(ui, right, right_selected) {
                    selected = true;
                }
            });
        });
    selected
}

fn segment(ui: &mut Ui, text: &str, active: bool) -> bool {
    egui::Frame::new()
        .fill(if active { HOVER_BG } else { Color32::TRANSPARENT })
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(text)
                    .size(12.0)
                    .color(if active { TEXT } else { MUTED }),
            );
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

// What a date stepper click resolved to.
pub(crate) enum Step {
    Prev,
    Next,
    Open,
    None,
}

// A grouped date stepper pill — ‹ [date] › — where the date opens the calendar.
// `can_next` greys the forward arrow on the live "today" view.
pub(crate) fn date_stepper(ui: &mut Ui, label: &str, can_next: bool) -> Step {
    let mut action = Step::None;
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                // This pill sits in a right-aligned (right-to-left) slot, which
                // reverses child order — so emit forward-arrow → date → back-arrow
                // to read "‹ date ›" on screen.
                if step_icon(ui, regular::CARET_RIGHT, can_next) {
                    action = Step::Next;
                }
                // Fixed-width, centered date so the pill never resizes (and the
                // arrows never shift under the cursor) as the label changes.
                let date = ui
                    .add_sized(
                        egui::vec2(118.0, 18.0),
                        egui::Label::new(
                            egui::RichText::new(format!("{}  {label}", regular::CALENDAR))
                                .size(13.0)
                                .color(TEXT),
                        )
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                if date.clicked() {
                    action = Step::Open;
                }
                if step_icon(ui, regular::CARET_LEFT, true) {
                    action = Step::Prev;
                }
            });
        });
    action
}

fn step_icon(ui: &mut Ui, icon: &str, enabled: bool) -> bool {
    let color = if enabled { MUTED } else { FAINT };
    let resp = ui.add(
        egui::Label::new(glyph(icon).size(14.0).color(color)).sense(egui::Sense::click()),
    );
    enabled && resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

// A grouped month stepper pill — ◀ [Month Year] ▶ — for the calendar header.
// Like `date_stepper` but the label is inert (no calendar-open) since we're
// already on the calendar. Returns Prev/Next/None; Open is never produced.
// Designed to sit in a right-aligned (right-to-left) slot, so the arrows are
// emitted in reverse to read "◀ label ▶" on screen; the label has a fixed
// width so the arrows never shift under the cursor as the month name changes.
pub(crate) fn month_stepper(ui: &mut Ui, label: &str) -> Step {
    let mut action = Step::None;
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .corner_radius(9.0)
        .inner_margin(egui::Margin::symmetric(8, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                if step_icon(ui, regular::CARET_RIGHT, true) {
                    action = Step::Next;
                }
                ui.add_sized(
                    egui::vec2(104.0, 18.0),
                    egui::Label::new(egui::RichText::new(label).size(13.0).color(TEXT)),
                );
                if step_icon(ui, regular::CARET_LEFT, true) {
                    action = Step::Prev;
                }
            });
        });
    action
}

// A filled amber primary button (used in modals).
pub(crate) fn accent_button(ui: &mut Ui, label: &str) -> bool {
    egui::Frame::new()
        .fill(ACCENT_TINT)
        .stroke(egui::Stroke::new(1.0, ACCENT))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 6))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(13.0).color(ACCENT));
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

// The tracking toggle: green "Start tracking"/"Resume" when paused, neutral
// "Pause" while recording. Content-sized; `big` enlarges it for the empty-state
// call-to-action. Returns true when clicked.
pub(crate) fn tracking_button(ui: &mut Ui, is_tracking: bool, big: bool) -> bool {
    let (icon, label, fill, fill_hover, border, fg) = if is_tracking {
        (
            regular::PAUSE,
            "Pause",
            HOVER_BG,
            Color32::from_rgb(0x33, 0x33, 0x3b),
            CARD_BORDER,
            TEXT,
        )
    } else {
        (
            regular::PLAY,
            if big { "Start tracking" } else { "Resume" },
            Color32::from_rgb(0x22, 0x3a, 0x2b),
            Color32::from_rgb(0x2a, 0x46, 0x33),
            Color32::from_rgb(0x2f, 0x66, 0x45),
            RECORDING,
        )
    };
    let (fsize, isize, hpad, vpad) = if big {
        (14.0, 16.0, 18.0, 11.0)
    } else {
        (13.0, 15.0, 13.0, 8.0)
    };
    let gap = 7.0;
    let radius = egui::CornerRadius::same(if big { 10 } else { 9 });

    let icon_font = egui::FontId::new(isize, egui::FontFamily::Name("icons".into()));
    let label_font = egui::FontId::new(fsize, egui::FontFamily::Name("medium".into()));
    let g_icon = ui.painter().layout_no_wrap(icon.to_string(), icon_font, fg);
    let g_label = ui.painter().layout_no_wrap(label.to_string(), label_font, fg);
    let (iw, ih) = (g_icon.size().x, g_icon.size().y);
    let (lw, lh) = (g_label.size().x, g_label.size().y);

    let size = egui::vec2(iw + gap + lw + hpad * 2.0, ih.max(lh) + vpad * 2.0);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let hovered = resp.hovered();
    let bg = if hovered { fill_hover } else { fill };
    let cy = rect.center().y;
    let start = rect.left() + hpad;

    let painter = ui.painter();
    painter.rect(
        rect,
        radius,
        bg,
        egui::Stroke::new(1.0, border),
        egui::StrokeKind::Inside,
    );
    painter.galley(egui::pos2(start, cy - ih / 2.0), g_icon, fg);
    painter.galley(egui::pos2(start + iw + gap, cy - lh / 2.0), g_label, fg);

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp.clicked()
}

// A neutral outlined button.
pub(crate) fn ghost_button(ui: &mut Ui, label: &str) -> bool {
    egui::Frame::new()
        .fill(CARD)
        .stroke(egui::Stroke::new(1.0, CARD_BORDER))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(14, 6))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(label).size(13.0).color(MUTED));
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

// A drill-down sub-row: an indented window title with its time and a thin bar.
pub(crate) fn title_row(ui: &mut Ui, title: &str, secs: u64, max: u64, color: Color32) {
    let frac = frac_of(secs, max);
    let shown = if title.trim().is_empty() {
        "(no title)"
    } else {
        title
    };

    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 26,
            right: 10,
            top: 3,
            bottom: 3,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Give the title an explicit width (the row minus a reserve for
                // the time) so it truncates instead of running under the time.
                // A left-to-right layout keeps the text left-aligned (add_sized
                // would centre it).
                let title_w = (ui.available_width() - 56.0).max(20.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(title_w, 16.0),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(shown).size(12.0).color(MUTED))
                                .truncate(),
                        )
                        .on_hover_text(shown);
                    },
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(fmt_duration(secs))
                            .size(12.0)
                            .color(MUTED),
                    );
                });
            });
            fill_bar(ui, frac, color.gamma_multiply(0.75), 5.0, 3.0);
        });
}

// Idle time, de-emphasized and kept out of the active total.
pub(crate) fn idle_row(ui: &mut Ui, secs: u64, bar_max: u64) {
    let frac = frac_of(secs, bar_max);
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{}  {IDLE}", regular::COFFEE))
                        .size(13.0)
                        .color(MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(fmt_duration(secs))
                            .size(12.5)
                            .color(MUTED),
                    );
                });
            });
            usage_bar(ui, frac, IDLE_COLOR);
        });
}
