use crate::db::DbState;
use dirs::desktop_dir;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Clone, Serialize)]
pub struct FileHistoryEntry {
    pub id: i64,
    pub path: String,
    pub name: String,
    pub last_accessed: i64,
    pub access_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecentAndFrequentFiles {
    pub recent: Vec<FileHistoryEntry>,
    pub frequent: Vec<FileHistoryEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileGroupingPolicy {
    pub root_folder_name: String,
    pub older_than_days: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileGroupingPolicyInput {
    pub root_folder_name: Option<String>,
    pub older_than_days: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileGroupingCandidate {
    pub file_name: String,
    pub from_path: String,
    pub target_folder: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileGroupingPreview {
    pub policy: FileGroupingPolicy,
    pub candidate_count: i64,
    pub candidates: Vec<FileGroupingCandidate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileGroupingResult {
    pub policy: FileGroupingPolicy,
    pub total_candidates: i64,
    pub moved_count: i64,
    pub skipped_count: i64,
    pub created_folders: Vec<String>,
    pub errors: Vec<String>,
}

fn now_ts() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

fn category_for_extension(ext: &str) -> &'static str {
    match ext {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" => "Images",
        "mp3" | "wav" | "m4a" | "flac" => "Audio",
        "mp4" | "mov" | "mkv" | "avi" | "webm" => "Video",
        "zip" | "rar" | "7z" | "tar" | "gz" => "Archives",
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx" | "txt" | "md" => "Documents",
        "ts" | "js" | "jsx" | "tsx" | "rs" | "py" | "java" | "c" | "cpp" | "go" | "cs" => "Code",
        "csv" | "json" | "yaml" | "yml" | "xml" => "Data",
        _ => "Other",
    }
}

fn normalize_segment(raw: &str) -> String {
    let cleaned = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();

    let trimmed = cleaned.trim_matches('_').trim();
    if trimmed.is_empty() {
        "misc".to_string()
    } else {
        trimmed.to_string()
    }
}

fn relation_key(file_name: &str) -> Option<String> {
    let stem = Path::new(file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let key = stem
        .split(['_', '-', ' ', '.'])
        .find(|part| part.len() >= 4)
        .map(normalize_segment)?;
    Some(key)
}

fn compute_grouping_candidates(
    desktop_path: &Path,
    policy: &FileGroupingPolicy,
) -> Result<Vec<FileGroupingCandidate>, String> {
    let now = now_ts()?;
    let cutoff = now.saturating_sub(policy.older_than_days.saturating_mul(24 * 60 * 60));

    let mut entries: Vec<(PathBuf, String, String, Option<String>)> = Vec::new();

    for entry in fs::read_dir(desktop_path).map_err(|e| e.to_string())? {
        let entry = match entry {
            Ok(value) => value,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = match path.file_name().and_then(|name| name.to_str()) {
            Some(value) => value.to_string(),
            None => continue,
        };

        if file_name.starts_with('.') {
            continue;
        }

        let metadata = match fs::metadata(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let modified = match metadata.modified() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let modified_ts = match modified.duration_since(UNIX_EPOCH) {
            Ok(value) => value.as_secs() as i64,
            Err(_) => continue,
        };

        if modified_ts > cutoff {
            continue;
        }

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let category = category_for_extension(&ext).to_string();
        let key = relation_key(&file_name);

        entries.push((path, file_name, category, key));
    }

    let mut key_counts: HashMap<String, i64> = HashMap::new();
    for (_, _, _, key) in &entries {
        if let Some(key) = key {
            *key_counts.entry(key.clone()).or_insert(0) += 1;
        }
    }

    let mut candidates = Vec::new();
    for (path, file_name, category, key) in entries {
        let (target_folder, reason) = if let Some(group_key) = key {
            if key_counts.get(&group_key).copied().unwrap_or(0) >= 2 {
                (
                    format!("Related/{}", normalize_segment(&group_key)),
                    format!("Related group by shared name key '{}'", group_key),
                )
            } else {
                (
                    format!("ByType/{}", normalize_segment(&category)),
                    format!("Type-based grouping for {}", category),
                )
            }
        } else {
            (
                format!("ByType/{}", normalize_segment(&category)),
                format!("Type-based grouping for {}", category),
            )
        };

        candidates.push(FileGroupingCandidate {
            file_name,
            from_path: path.to_string_lossy().to_string(),
            target_folder,
            reason,
        });
    }

    Ok(candidates)
}

fn load_policy(state: State<'_, DbState>) -> Result<FileGroupingPolicy, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let mut stmt = conn
        .prepare(
            "SELECT root_folder_name, older_than_days, updated_at
             FROM file_grouping_policies
             WHERE id = 1",
        )
        .map_err(|e| e.to_string())?;

    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(FileGroupingPolicy {
            root_folder_name: row.get(0).map_err(|e| e.to_string())?,
            older_than_days: row.get(1).map_err(|e| e.to_string())?,
            updated_at: row.get(2).map_err(|e| e.to_string())?,
        })
    } else {
        Ok(FileGroupingPolicy {
            root_folder_name: "AI_AutoGroup".to_string(),
            older_than_days: 90,
            updated_at: 0,
        })
    }
}

#[tauri::command]
pub fn get_desktop_files() -> Result<Vec<String>, String> {
    let desktop_path = desktop_dir().ok_or("Could not resolve desktop directory")?;

    let files = fs::read_dir(desktop_path)
        .map_err(|e| e.to_string())?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            if path.is_file() {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    Ok(files)
}

#[tauri::command]
pub fn log_file_access(path: String, name: String, state: State<'_, DbState>) -> Result<(), String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64;

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "INSERT INTO file_history (path, name, last_accessed, access_count)
         VALUES (?1, ?2, ?3, 1)
         ON CONFLICT(path) DO UPDATE SET
            name = excluded.name,
            last_accessed = excluded.last_accessed,
            access_count = file_history.access_count + 1",
        params![path, name, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn open_file(path: String, app: AppHandle) -> Result<(), String> {
    let path = PathBuf::from(path.trim());

    if !path.is_file() {
        return Err(format!("File is unavailable: {}", path.display()));
    }

    app.opener()
        .open_path(&path, None::<&str>)
        .map_err(|error| format!("Could not open {}: {}", path.display(), error))
}

#[tauri::command]
pub fn get_recent_and_frequent_files(state: State<'_, DbState>) -> Result<RecentAndFrequentFiles, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;

    let mut recent_stmt = conn
        .prepare(
            "SELECT id, path, name, last_accessed, access_count
             FROM file_history
             ORDER BY last_accessed DESC
             LIMIT 5",
        )
        .map_err(|e| e.to_string())?;

    let recent = recent_stmt
        .query_map([], |row| {
            Ok(FileHistoryEntry {
                id: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                last_accessed: row.get(3)?,
                access_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut frequent_stmt = conn
        .prepare(
            "SELECT id, path, name, last_accessed, access_count
             FROM file_history
             ORDER BY access_count DESC, last_accessed DESC
             LIMIT 5",
        )
        .map_err(|e| e.to_string())?;

    let frequent = frequent_stmt
        .query_map([], |row| {
            Ok(FileHistoryEntry {
                id: row.get(0)?,
                path: row.get(1)?,
                name: row.get(2)?,
                last_accessed: row.get(3)?,
                access_count: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    Ok(RecentAndFrequentFiles { recent, frequent })
}

#[tauri::command]
pub fn get_file_grouping_policy(state: State<'_, DbState>) -> Result<FileGroupingPolicy, String> {
    load_policy(state)
}

#[tauri::command]
pub fn save_file_grouping_policy(
    input: FileGroupingPolicyInput,
    state: State<'_, DbState>,
) -> Result<FileGroupingPolicy, String> {
    let now = now_ts()?;
    let current = load_policy(state.clone())?;

    let updated = FileGroupingPolicy {
        root_folder_name: input
            .root_folder_name
            .map(|value| normalize_segment(value.trim()))
            .filter(|value| !value.is_empty())
            .unwrap_or(current.root_folder_name),
        older_than_days: input
            .older_than_days
            .map(|value| value.clamp(7, 3650))
            .unwrap_or(current.older_than_days),
        updated_at: now,
    };

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "INSERT INTO file_grouping_policies (id, root_folder_name, older_than_days, updated_at)
         VALUES (1, ?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET
           root_folder_name = excluded.root_folder_name,
           older_than_days = excluded.older_than_days,
           updated_at = excluded.updated_at",
        params![updated.root_folder_name, updated.older_than_days, updated.updated_at],
    )
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

#[tauri::command]
pub fn preview_file_grouping_batch(
    state: State<'_, DbState>,
) -> Result<FileGroupingPreview, String> {
    let policy = load_policy(state)?;
    let desktop_path = desktop_dir().ok_or("Could not resolve desktop directory")?;
    let candidates = compute_grouping_candidates(&desktop_path, &policy)?;

    Ok(FileGroupingPreview {
        policy,
        candidate_count: candidates.len() as i64,
        candidates,
    })
}

#[tauri::command]
pub fn run_file_grouping_batch(state: State<'_, DbState>) -> Result<FileGroupingResult, String> {
    let policy = load_policy(state.clone())?;
    let desktop_path = desktop_dir().ok_or("Could not resolve desktop directory")?;
    let root_folder = desktop_path.join(normalize_segment(&policy.root_folder_name));

    fs::create_dir_all(&root_folder).map_err(|e| e.to_string())?;

    let candidates = compute_grouping_candidates(&desktop_path, &policy)?;
    let total_candidates = candidates.len() as i64;

    let mut moved_count = 0_i64;
    let mut skipped_count = 0_i64;
    let mut created_folders = Vec::<String>::new();
    let mut errors = Vec::<String>::new();

    for candidate in &candidates {
        let source_path = PathBuf::from(&candidate.from_path);
        if !source_path.exists() {
            skipped_count += 1;
            continue;
        }

        let target_folder = root_folder.join(&candidate.target_folder);
        if !target_folder.exists() {
            if let Err(err) = fs::create_dir_all(&target_folder) {
                errors.push(format!("{}: {}", candidate.file_name, err));
                skipped_count += 1;
                continue;
            }
            created_folders.push(target_folder.to_string_lossy().to_string());
        }

        let mut target_file = target_folder.join(&candidate.file_name);
        if target_file.exists() {
            let stem = target_file
                .file_stem()
                .and_then(|v| v.to_str())
                .unwrap_or("file");
            let ext = target_file.extension().and_then(|v| v.to_str()).unwrap_or("");
            let suffix = now_ts().unwrap_or(0);
            let replacement = if ext.is_empty() {
                format!("{}_{}", stem, suffix)
            } else {
                format!("{}_{}.{}", stem, suffix, ext)
            };
            target_file = target_folder.join(replacement);
        }

        match fs::rename(&source_path, &target_file) {
            Ok(_) => moved_count += 1,
            Err(err) => {
                skipped_count += 1;
                errors.push(format!("{}: {}", candidate.file_name, err));
            }
        }
    }

    created_folders.sort();
    created_folders.dedup();

    let now = now_ts()?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "INSERT INTO file_grouping_runs (total_candidates, moved_count, skipped_count, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![total_candidates, moved_count, skipped_count, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(FileGroupingResult {
        policy,
        total_candidates,
        moved_count,
        skipped_count,
        created_folders,
        errors,
    })
}
