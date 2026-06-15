//! SQLite storage: schema, queries, and the periodic flush.

use crate::paths;
use crate::state::{ActivityKey, IDLE, UsageMap, UsageRow};
use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const UPSERT_USAGE: &str =
    "INSERT INTO app_usage (date, app_name, title, seconds) VALUES (?1, ?2, ?3, ?4)
    ON CONFLICT (date, app_name, title) DO UPDATE SET seconds = excluded.seconds";

// Open a connection, waiting briefly on the lock rather than failing if another
// connection is mid-write.
pub fn open() -> Connection {
    let conn = Connection::open(paths::db_path()).expect("Can't open database");
    let _ = conn.busy_timeout(std::time::Duration::from_secs(5));
    conn
}

// Open the DB and ensure the schema exists. Panics on failure — the one place
// that's right, since the app can't run without storage (everything else
// degrades gracefully via `or_warn`).
pub fn init_db() -> Connection {
    let conn = open();
    run_migration(&conn);
    conn
}

fn run_migration(conn: &Connection) {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS app_usage (
            date TEXT NOT NULL,
            app_name TEXT NOT NULL,
            title TEXT NOT NULL,
            seconds INTEGER NOT NULL,
            PRIMARY KEY (date, app_name, title)
        )",
        [],
    )
    .expect("Could not create table");
}

// Unwrap a DB result, or log and fall back to the type's default — turning a
// query failure into a degraded-but-running state, never a crash or silent drop.
pub fn or_warn<T: Default>(result: rusqlite::Result<T>, context: &str) -> T {
    result.unwrap_or_else(|err| {
        eprintln!("suntrack: {context}: {err}");
        T::default()
    })
}

// The canonical "YYYY-MM-DD" encoding for the `date` column — defined once so
// every writer and reader agrees.
pub fn date_key(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

// Map a (label TEXT, seconds INTEGER) row to a UsageRow — shared by the app-total
// and title queries, which return the same two-column shape.
fn label_row(row: &Row) -> rusqlite::Result<UsageRow> {
    Ok(UsageRow::new(
        row.get::<_, String>(0)?,
        row.get::<_, i64>(1)? as u64,
    ))
}

// App-level totals for a day (titles summed per app) — what the dashboard's
// front page shows.
pub fn usage_for_date(conn: &Connection, date: &str) -> rusqlite::Result<Vec<UsageRow>> {
    let mut stmt = conn.prepare(
        "SELECT app_name, SUM(seconds) FROM app_usage WHERE date = ?1 GROUP BY app_name",
    )?;
    let rows = stmt.query_map(params![date], label_row)?;
    rows.collect()
}

// Per-day active totals (Idle excluded) for a month, for the calendar heatmap.
// `month_prefix` is "YYYY-MM".
pub fn totals_for_month(
    conn: &Connection,
    month_prefix: &str,
) -> rusqlite::Result<HashMap<String, u64>> {
    let mut stmt = conn.prepare(
        "SELECT date, SUM(seconds) FROM app_usage
            WHERE date LIKE ?1 AND app_name != ?2 GROUP BY date",
    )?;
    let pattern = format!("{month_prefix}%");
    let rows = stmt.query_map(params![pattern, IDLE], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
    })?;
    rows.collect()
}

// Per-window-title breakdown for one app on a day, used by the drill-down.
pub fn titles_for(conn: &Connection, date: &str, app: &str) -> rusqlite::Result<Vec<UsageRow>> {
    let mut stmt =
        conn.prepare("SELECT title, seconds FROM app_usage WHERE date = ?1 AND app_name = ?2")?;
    let rows = stmt.query_map(params![date, app], label_row)?;
    rows.collect()
}

// Load a day's full per-(app, title) breakdown into the live tracker map.
pub fn load_usage_for_date(
    conn: &Connection,
    date: &str,
    usage: &Arc<Mutex<UsageMap>>,
) -> rusqlite::Result<()> {
    let mut stmt =
        conn.prepare("SELECT app_name, title, seconds FROM app_usage WHERE date = ?1")?;
    let rows = stmt.query_map(params![date], |row| {
        Ok((
            ActivityKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
            row.get::<_, i64>(2)? as u64,
        ))
    })?;

    // Materialize the whole result before touching the live map, so a row error
    // mid-iteration can never leave it cleared or half-loaded.
    let loaded: UsageMap = rows.collect::<rusqlite::Result<_>>()?;
    *usage.lock().unwrap() = loaded;
    Ok(())
}

// Persist the whole map for `date`. Snapshots under the lock, then writes the
// snapshot in one transaction with a single reused statement — so the lock isn't
// held across disk I/O and the SQL is parsed once per flush, not per row.
pub fn flush_usage(
    conn: &Connection,
    date: &str,
    usage: &Arc<Mutex<UsageMap>>,
) -> rusqlite::Result<()> {
    let snapshot: Vec<(ActivityKey, u64)> = {
        let map = usage.lock().unwrap();
        map.iter().map(|(key, secs)| (key.clone(), *secs)).collect()
    };

    let tx = conn.unchecked_transaction()?;
    {
        let mut stmt = tx.prepare(UPSERT_USAGE)?;
        for (key, seconds) in snapshot {
            stmt.execute(params![date, key.app, key.title, seconds as i64])?;
        }
    }
    tx.commit()
}
