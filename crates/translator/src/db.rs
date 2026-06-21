//! SQLite call history — port of web/db.py.

use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::paths::{db_path, ensure_parent};
use crate::settings::{Settings, CALL_IDLE_TIMEOUT_SECS};

pub struct Db {
    conn: Mutex<Connection>,
    call: Mutex<CallTracker>,
}

struct CallTracker {
    current_call_id: Option<i64>,
    last_activity: u64,
    pending: std::collections::HashMap<String, PendingUtterance>,
}

struct PendingUtterance {
    transcript: String,
    ts: String,
    id: Option<i64>,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = db_path();
        ensure_parent(&path)?;
        let conn = Connection::open(&path).context("open calls.db")?;
        conn.execute_batch("PRAGMA journal_mode=WAL")?;
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS calls (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                ended_at TEXT,
                my_language TEXT,
                their_language TEXT,
                summary TEXT
            );
            CREATE TABLE IF NOT EXISTS utterances (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                call_id INTEGER NOT NULL REFERENCES calls(id),
                ts TEXT NOT NULL,
                direction TEXT NOT NULL,
                speaker TEXT NOT NULL,
                original TEXT,
                translated TEXT
            );
            ",
        )?;
        let now = chrono_ts();
        conn.execute(
            "UPDATE calls SET ended_at = ?1 WHERE ended_at IS NULL",
            params![now],
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
            call: Mutex::new(CallTracker {
                current_call_id: None,
                last_activity: 0,
                pending: std::collections::HashMap::new(),
            }),
        })
    }

    pub fn new_session(&self, settings: &Settings) -> Result<i64> {
        let mut tracker = self.call.lock().unwrap();
        tracker.close_inner(&self.conn.lock().unwrap())?;
        let conn = self.conn.lock().unwrap();
        let now = chrono_ts();
        conn.execute(
            "INSERT INTO calls (started_at, my_language, their_language) VALUES (?1, ?2, ?3)",
            params![
                now,
                settings.str_field("my_language"),
                settings.str_field("their_language")
            ],
        )?;
        let id = conn.last_insert_rowid();
        tracker.current_call_id = Some(id);
        tracker.last_activity = now_secs();
        tracker.pending.clear();
        Ok(id)
    }

    pub fn end_call(&self) -> Result<()> {
        let mut tracker = self.call.lock().unwrap();
        tracker.close_inner(&self.conn.lock().unwrap())?;
        Ok(())
    }

    pub fn record_transcript(&self, direction: &str, text: &str) {
        let mut tracker = self.call.lock().unwrap();
        let call_id = match tracker.ensure_call(&self.conn.lock().unwrap()) {
            Ok(id) => id,
            Err(_) => return,
        };
        let speaker = if direction == "outgoing" { "me" } else { "them" };
        let ts = chrono_ts();
        let conn = self.conn.lock().unwrap();
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM utterances WHERE call_id = ?1 AND direction = ?2 AND original = ?3",
                params![call_id, direction, text],
                |r| r.get(0),
            )
            .ok();
        let utterance_id = if let Some(id) = existing {
            id
        } else if conn
            .execute(
                "INSERT INTO utterances (call_id, ts, direction, speaker, original, translated) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![call_id, ts, direction, speaker, text, ""],
            )
            .is_ok()
        {
            conn.last_insert_rowid()
        } else {
            return;
        };
        tracker.pending.insert(
            direction.to_string(),
            PendingUtterance {
                transcript: text.to_string(),
                ts,
                id: Some(utterance_id),
            },
        );
    }

    pub fn record_translation(&self, direction: &str, text: &str) {
        let mut tracker = self.call.lock().unwrap();
        if let Some(prev) = tracker.pending.remove(direction) {
            let conn = self.conn.lock().unwrap();
            if let Some(id) = prev.id {
                let _ = conn.execute(
                    "UPDATE utterances SET translated = ?1 WHERE id = ?2",
                    params![text, id],
                );
            } else if let Ok(call_id) = tracker.ensure_call(&conn) {
                let speaker = if direction == "outgoing" { "me" } else { "them" };
                let _ = conn.execute(
                    "INSERT INTO utterances (call_id, ts, direction, speaker, original, translated) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![call_id, prev.ts, direction, speaker, prev.transcript, text],
                );
            }
        }
    }

    pub fn list_calls(&self) -> Result<Vec<Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.*, COUNT(u.id) as utterance_count FROM calls c LEFT JOIN utterances u ON u.call_id = c.id GROUP BY c.id ORDER BY c.id DESC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(json!({
                "id": row.get::<_, i64>(0)?,
                "started_at": row.get::<_, String>(1)?,
                "ended_at": row.get::<_, Option<String>>(2)?,
                "my_language": row.get::<_, Option<String>>(3)?,
                "their_language": row.get::<_, Option<String>>(4)?,
                "summary": row.get::<_, Option<String>>(5)?,
                "utterance_count": row.get::<_, i64>(6)?,
            }))
        })?;
        rows.collect::<Result<Vec<_>, _>>().context("list calls")
    }

    pub fn get_call(&self, call_id: i64) -> Result<Option<Value>> {
        let conn = self.conn.lock().unwrap();
        let call: Option<Value> = conn
            .query_row("SELECT * FROM calls WHERE id = ?1", params![call_id], |row| {
                Ok(json!({
                    "id": row.get::<_, i64>(0)?,
                    "started_at": row.get::<_, String>(1)?,
                    "ended_at": row.get::<_, Option<String>>(2)?,
                    "my_language": row.get::<_, Option<String>>(3)?,
                    "their_language": row.get::<_, Option<String>>(4)?,
                    "summary": row.get::<_, Option<String>>(5)?,
                }))
            })
            .ok();
        let Some(call) = call else {
            return Ok(None);
        };
        let mut stmt =
            conn.prepare("SELECT * FROM utterances WHERE call_id = ?1 ORDER BY id")?;
        let utterances = stmt
            .query_map(params![call_id], |row| {
                Ok(json!({
                    "id": row.get::<_, i64>(0)?,
                    "call_id": row.get::<_, i64>(1)?,
                    "ts": row.get::<_, String>(2)?,
                    "direction": row.get::<_, String>(3)?,
                    "speaker": row.get::<_, String>(4)?,
                    "original": row.get::<_, Option<String>>(5)?,
                    "translated": row.get::<_, Option<String>>(6)?,
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(json!({ "call": call, "utterances": utterances })))
    }

    pub fn delete_call(&self, call_id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM utterances WHERE call_id = ?1", params![call_id])?;
        conn.execute("DELETE FROM calls WHERE id = ?1", params![call_id])?;
        Ok(())
    }

    pub fn save_summary(&self, call_id: i64, summary: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE calls SET summary = ?1 WHERE id = ?2",
            params![summary, call_id],
        )?;
        Ok(())
    }

    pub fn utterances_for_summary(&self, call_id: i64) -> Result<Vec<(String, String, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT speaker, original, translated FROM utterances WHERE call_id = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![call_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                row.get::<_, Option<String>>(2)?.unwrap_or_default(),
            ))
        })?;
        rows.collect::<Result<Vec<_>, _>>().context("utterances")
    }
}

impl CallTracker {
    fn ensure_call(&mut self, conn: &Connection) -> Result<i64> {
        let now = now_secs();
        if self.current_call_id.is_some()
            && now.saturating_sub(self.last_activity) > CALL_IDLE_TIMEOUT_SECS
        {
            self.close_inner(conn)?;
        }
        if self.current_call_id.is_none() {
            let settings = Settings::load().unwrap_or_default();
            let ts = chrono_ts();
            conn.execute(
                "INSERT INTO calls (started_at, my_language, their_language) VALUES (?1, ?2, ?3)",
                params![
                    ts,
                    settings.str_field("my_language"),
                    settings.str_field("their_language")
                ],
            )?;
            self.current_call_id = Some(conn.last_insert_rowid());
        }
        self.last_activity = now;
        Ok(self.current_call_id.unwrap())
    }

    fn close_inner(&mut self, conn: &Connection) -> Result<()> {
        if let Some(id) = self.current_call_id.take() {
            conn.execute(
                "UPDATE calls SET ended_at = ?1 WHERE id = ?2",
                params![chrono_ts(), id],
            )?;
        }
        self.pending.clear();
        Ok(())
    }
}

fn chrono_ts() -> String {
    chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
