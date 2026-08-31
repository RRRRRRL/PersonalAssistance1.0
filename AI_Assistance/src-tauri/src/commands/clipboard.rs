//! Clipboard Manager — cross-platform clipboard access via `arboard` crate.
//!
//! Provides read/write clipboard text and a searchable history (SQLite-backed).

use crate::db::DbState;
use arboard::Clipboard;
use rusqlite::params;
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: String,
    pub preview: String,
    pub copied_at: i64,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[tauri::command]
pub fn get_clipboard_text() -> Result<String, String> {
    let mut clip = Clipboard::new().map_err(|e| e.to_string())?;
    clip.get_text().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_clipboard_text(text: String, state: State<'_, DbState>) -> Result<(), String> {
    let mut clip = Clipboard::new().map_err(|e| e.to_string())?;
    clip.set_text(&text).map_err(|e| e.to_string())?;

    // Also save to history
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let now = now_ts();
    let preview = if text.len() > 80 {
        let truncated: String = text.chars().take(77).collect();
        format!("{}...", truncated)
    } else {
        text.clone()
    };

    // Avoid duplicate consecutive entries
    let last_content: Option<String> = conn
        .query_row(
            "SELECT content FROM clipboard_history ORDER BY copied_at DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();

    if last_content.as_deref() != Some(&text) {
        conn.execute(
            "INSERT INTO clipboard_history (content, preview, copied_at) VALUES (?1, ?2, ?3)",
            params![text, preview, now],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub fn get_clipboard_history(
    query: Option<String>,
    limit: Option<i64>,
    state: State<'_, DbState>,
) -> Result<Vec<ClipboardEntry>, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let max = limit.unwrap_or(30).min(100);

    let entries = if let Some(q) = query {
        let pattern = format!("%{}%", q);
        let mut stmt = conn
            .prepare(
                "SELECT id, content, preview, copied_at FROM clipboard_history
                 WHERE content LIKE ?1 ORDER BY copied_at DESC LIMIT ?2",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![pattern, max], |row| {
            Ok(ClipboardEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                preview: row.get(2)?,
                copied_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect()
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, content, preview, copied_at FROM clipboard_history
                 ORDER BY copied_at DESC LIMIT ?1",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_map(params![max], |row| {
            Ok(ClipboardEntry {
                id: row.get(0)?,
                content: row.get(1)?,
                preview: row.get(2)?,
                copied_at: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect()
    };

    Ok(entries)
}

#[tauri::command]
pub fn clear_clipboard_history(state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    conn.execute("DELETE FROM clipboard_history", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}
