//! SQLite storage: schema, queries, and the periodic flush.

use crate::paths;
use crate::state::{ActivityKey, DEFAULT_SESSION_ID, IDLE, SessionId, UsageMap, UsageRow};
use chrono::NaiveDate;
use rusqlite::{Connection, Row, params};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// The `meta` key under which the active session id is persisted, so the next
// launch can continue the session you were last in.
pub const ACTIVE_SESSION_KEY: &str = "active_session_id";

const UPSERT_USAGE: &str =
    "INSERT INTO app_usage (date, session_id, app_name, title, seconds) VALUES (?1, ?2, ?3, ?4, ?5)
    ON CONFLICT (date, session_id, app_name, title) DO UPDATE SET seconds = excluded.seconds";

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
    // The `sessions` table doubles as our migration marker: if it doesn't exist,
    // this database predates sessions and its `app_usage` has an incompatible
    // primary key. Per the 0.2 design we deliberately drop that pre-sessions data
    // rather than carry a versioned migration — the version bump signals the wipe.
    // (On a fresh install there's nothing to drop and this is a no-op.)
    let has_sessions: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'sessions'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !has_sessions {
        conn.execute("DROP TABLE IF EXISTS app_usage", [])
            .expect("Could not drop legacy app_usage table");
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            archived INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS app_usage (
            date TEXT NOT NULL,
            session_id INTEGER NOT NULL,
            app_name TEXT NOT NULL,
            title TEXT NOT NULL,
            seconds INTEGER NOT NULL,
            PRIMARY KEY (date, session_id, app_name, title)
        );
        CREATE TABLE IF NOT EXISTS meta (
            key TEXT PRIMARY KEY,
            value TEXT
        );",
    )
    .expect("Could not create tables");

    // Seed a default session so there is always at least one to record into.
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, name) VALUES (?1, 'General')",
        params![DEFAULT_SESSION_ID],
    )
    .expect("Could not seed default session");
}

// A session: the named context tracked time is filed under.
#[derive(Clone, Debug)]
pub struct Session {
    pub id: SessionId,
    pub name: String,
    // The sessions manager splits its list on this; the pickers exclude archived
    // sessions up front via `list_sessions(.., false)`.
    pub archived: bool,
}

// All sessions, ordered by creation (id). `include_archived` is false for the
// pickers, which only offer live sessions.
pub fn list_sessions(conn: &Connection, include_archived: bool) -> rusqlite::Result<Vec<Session>> {
    let sql = if include_archived {
        "SELECT id, name, archived FROM sessions ORDER BY id"
    } else {
        "SELECT id, name, archived FROM sessions WHERE archived = 0 ORDER BY id"
    };
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(Session {
            id: row.get(0)?,
            name: row.get(1)?,
            archived: row.get::<_, i64>(2)? != 0,
        })
    })?;
    rows.collect()
}

// Create a session and return its new id.
pub fn create_session(conn: &Connection, name: &str) -> rusqlite::Result<SessionId> {
    conn.execute("INSERT INTO sessions (name) VALUES (?1)", params![name])?;
    Ok(conn.last_insert_rowid())
}

// Rename a session.
pub fn rename_session(conn: &Connection, id: SessionId, name: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE sessions SET name = ?1 WHERE id = ?2", params![name, id])
        .map(|_| ())
}

// Archive or restore a session. Archived sessions keep their recorded time but
// drop out of the pickers (`list_sessions` with `include_archived = false`); the
// manager page is the only place they remain visible, for restoring.
pub fn set_archived(conn: &Connection, id: SessionId, archived: bool) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE sessions SET archived = ?1 WHERE id = ?2",
        params![archived as i64, id],
    )
    .map(|_| ())
}

// The name of a session, or None if no such row exists.
fn session_name(conn: &Connection, id: SessionId) -> Option<String> {
    conn.query_row("SELECT name FROM sessions WHERE id = ?1", params![id], |r| {
        r.get(0)
    })
    .ok()
}

// The id of the session with this exact name, if one exists.
fn session_id_by_name(conn: &Connection, name: &str) -> Option<SessionId> {
    conn.query_row("SELECT id FROM sessions WHERE name = ?1", params![name], |r| {
        r.get(0)
    })
    .ok()
}

