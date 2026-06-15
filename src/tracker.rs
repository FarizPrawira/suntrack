//! Background thread: samples the active window each tick and persists usage.

use crate::config::Config;
use crate::db;
use crate::state::{ActivityKey, SharedState, UNKNOWN};
use active_win_pos_rs::get_active_window;
use chrono::{Local, Timelike};
use rusqlite::Connection;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant};
use user_idle::UserIdle;

pub fn run_tracker(state: SharedState, config: Config) {
    let refresh_rate = config.refresh_rate_secs;

    // Cap how much time one tick can credit, so a long stall (sleep/suspend/
    // debugger) isn't dumped onto whatever app happens to be active now.
    let max_tick_gap = (refresh_rate * 5) as f64;

    let conn = db::open();

    let mut current_date = db::date_key(Local::now().date_naive());

    db::or_warn(
        db::load_usage_for_date(&conn, &current_date, &state.usage),
        "load today's usage",
    );

    let mut last_tick = Instant::now();
    let mut last_save = Instant::now();
    // Sub-second remainder carried between ticks (the map stores whole seconds).
    let mut carried_remainder = 0.0_f64;
    // Previous tick's on/off state, so we flush exactly once on the pause edge.
    let mut was_tracking = true;

    loop {
        thread::sleep(Duration::from_secs(refresh_rate));

        // Measure the real time that passed, not the assumed refresh rate.
        let measured_secs = last_tick.elapsed().as_secs_f64();
        last_tick = Instant::now();

        // Add the carried remainder, clamp away suspend-sized gaps, then split
        // into whole seconds to store and a new remainder to carry forward.
        let total_elapsed = (measured_secs + carried_remainder).min(max_tick_gap);
        let elapsed = total_elapsed.floor() as u64;
        carried_remainder = total_elapsed - elapsed as f64;

        // Detect a midnight crossing. `now` is sampled once and reused below to
        // split the tick that straddles midnight.
        let now = Local::now();
        let today = db::date_key(now.date_naive());
        let crossed_midnight = today != current_date;

        if elapsed == 0 {
            // No whole second to attribute this tick, but the day may still have
            // rolled over while sub-second ticks accumulated.
            if crossed_midnight {
                roll_day(&conn, &state, &mut current_date, today, &mut last_save);
            }
            continue;
        }

        // Tracking is paused: keep the loop alive (so timing/rollover stay
        // correct) but don't record any activity.
        let is_tracking = state.tracking.load(Ordering::Relaxed);
        if !is_tracking {
            *state.current_app.lock().unwrap() = None;
            if crossed_midnight {
                // The rollover flush already persists the closing day, so it
                // subsumes the pause flush.
                roll_day(&conn, &state, &mut current_date, today, &mut last_save);
            } else if was_tracking {
                // On the pause transition, persist once so a hard kill while
                // paused can't lose the seconds accumulated since the last save.
                db::or_warn(
                    db::flush_usage(&conn, &current_date, &state.usage),
                    "flush on pause",
                );
                last_save = Instant::now();
            }
            was_tracking = false;
            continue;
        }
        was_tracking = true;

        let idle_seconds = UserIdle::get_time()
            .map(|idle| idle.as_seconds())
            .unwrap_or(0);

        let key = if idle_seconds >= config.idle_timeout_secs {
            ActivityKey::idle()
        } else {
            match get_active_window() {
                Ok(window) => ActivityKey::new(window.app_name, window.title),
                Err(_) => ActivityKey::new(UNKNOWN, String::new()),
            }
        };

        *state.current_app.lock().unwrap() = Some(key.app.clone());

        if crossed_midnight {
            // The tick straddles midnight: seconds before midnight belong to the
            // day that's ending, the rest to the new one. `num_seconds_from_midnight`
            // caps how many of this tick's seconds can belong to the new day.
            let after_midnight = elapsed.min(now.num_seconds_from_midnight() as u64);
            let before_midnight = elapsed - after_midnight;

            if before_midnight > 0 {
                let mut map = state.usage.lock().unwrap();
                *map.entry(key.clone()).or_insert(0) += before_midnight;
            }

            // Flush the closing day (with its pre-midnight seconds) and reset the
            // map before crediting the new one. carried_remainder is left intact
            // on purpose — it's real sub-second time, and dropping it would
            // reintroduce the truncation loss it exists to prevent.
            roll_day(&conn, &state, &mut current_date, today, &mut last_save);

            if after_midnight > 0 {
                let mut map = state.usage.lock().unwrap();
                *map.entry(key).or_insert(0) += after_midnight;
            }
        } else {
            let mut map = state.usage.lock().unwrap();
            *map.entry(key).or_insert(0) += elapsed;
        }

        // Persist periodically instead of on every tick.
        if last_save.elapsed().as_secs() >= config.save_interval_secs {
            db::or_warn(
                db::flush_usage(&conn, &current_date, &state.usage),
                "periodic flush",
            );
            last_save = Instant::now();
        }
    }
}

// Persist the closing day, then reset the live map and save timer for the new
// one. Centralised so every midnight path (active, paused, sub-second) agrees.
fn roll_day(
    conn: &Connection,
    state: &SharedState,
    current_date: &mut String,
    today: String,
    last_save: &mut Instant,
) {
    // Persist the final state of the day before we throw it away.
    db::or_warn(
        db::flush_usage(conn, current_date.as_str(), &state.usage),
        "flush usage at day rollover",
    );
    *current_date = today;
    state.usage.lock().unwrap().clear();
    *last_save = Instant::now();
}
