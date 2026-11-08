//! CoderCraft eternal session memory (SQLite, disk-backed).
//!
//! Every agent turn is persisted so later elicitation ("what did we talk
//! about last September?") is a query, not a memory. Backed by
//! `databasecraft-store` (rusqlite under the hood) plus an FTS5 index.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::unimplemented)]
#![deny(clippy::todo)]

use databasecraft_store::{DatabaseStore, StoreError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    pub workspace_dir: String,
    pub mode: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
    pub turn_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnView {
    pub turn_id: i64,
    pub session_id: String,
    pub session_title: String,
    pub turn_seq: i64,
    pub role: String,
    pub content: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub turn_id: i64,
    pub session_id: String,
    pub session_title: String,
    pub role: String,
    pub snippet: String,
    pub rank: f64,
}

fn esc(s: &str) -> String {
    s.replace('\'', "''")
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

fn cell_str(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn cell_i64(v: &serde_json::Value) -> i64 {
    match v {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(0),
        serde_json::Value::String(s) => s.parse::<i64>().unwrap_or(0),
        _ => 0,
    }
}

/// Create tables if missing. Idempotent; safe to call on every dispatch.
pub fn ensure_schema(store: &DatabaseStore) -> Result<(), StoreError> {
    store.execute(
        "CREATE TABLE IF NOT EXISTS sessions (
            session_id TEXT PRIMARY KEY,
            workspace_dir TEXT NOT NULL,
            mode TEXT NOT NULL,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS turns (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            turn_seq INTEGER NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            tool_calls_json TEXT NOT NULL,
            timestamp_ms INTEGER NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_turns_session ON turns(session_id, turn_seq);
        CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);",
    )?;
    store.execute(
        "CREATE VIRTUAL TABLE IF NOT EXISTS session_fts USING fts5(content, session_id, role);",
    )?;
    Ok(())
}

fn session_exists(store: &DatabaseStore, session_id: &str) -> Result<bool, StoreError> {
    let qr = store.query(&format!(
        "SELECT COUNT(*) FROM sessions WHERE session_id = '{}';",
        esc(session_id)
    ))?;
    let n = qr
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    Ok(n > 0)
}

pub fn create_session(
    store: &DatabaseStore,
    session_id: &str,
    workspace_dir: &str,
    mode: &str,
    title: &str,
) -> Result<String, StoreError> {
    ensure_schema(store)?;
    let id = if session_id.trim().is_empty() {
        format!("sess-{}", now_ms())
    } else {
        session_id.trim().to_string()
    };
    let now = now_rfc3339();
    store.execute(&format!(
        "INSERT OR IGNORE INTO sessions (session_id, workspace_dir, mode, title, created_at, updated_at) VALUES ('{}', '{}', '{}', '{}', '{}', '{}');",
        esc(&id),
        esc(workspace_dir),
        esc(mode),
        esc(title),
        esc(&now),
        esc(&now)
    ))?;
    Ok(id)
}

pub fn append_turn(
    store: &DatabaseStore,
    session_id: &str,
    role: &str,
    content: &str,
    tool_calls_json: &str,
) -> Result<i64, StoreError> {
    ensure_schema(store)?;
    if !session_exists(store, session_id)? {
        return Err(StoreError::NotFound(format!(
            "session '{session_id}' not found; create it first"
        )));
    }
    let seq_qr = store.query(&format!(
        "SELECT COALESCE(MAX(turn_seq), 0) FROM turns WHERE session_id = '{}';",
        esc(session_id)
    ))?;
    let next_seq = seq_qr
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .map(|m| m + 1)
        .unwrap_or(1);
    let ts = now_ms();
    store.execute(&format!(
        "INSERT INTO turns (session_id, turn_seq, role, content, tool_calls_json, timestamp_ms) VALUES ('{}', {}, '{}', '{}', '{}', {});",
        esc(session_id),
        next_seq,
        esc(role),
        esc(content),
        esc(tool_calls_json),
        ts
    ))?;
    let id_qr = store.query("SELECT last_insert_rowid();")?;
    let row_id = id_qr
        .rows
        .first()
        .and_then(|r| r.first())
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    // Mirror into FTS with the same rowid so FTS hits join back to turns.
    store.execute(&format!(
        "INSERT INTO session_fts (rowid, content, session_id, role) VALUES ({}, '{}', '{}', '{}');",
        row_id,
        esc(content),
        esc(session_id),
        esc(role)
    ))?;
    store.execute(&format!(
        "UPDATE sessions SET updated_at = '{}' WHERE session_id = '{}';",
        esc(&now_rfc3339()),
        esc(session_id)
    ))?;
    Ok(row_id)
}

pub fn list_sessions(
    store: &DatabaseStore,
    workspace_filter: Option<&str>,
    since: Option<&str>,
    until: Option<&str>,
    limit: usize,
) -> Result<Vec<SessionInfo>, StoreError> {
    ensure_schema(store)?;
    let cap = limit.clamp(1, 200);
    let mut where_parts: Vec<String> = Vec::new();
    if let Some(w) = workspace_filter {
        if !w.trim().is_empty() {
            where_parts.push(format!("s.workspace_dir LIKE '%{}%'", esc(w)));
        }
    }
    if let Some(s) = since {
        if !s.trim().is_empty() {
            where_parts.push(format!("s.updated_at >= '{}'", esc(s)));
        }
    }
    if let Some(u) = until {
        if !u.trim().is_empty() {
            where_parts.push(format!("s.updated_at <= '{}'", esc(u)));
        }
    }
    let where_sql = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };
    let qr = store.query(&format!(
        "SELECT s.session_id, s.workspace_dir, s.mode, s.title, s.created_at, s.updated_at, COUNT(t.id) FROM sessions s LEFT JOIN turns t ON t.session_id = s.session_id {where_sql} GROUP BY s.session_id ORDER BY s.updated_at DESC LIMIT {cap};"
    ))?;
    let mut out = Vec::new();
    for row in &qr.rows {
        let get = |i: usize| row.get(i).map(cell_str).unwrap_or_default();
        let count = row.get(6).map(cell_i64).unwrap_or(0);
        out.push(SessionInfo {
            session_id: get(0),
            workspace_dir: get(1),
            mode: get(2),
            title: get(3),
            created_at: get(4),
            updated_at: get(5),
            turn_count: count,
        });
    }
    Ok(out)
}

fn sanitize_fts(query: &str) -> String {
    let mut s: String = query
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect();
    s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if s.len() > 200 {
        s.truncate(200);
    }
    if s.trim().is_empty() {
        s = "codercraft".to_string();
    }
    s
}

pub fn search(
    store: &DatabaseStore,
    query: &str,
    top_k: usize,
) -> Result<Vec<SearchHit>, StoreError> {
    ensure_schema(store)?;
    let cap = top_k.clamp(1, 50);
    let fts_q = sanitize_fts(query);
    // FTS5 MATCH can reject odd input; fall back to LIKE on error.
    let fts_rows = match store.query(&format!(
        "SELECT rowid, rank FROM session_fts WHERE session_fts MATCH '{}' ORDER BY rank LIMIT {};",
        esc(&fts_q),
        cap
    )) {
        Ok(qr) => qr,
        Err(_) => {
            return like_search(store, query, cap);
        }
    };
    let mut out = Vec::new();
    for row in &fts_rows.rows {
        let turn_id = row.first().map(cell_i64).unwrap_or(0);
        let rank = row.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        if turn_id <= 0 {
            continue;
        }
        let detail = store.query(&format!(
            "SELECT t.content, t.session_id, t.role, COALESCE(s.title, '') FROM turns t LEFT JOIN sessions s ON s.session_id = t.session_id WHERE t.id = {turn_id};"
        ))?;
        if let Some(drow) = detail.rows.first() {
            let content = drow.first().map(cell_str).unwrap_or_default();
            let snippet: String = content.chars().take(280).collect();
            out.push(SearchHit {
                turn_id,
                session_id: drow.get(1).map(cell_str).unwrap_or_default(),
                session_title: drow.get(3).map(cell_str).unwrap_or_default(),
                role: drow.get(2).map(cell_str).unwrap_or_default(),
                snippet,
                rank,
            });
        }
    }
    Ok(out)
}

fn like_search(
    store: &DatabaseStore,
    query: &str,
    cap: usize,
) -> Result<Vec<SearchHit>, StoreError> {
    let needle = esc(query.trim());
    let qr = store.query(&format!(
        "SELECT t.id, t.session_id, t.role, COALESCE(s.title, ''), substr(t.content, 1, 280) FROM turns t LEFT JOIN sessions s ON s.session_id = t.session_id WHERE t.content LIKE '%{needle}%' ORDER BY t.timestamp_ms DESC LIMIT {cap};"
    ))?;
    let mut out = Vec::new();
    for row in &qr.rows {
        out.push(SearchHit {
            turn_id: row.first().map(cell_i64).unwrap_or(0),
            session_id: row.get(1).map(cell_str).unwrap_or_default(),
            role: row.get(2).map(cell_str).unwrap_or_default(),
            session_title: row.get(3).map(cell_str).unwrap_or_default(),
            snippet: row.get(4).map(cell_str).unwrap_or_default(),
            rank: 0.0,
        });
    }
    Ok(out)
}

/// Elicitation by calendar month: "what did we talk about last September?"
/// Matches on the session's creation month (YYYY-MM prefix on RFC3339).
pub fn recall_month(
    store: &DatabaseStore,
    year: i64,
    month: u32,
    limit: usize,
) -> Result<Vec<TurnView>, StoreError> {
    ensure_schema(store)?;
    if !(2000..=2100).contains(&year) || !(1..=12).contains(&month) {
        return Err(StoreError::InvalidInput(format!(
            "bad year/month {year}-{month}; want YYYY and 1-12"
        )));
    }
    let cap = limit.clamp(1, 200);
    let prefix = format!("{year:04}-{month:02}");
    let qr = store.query(&format!(
        "SELECT t.id, t.session_id, COALESCE(s.title, ''), t.turn_seq, t.role, t.content, t.timestamp_ms FROM turns t JOIN sessions s ON s.session_id = t.session_id WHERE s.created_at LIKE '{prefix}%' ORDER BY t.timestamp_ms ASC LIMIT {cap};"
    ))?;
    let mut out = Vec::new();
    for row in &qr.rows {
        out.push(TurnView {
            turn_id: row.first().map(cell_i64).unwrap_or(0),
            session_id: row.get(1).map(cell_str).unwrap_or_default(),
            session_title: row.get(2).map(cell_str).unwrap_or_default(),
            turn_seq: row.get(3).map(cell_i64).unwrap_or(0),
            role: row.get(4).map(cell_str).unwrap_or_default(),
            content: row.get(5).map(cell_str).unwrap_or_default(),
            timestamp_ms: row.get(6).map(cell_i64).unwrap_or(0),
        });
    }
    Ok(out)
}

pub fn read_session(
    store: &DatabaseStore,
    session_id: &str,
    limit: usize,
) -> Result<Vec<TurnView>, StoreError> {
    ensure_schema(store)?;
    let cap = limit.clamp(1, 500);
    let qr = store.query(&format!(
        "SELECT t.id, t.session_id, COALESCE(s.title, ''), t.turn_seq, t.role, t.content, t.timestamp_ms FROM turns t LEFT JOIN sessions s ON s.session_id = t.session_id WHERE t.session_id = '{}' ORDER BY t.turn_seq ASC LIMIT {cap};",
        esc(session_id)
    ))?;
    let mut out = Vec::new();
    for row in &qr.rows {
        out.push(TurnView {
            turn_id: row.first().map(cell_i64).unwrap_or(0),
            session_id: row.get(1).map(cell_str).unwrap_or_default(),
            session_title: row.get(2).map(cell_str).unwrap_or_default(),
            turn_seq: row.get(3).map(cell_i64).unwrap_or(0),
            role: row.get(4).map(cell_str).unwrap_or_default(),
            content: row.get(5).map(cell_str).unwrap_or_default(),
            timestamp_ms: row.get(6).map(cell_i64).unwrap_or(0),
        });
    }
    Ok(out)
}