// Read a `meta` value by key.
fn get_meta(conn: &Connection, key: &str) -> Option<String> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", params![key], |r| {
        r.get(0)
    })
    .ok()
}

// Write (insert or replace) a `meta` value.
pub fn set_meta(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map(|_| ())
}

// The last active session, falling back to the default if it's unset or points
// at a session that no longer exists.
fn last_active_session(conn: &Connection) -> SessionId {
    match get_meta(conn, ACTIVE_SESSION_KEY).and_then(|v| v.parse::<SessionId>().ok()) {
        Some(id) if session_name(conn, id).is_some() => id,
        _ => DEFAULT_SESSION_ID,
    }
}

// Decide which session to start in on launch. A blank `default_name` means
// "continue the last session"; otherwise the named session is used, created if
// it doesn't exist yet.
pub fn resolve_start_session(conn: &Connection, default_name: &str) -> SessionId {
    let name = default_name.trim();
    if name.is_empty() {
        return last_active_session(conn);
    }
    if let Some(id) = session_id_by_name(conn, name) {
        return id;
    }
    match create_session(conn, name) {
        Ok(id) => id,
        Err(err) => {
            eprintln!("suntrack: could not create default session '{name}': {err}");
            last_active_session(conn)
        }
    }
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
// front page shows. `session` filters to one session; None combines all of them.
pub fn usage_for_date(
    conn: &Connection,
    date: &str,
    session: Option<SessionId>,
) -> rusqlite::Result<Vec<UsageRow>> {
    let mut stmt = conn.prepare(
        "SELECT app_name, SUM(seconds) FROM app_usage
            WHERE date = ?1 AND (?2 IS NULL OR session_id = ?2) GROUP BY app_name",
    )?;
    let rows = stmt.query_map(params![date, session], label_row)?;
    rows.collect()
}

// Per-day active totals (Idle excluded) for a month, for the calendar heatmap.
// `month_prefix` is "YYYY-MM"; `session` filters to one session, None to all.
pub fn totals_for_month(
    conn: &Connection,
    month_prefix: &str,
    session: Option<SessionId>,
) -> rusqlite::Result<HashMap<String, u64>> {
    let mut stmt = conn.prepare(
        "SELECT date, SUM(seconds) FROM app_usage
            WHERE date LIKE ?1 AND app_name != ?2 AND (?3 IS NULL OR session_id = ?3)
            GROUP BY date",
    )?;
    let pattern = format!("{month_prefix}%");
    let rows = stmt.query_map(params![pattern, IDLE, session], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u64))
    })?;
    rows.collect()
}

// Per-window-title breakdown for one app on a day, used by the drill-down.
// Summed per title so it's correct whether scoped to one session or all of them.
pub fn titles_for(
    conn: &Connection,
    date: &str,
    app: &str,
    session: Option<SessionId>,
) -> rusqlite::Result<Vec<UsageRow>> {
    let mut stmt = conn.prepare(
        "SELECT title, SUM(seconds) FROM app_usage
            WHERE date = ?1 AND app_name = ?2 AND (?3 IS NULL OR session_id = ?3)
            GROUP BY title",
    )?;
    let rows = stmt.query_map(params![date, app, session], label_row)?;
    rows.collect()
}

// Load one session's day into the live tracker map (its per-(app, title) seconds).
pub fn load_usage_for_date(
    conn: &Connection,
    date: &str,
    session: SessionId,
    usage: &Arc<Mutex<UsageMap>>,
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT app_name, title, seconds FROM app_usage WHERE date = ?1 AND session_id = ?2",
    )?;
    let rows = stmt.query_map(params![date, session], |row| {
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

// Persist the whole map for `date` under `session`. Snapshots under the lock,
// then writes the snapshot in one transaction with a single reused statement —
// so the lock isn't held across disk I/O and the SQL is parsed once per flush,
// not per row.
pub fn flush_usage(
    conn: &Connection,
    date: &str,
    session: SessionId,
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
            stmt.execute(params![date, session, key.app, key.title, seconds as i64])?;
        }
    }
    tx.commit()
}
