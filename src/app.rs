//! The eframe app: window/mini-HUD state and per-frame page dispatch.

use crate::config::Config;
use crate::db;
use crate::state::{SessionId, SharedState, UsageRow};
use crate::view::theme::{BG, SURFACE};
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
    Sessions,
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
    // Live (non-archived) sessions, cached for the picker; refreshed on change.
    pub(crate) sessions: Vec<db::Session>,
    // View scope: false shows the active session, true combines all sessions.
    pub(crate) view_all: bool,
    // Cache key for `day_usage` — the DB-backed view rows are reloaded only when
    // the (date, scope, active session) it was loaded for changes.
    pub(crate) loaded_usage_key: Option<(Option<NaiveDate>, bool, SessionId)>,
    // The "new session" popup, its in-progress name, and a one-shot flag to focus
    // the field when it opens.
    pub(crate) show_new_session: bool,
    pub(crate) new_session_name: String,
    pub(crate) focus_new_session: bool,
    // Sessions manager: the full list (incl. archived — `sessions` above stays
    // live-only for the pickers), plus inline-rename state (which row, the
    // in-progress text, and a one-shot focus flag for the edit field).
    pub(crate) manage_sessions: Vec<db::Session>,
    pub(crate) renaming: Option<SessionId>,
    pub(crate) rename_buf: String,
    pub(crate) focus_rename: bool,
    // Calendar heatmap: cached per-day totals for the loaded month, keyed by the
    // (month, session filter) it was loaded for so the scope toggle reloads it.
    pub(crate) month_totals: HashMap<String, u64>,
    pub(crate) loaded_month: Option<(NaiveDate, Option<SessionId>)>,
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
        let sessions = db::or_warn(db::list_sessions(&conn, false), "list sessions");
        Self {
            state,
            conn,
            page: Page::Tracker,
            view_month: Local::now().date_naive().with_day(1).unwrap(),
            selected_date: None,
            day_usage: Vec::new(),
            sessions,
            view_all: false,
            loaded_usage_key: None,
            show_new_session: false,
            new_session_name: String::new(),
            focus_new_session: false,
            manage_sessions: Vec::new(),
            renaming: None,
            rename_buf: String::new(),
            focus_rename: false,
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
    // back to the live "today" view; the page reloads any past-day rows lazily
    // via `loaded_usage_key`, so we only reset the open drill-down here.
    pub(crate) fn view_date(&mut self, date: NaiveDate, today: NaiveDate) {
        // Any open drill-down belongs to the previous day's data.
        self.expanded_app = None;
        self.expanded_titles.clear();

        self.selected_date = if date >= today { None } else { Some(date) };
    }

    // Whether the dashboard is showing the live in-memory map: today, scoped to
    // the active session. Every other view (a past day, or "All sessions") reads
    // from the database instead.
    pub(crate) fn is_live_view(&self) -> bool {
        self.selected_date.is_none() && !self.view_all
    }

    // The session filter for DB-backed views: None for "All", else the active one.
    pub(crate) fn view_filter(&self) -> Option<SessionId> {
        if self.view_all {
            None
        } else {
            Some(self.state.current_session.load(Ordering::Relaxed))
        }
    }

    // Per-title breakdown for an app: live from the in-memory map for the active
    // session today, or the cached DB rows otherwise. Sorted by time, descending.
    pub(crate) fn titles_for(&self, app: &str) -> Vec<UsageRow> {
        let mut titles: Vec<UsageRow> = if self.is_live_view() {
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

    // Reload the cached session list (e.g. after creating one).
    pub(crate) fn refresh_sessions(&mut self) {
        self.sessions = db::or_warn(db::list_sessions(&self.conn, false), "list sessions");
    }

    // Reload the manager's full list, including archived sessions.
    pub(crate) fn refresh_manage_sessions(&mut self) {
        self.manage_sessions = db::or_warn(db::list_sessions(&self.conn, true), "list all sessions");
    }

    // Make `id` the session new time records into. The tracker rolls the live map
    // over on its next tick; persisting to `meta` keeps "continue last" correct.
    pub(crate) fn set_active_session(&mut self, id: SessionId) {
        self.state.current_session.store(id, Ordering::Relaxed);
        db::or_warn(
            db::set_meta(&self.conn, db::ACTIVE_SESSION_KEY, &id.to_string()),
            "persist active session",
        );
    }
}

impl eframe::App for TrackerApp {
    // Flush today's usage on a graceful close, so the seconds since the last
    // periodic save aren't lost.
    fn on_exit(&mut self) {
        let today = db::date_key(Local::now().date_naive());
        let session = self.state.current_session.load(Ordering::Relaxed);
        db::or_warn(
            db::flush_usage(&self.conn, &today, session, &self.state.usage),
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

        // Left navigation rail, then the page content fills the rest.
        egui::Panel::left("nav")
            .resizable(false)
            .exact_size(176.0)
            .frame(
                egui::Frame::new()
                    .fill(SURFACE)
                    .inner_margin(egui::Margin::same(12)),
            )
            .show_inside(ui, |ui| self.sidebar(ui));

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(BG)
                    .inner_margin(egui::Margin::same(18)),
            )
            .show_inside(ui, |ui| match self.page {
                Page::Tracker => self.tracker_page(ui),
                Page::Calendar => self.calendar_page(ui),
                Page::Sessions => self.sessions_page(ui),
            });

        ctx.request_repaint_after(Duration::from_secs(self.config.refresh_rate_secs));
    }
}
