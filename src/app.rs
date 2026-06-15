//! The eframe app: window/mini-HUD state and per-frame page dispatch.

use crate::config::Config;
use crate::db;
use crate::state::{SharedState, UsageRow};
use chrono::{Datelike, Local, NaiveDate};
use eframe::egui::Ui;
use eframe::{Frame, egui};
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::time::Duration;

#[derive(PartialEq)]
pub(crate) enum Page {
    Tracker,
    Calendar,
}

pub struct TrackerApp {
    // Shared with the background tracker thread.
    pub(crate) state: SharedState,
    pub(crate) conn: Connection,
    pub(crate) page: Page,
    pub(crate) view_month: NaiveDate,
    // Date being viewed; None means the live "today" view.
    pub(crate) selected_date: Option<NaiveDate>,
    pub(crate) day_usage: Vec<UsageRow>,
    // Calendar heatmap: cached per-day totals for the loaded month.
    pub(crate) month_totals: HashMap<String, u64>,
    pub(crate) loaded_month: Option<NaiveDate>,
    // Drill-down: the expanded app row and (for past days) its cached titles.
    pub(crate) expanded_app: Option<String>,
    pub(crate) expanded_titles: Vec<UsageRow>,
    // Mini-HUD: whether collapsed, plus full-window geometry to restore on expand.
    pub(crate) is_mini: bool,
    pub(crate) full_size: egui::Vec2,
    pub(crate) full_pos: Option<egui::Pos2>,
    pub(crate) hud_pos: Option<egui::Pos2>,
    pub(crate) config: Config,
}

impl TrackerApp {
    pub fn new(state: SharedState, conn: Connection, config: Config) -> Self {
        Self {
            state,
            conn,
            page: Page::Tracker,
            view_month: Local::now().date_naive().with_day(1).unwrap(),
            selected_date: None,
            day_usage: Vec::new(),
            month_totals: HashMap::new(),
            loaded_month: None,
            expanded_app: None,
            expanded_titles: Vec::new(),
            is_mini: false,
            full_size: egui::vec2(800.0, 640.0), // refreshed from the real window each frame
            full_pos: None,
            hud_pos: None,
            config,
        }
    }

    // Switch the tracker view to a given date. Selecting today (or later) drops
    // back to the live in-memory view; any past day is loaded from the database.
    pub(crate) fn view_date(&mut self, date: NaiveDate, today: NaiveDate) {
        // Any open drill-down belongs to the previous day's data.
        self.expanded_app = None;
        self.expanded_titles.clear();

        if date >= today {
            self.selected_date = None;
        } else {
            self.selected_date = Some(date);
            self.day_usage = db::or_warn(
                db::usage_for_date(&self.conn, &db::date_key(date)),
                "load day usage",
            );
        }
    }

    // Per-title breakdown for an app: live from the in-memory map today, or the
    // cached past-day rows. Sorted by time, descending.
    pub(crate) fn titles_for(&self, app: &str) -> Vec<UsageRow> {
        let mut titles: Vec<UsageRow> = if self.selected_date.is_none() {
            let map = self.state.usage.lock().unwrap();
            map.iter()
                .filter(|(key, _)| key.app == app)
                .map(|(key, secs)| UsageRow::new(key.title.clone(), *secs))
                .collect()
        } else {
            self.expanded_titles.clone()
        };
        titles.sort_by(UsageRow::by_usage_desc);
        titles
    }
}

impl eframe::App for TrackerApp {
    // Flush today's usage on a graceful close, so the seconds since the last
    // periodic save aren't lost.
    fn on_exit(&mut self) {
        let today = db::date_key(Local::now().date_naive());
        db::or_warn(
            db::flush_usage(&self.conn, &today, &self.state.usage),
            "flush usage on exit",
        );
    }

    fn ui(&mut self, ui: &mut Ui, _frame: &mut Frame) {
        let ctx = ui.ctx().clone();

        // Track the full-window size and position while expanded, so we can
        // restore both when leaving the mini-HUD.
        if !self.is_mini {
            let (inner, outer) = ctx.input(|i| (i.viewport().inner_rect, i.viewport().outer_rect));
            if let Some(rect) = inner {
                self.full_size = rect.size();
            }
            if let Some(rect) = outer {
                self.full_pos = Some(rect.min);
            }
        }

        // The minimize watcher (in main) raises this when the window is
        // minimized; collapse to the mini-HUD instead. (Close is left native, so
        // the X really quits.)
        if !self.is_mini && self.state.wants_mini.swap(false, Ordering::Relaxed) {
            self.enter_mini(&ctx);
        }

        if self.is_mini {
            self.mini_hud(ui, &ctx);
            ctx.request_repaint_after(Duration::from_secs(self.config.refresh_rate_secs));
            return;
        }

        egui::Frame::new()
            .inner_margin(egui::Margin::same(16))
            .show(ui, |ui| match self.page {
                Page::Tracker => self.tracker_page(ui),
                Page::Calendar => self.calendar_page(ui),
            });

        ctx.request_repaint_after(Duration::from_secs(self.config.refresh_rate_secs));
    }
}
