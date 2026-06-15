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

// A compact icon-only square button, shared by the toolbar nav controls.
pub(crate) fn icon_button(ui: &mut Ui, icon: &str, hover: &str, enabled: bool) -> bool {
    let btn = egui::Button::new(egui::RichText::new(icon).size(16.0))
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
        let btn = egui::Button::new(egui::RichText::new(icon).size(16.0).color(fg))
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

// One app row: chevron + dot + name, right-aligned time and percent, and a bar.
// The whole row is clickable to toggle the drill-down; returns true when clicked.
pub(crate) fn app_row(
    ui: &mut Ui,
    name: &str,
    secs: u64,
    total: u64,
    bar_max: u64,
    color: Color32,
    expanded: bool,
) -> bool {
    let frac = frac_of(secs, bar_max);
    let pct = if total > 0 {
        (secs as f64 / total as f64 * 100.0).round() as u64
    } else {
        0
    };

    let chevron = if expanded {
        regular::CARET_DOWN
    } else {
        regular::CARET_RIGHT
    };

    let response = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(chevron).size(11.0).color(MUTED));
                color_dot(ui, color, 4.5);
                ui.label(egui::RichText::new(name).size(13.5).color(TEXT));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        egui::RichText::new(format!("{pct}%"))
                            .size(12.0)
                            .color(MUTED),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(fmt_duration(secs))
                            .size(13.0)
                            .color(TEXT),
                    );
                });
            });
            usage_bar(ui, frac, color);
        })
        .response
        .interact(egui::Sense::click())
        .on_hover_cursor(egui::CursorIcon::PointingHand);

    response.clicked()
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
                ui.add(
                    egui::Label::new(egui::RichText::new(shown).size(12.0).color(MUTED)).truncate(),
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
