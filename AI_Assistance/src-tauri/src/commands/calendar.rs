use crate::db::DbState;
use chrono::Local;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct CalendarEvent {
    pub title: String,
    pub start_time: String,
    pub end_time: String,
    pub is_deep_work_block: bool,
}

#[tauri::command]
pub fn get_calendar_events(state: State<'_, DbState>) -> Result<Vec<CalendarEvent>, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let mut stmt = conn
        .prepare(
            "SELECT title, start_time, end_time, is_deep_work_block
             FROM calendar_events
             ORDER BY start_time ASC
             LIMIT 20",
        )
        .map_err(|e| e.to_string())?;

    let stored_events = stmt
        .query_map([], |row| {
            Ok(CalendarEvent {
                title: row.get(0)?,
                start_time: row.get(1)?,
                end_time: row.get(2)?,
                is_deep_work_block: row.get::<_, i64>(3)? > 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    if !stored_events.is_empty() {
        return Ok(stored_events);
    }

    let today = Local::now().format("%Y-%m-%d").to_string();

    Ok(vec![
        CalendarEvent {
            title: "Morning planning and priority review".to_string(),
            start_time: format!("{} 08:45", today),
            end_time: format!("{} 09:15", today),
            is_deep_work_block: false,
        },
        CalendarEvent {
            title: "Deep work: overlay command pipeline".to_string(),
            start_time: format!("{} 09:30", today),
            end_time: format!("{} 11:30", today),
            is_deep_work_block: true,
        },
        CalendarEvent {
            title: "Design sync and sprint updates".to_string(),
            start_time: format!("{} 14:00", today),
            end_time: format!("{} 14:45", today),
            is_deep_work_block: false,
        },
    ])
}
