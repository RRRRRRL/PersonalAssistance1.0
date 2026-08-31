use crate::db::DbState;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct YouTubeMusicSettings {
    pub provider_base_url: String,
    pub provider_api_key_env: Option<String>,
    pub preferred_playlist_id: Option<String>,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct YouTubeMusicSettingsInput {
    pub provider_base_url: Option<String>,
    pub provider_api_key_env: Option<String>,
    pub preferred_playlist_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct YouTubeMusicStatus {
    pub settings: YouTubeMusicSettings,
    pub provider_configured: bool,
    pub api_key_present: bool,
    pub provider_reachable: bool,
    pub authenticated: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeMusicPlaylist {
    pub title: String,
    pub playlist_id: String,
    pub track_count: Option<i64>,
    pub privacy: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeMusicSearchResult {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub item_type: String,
    pub video_id: Option<String>,
    pub browse_id: Option<String>,
    pub url: Option<String>,
}

fn now_ts() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

fn load_settings(state: State<'_, DbState>) -> Result<YouTubeMusicSettings, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.query_row(
        "SELECT provider_base_url, provider_api_key_env, preferred_playlist_id, updated_at
         FROM youtube_music_settings
         WHERE id = 1",
        [],
        |row| {
            Ok(YouTubeMusicSettings {
                provider_base_url: row.get(0)?,
                provider_api_key_env: row.get(1)?,
                preferred_playlist_id: row.get(2)?,
                updated_at: row.get(3)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())?
    .map(Ok)
    .unwrap_or_else(|| {
        Ok(YouTubeMusicSettings {
            provider_base_url: "https://api.poe.com/v1/music".to_string(),
            provider_api_key_env: Some("POE_API_KEY".to_string()),
            preferred_playlist_id: None,
            updated_at: 0,
        })
    })
}

fn resolve_api_key(settings: &YouTubeMusicSettings) -> Option<String> {
    settings
        .provider_api_key_env
        .as_ref()
        .and_then(|key_name| env::var(key_name).ok())
        .filter(|value| !value.trim().is_empty())
}

fn normalize_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

async fn call_provider(
    settings: &YouTubeMusicSettings,
    operation: &str,
    payload: Value,
) -> Result<Value, String> {
    let api_key = resolve_api_key(settings).ok_or("Provider API key env is not set")?;
    let base_url = normalize_base_url(&settings.provider_base_url);
    if base_url.is_empty() {
        return Err("Provider base URL is empty".to_string());
    }

    let client = reqwest::Client::new();
    let response = client
        .post(&base_url)
        .bearer_auth(api_key)
        .json(&json!({
            "operation": operation,
            "payload": payload,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = response.status();
    let body = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("Provider request failed: {} {}", status.as_u16(), body));
    }

    serde_json::from_str(&body).map_err(|e| e.to_string())
}

fn fallback_playlists(settings: &YouTubeMusicSettings) -> Vec<YouTubeMusicPlaylist> {
    let mut playlists = vec![
        YouTubeMusicPlaylist {
            title: "Focus Mix".to_string(),
            playlist_id: settings
                .preferred_playlist_id
                .clone()
                .unwrap_or_else(|| "focus_mix".to_string()),
            track_count: Some(0),
            privacy: Some("private".to_string()),
            url: "https://music.youtube.com".to_string(),
        },
        YouTubeMusicPlaylist {
            title: "Deep Work".to_string(),
            playlist_id: "deep_work".to_string(),
            track_count: Some(0),
            privacy: Some("private".to_string()),
            url: "https://music.youtube.com".to_string(),
        },
    ];

    playlists.sort_by(|a, b| a.title.cmp(&b.title));
    playlists
}

fn parse_playlists(value: Value) -> Vec<YouTubeMusicPlaylist> {
    let rows = if let Some(array) = value.as_array() {
        array.clone()
    } else {
        value
            .get("playlists")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };

    rows
        .into_iter()
        .map(|item| YouTubeMusicPlaylist {
            title: item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Untitled Playlist")
                .to_string(),
            playlist_id: item
                .get("playlist_id")
                .or_else(|| item.get("playlistId"))
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            track_count: item.get("track_count").and_then(Value::as_i64),
            privacy: item
                .get("privacy")
                .and_then(Value::as_str)
                .map(str::to_string),
            url: item
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or("https://music.youtube.com")
                .to_string(),
        })
        .collect()
}

fn parse_search_results(value: Value) -> Vec<YouTubeMusicSearchResult> {
    let rows = if let Some(array) = value.as_array() {
        array.clone()
    } else {
        value
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };

    rows
        .into_iter()
        .map(|item| YouTubeMusicSearchResult {
            title: item
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or("Unknown Track")
                .to_string(),
            artist: item
                .get("artist")
                .and_then(Value::as_str)
                .map(str::to_string),
            album: item
                .get("album")
                .and_then(Value::as_str)
                .map(str::to_string),
            item_type: item
                .get("item_type")
                .or_else(|| item.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("song")
                .to_string(),
            video_id: item
                .get("video_id")
                .or_else(|| item.get("videoId"))
                .and_then(Value::as_str)
                .map(str::to_string),
            browse_id: item
                .get("browse_id")
                .or_else(|| item.get("browseId"))
                .and_then(Value::as_str)
                .map(str::to_string),
            url: item
                .get("url")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
        .collect()
}

fn fallback_search_results(query: &str) -> Vec<YouTubeMusicSearchResult> {
    let encoded = query.trim().replace(' ', "+");
    vec![YouTubeMusicSearchResult {
        title: format!("Search YouTube Music for '{}'", query.trim()),
        artist: Some("YouTube Music".to_string()),
        album: None,
        item_type: "shortcut".to_string(),
        video_id: None,
        browse_id: None,
        url: Some(format!("https://music.youtube.com/search?q={}", encoded)),
    }]
}

#[tauri::command]
pub fn save_youtube_music_settings(
    input: YouTubeMusicSettingsInput,
    state: State<'_, DbState>,
) -> Result<YouTubeMusicSettings, String> {
    let now = now_ts()?;
    let current = load_settings(state.clone())?;

    let updated = YouTubeMusicSettings {
        provider_base_url: input
            .provider_base_url
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or(current.provider_base_url),
        provider_api_key_env: input
            .provider_api_key_env
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or(current.provider_api_key_env),
        preferred_playlist_id: input
            .preferred_playlist_id
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .or(current.preferred_playlist_id),
        updated_at: now,
    };

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "INSERT INTO youtube_music_settings (id, provider_base_url, provider_api_key_env, preferred_playlist_id, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET
            provider_base_url = excluded.provider_base_url,
            provider_api_key_env = excluded.provider_api_key_env,
            preferred_playlist_id = excluded.preferred_playlist_id,
            updated_at = excluded.updated_at",
        params![
            updated.provider_base_url,
            updated.provider_api_key_env,
            updated.preferred_playlist_id,
            updated.updated_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

#[tauri::command]
pub async fn get_youtube_music_status(
    state: State<'_, DbState>,
) -> Result<YouTubeMusicStatus, String> {
    let settings = load_settings(state)?;
    let provider_configured = !normalize_base_url(&settings.provider_base_url).is_empty();
    let api_key_present = resolve_api_key(&settings).is_some();

    if !provider_configured {
        return Ok(YouTubeMusicStatus {
            settings,
            provider_configured,
            api_key_present,
            provider_reachable: false,
            authenticated: false,
            error: Some("Provider base URL is not configured".to_string()),
        });
    }

    if !api_key_present {
        return Ok(YouTubeMusicStatus {
            settings,
            provider_configured,
            api_key_present,
            provider_reachable: false,
            authenticated: false,
            error: Some("Provider API key is missing from environment".to_string()),
        });
    }

    match call_provider(&settings, "health", json!({ "provider": "youtube_music" })).await {
        Ok(_) => Ok(YouTubeMusicStatus {
            settings,
            provider_configured,
            api_key_present,
            provider_reachable: true,
            authenticated: true,
            error: None,
        }),
        Err(error) => Ok(YouTubeMusicStatus {
            settings,
            provider_configured,
            api_key_present,
            provider_reachable: false,
            authenticated: false,
            error: Some(error),
        }),
    }
}

#[tauri::command]
pub async fn get_youtube_music_playlists(
    state: State<'_, DbState>,
) -> Result<Vec<YouTubeMusicPlaylist>, String> {
    let settings = load_settings(state)?;

    if resolve_api_key(&settings).is_none() {
        return Ok(fallback_playlists(&settings));
    }

    match call_provider(
        &settings,
        "youtube_music_playlists",
        json!({
            "preferred_playlist_id": settings.preferred_playlist_id,
        }),
    )
    .await
    {
        Ok(value) => {
            let parsed = parse_playlists(value);
            if parsed.is_empty() {
                Ok(fallback_playlists(&settings))
            } else {
                Ok(parsed)
            }
        }
        Err(_) => Ok(fallback_playlists(&settings)),
    }
}

#[tauri::command]
pub async fn search_youtube_music(
    query: String,
    state: State<'_, DbState>,
) -> Result<Vec<YouTubeMusicSearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }

    let settings = load_settings(state)?;

    if resolve_api_key(&settings).is_none() {
        return Ok(fallback_search_results(&query));
    }

    match call_provider(
        &settings,
        "youtube_music_search",
        json!({ "query": query.trim() }),
    )
    .await
    {
        Ok(value) => {
            let parsed = parse_search_results(value);
            if parsed.is_empty() {
                Ok(fallback_search_results(&query))
            } else {
                Ok(parsed)
            }
        }
        Err(_) => Ok(fallback_search_results(&query)),
    }
}