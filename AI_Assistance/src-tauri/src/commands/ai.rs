use crate::commands::sensitive::{log_redaction_event, scan_and_redact};
use crate::db::DbState;
use futures::StreamExt;
use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;
use tauri::State;

// ─── Provider settings ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct AiProviderSettings {
    pub active_provider: String,
    pub poe_base_url: String,
    pub poe_api_key_env: String,
    pub poe_model: String,
    pub alibaba_base_url: String,
    pub alibaba_api_key_env: String,
    pub alibaba_model: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AiProviderSettingsInput {
    pub active_provider: Option<String>,
    pub poe_base_url: Option<String>,
    pub poe_api_key_env: Option<String>,
    pub poe_model: Option<String>,
    pub alibaba_base_url: Option<String>,
    pub alibaba_api_key_env: Option<String>,
    pub alibaba_model: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AiProviderStatus {
    pub settings: AiProviderSettings,
    pub poe_api_key_present: bool,
    pub alibaba_api_key_present: bool,
    pub active_provider_label: String,
    pub active_model: String,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn now_ts() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

fn now_ns() -> Result<u128, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos())
}

fn load_provider_settings(state: State<'_, DbState>) -> Result<AiProviderSettings, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.query_row(
        "SELECT active_provider, poe_base_url, poe_api_key_env, poe_model,
                alibaba_base_url, alibaba_api_key_env, alibaba_model, updated_at
         FROM ai_provider_settings WHERE id = 1",
        [],
        |row| {
            Ok(AiProviderSettings {
                active_provider: row.get(0)?,
                poe_base_url: row.get(1)?,
                poe_api_key_env: row.get(2)?,
                poe_model: row.get(3)?,
                alibaba_base_url: row.get(4)?,
                alibaba_api_key_env: row.get(5)?,
                alibaba_model: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())?
    .map(Ok)
    .unwrap_or_else(|| {
        Ok(AiProviderSettings {
            active_provider: "poe".to_string(),
            poe_base_url: "https://api.poe.com/v1".to_string(),
            poe_api_key_env: "POE_API_KEY".to_string(),
            poe_model: "GPT-4o".to_string(),
            alibaba_base_url: "https://dashscope-intl.aliyuncs.com/compatible-mode/v1".to_string(),
            alibaba_api_key_env: "ALIBABA_API_KEY".to_string(),
            alibaba_model: "qwen-plus".to_string(),
            updated_at: 0,
        })
    })
}

fn resolve_env_key(env_name: &str) -> Option<String> {
    env::var(env_name)
        .ok()
        .filter(|v| !v.trim().is_empty())
}

fn active_base_url_and_key(settings: &AiProviderSettings) -> (String, String, String, String) {
    if settings.active_provider == "alibaba" {
        (
            settings.alibaba_base_url.clone(),
            settings.alibaba_api_key_env.clone(),
            settings.alibaba_model.clone(),
            "Alibaba Qwen".to_string(),
        )
    } else {
        (
            settings.poe_base_url.clone(),
            settings.poe_api_key_env.clone(),
            settings.poe_model.clone(),
            "Poe".to_string(),
        )
    }
}

fn normalize_url(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

// ─── Provider settings commands ───────────────────────────────────────────────

#[tauri::command]
pub fn get_ai_provider_settings(
    state: State<'_, DbState>,
) -> Result<AiProviderStatus, String> {
    let settings = load_provider_settings(state)?;
    let poe_api_key_present = resolve_env_key(&settings.poe_api_key_env).is_some();
    let alibaba_api_key_present = resolve_env_key(&settings.alibaba_api_key_env).is_some();
    let (_, _, model, label) = active_base_url_and_key(&settings);
    Ok(AiProviderStatus {
        active_provider_label: label,
        active_model: model,
        poe_api_key_present,
        alibaba_api_key_present,
        settings,
    })
}

#[tauri::command]
pub fn set_ai_provider_settings(
    input: AiProviderSettingsInput,
    state: State<'_, DbState>,
) -> Result<AiProviderSettings, String> {
    let now = now_ts()?;
    let current = load_provider_settings(state.clone())?;

    let active_provider = input
        .active_provider
        .map(|v| v.trim().to_lowercase())
        .filter(|v| v == "poe" || v == "alibaba")
        .unwrap_or(current.active_provider);

    let updated = AiProviderSettings {
        active_provider,
        poe_base_url: input
            .poe_base_url
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(current.poe_base_url),
        poe_api_key_env: input
            .poe_api_key_env
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(current.poe_api_key_env),
        poe_model: input
            .poe_model
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(current.poe_model),
        alibaba_base_url: input
            .alibaba_base_url
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(current.alibaba_base_url),
        alibaba_api_key_env: input
            .alibaba_api_key_env
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(current.alibaba_api_key_env),
        alibaba_model: input
            .alibaba_model
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .unwrap_or(current.alibaba_model),
        updated_at: now,
    };

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "INSERT INTO ai_provider_settings
            (id, active_provider, poe_base_url, poe_api_key_env, poe_model,
             alibaba_base_url, alibaba_api_key_env, alibaba_model, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
            active_provider   = excluded.active_provider,
            poe_base_url      = excluded.poe_base_url,
            poe_api_key_env   = excluded.poe_api_key_env,
            poe_model         = excluded.poe_model,
            alibaba_base_url  = excluded.alibaba_base_url,
            alibaba_api_key_env = excluded.alibaba_api_key_env,
            alibaba_model     = excluded.alibaba_model,
            updated_at        = excluded.updated_at",
        params![
            updated.active_provider,
            updated.poe_base_url,
            updated.poe_api_key_env,
            updated.poe_model,
            updated.alibaba_base_url,
            updated.alibaba_api_key_env,
            updated.alibaba_model,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

// ─── Session commands ─────────────────────────────────────────────────────────

#[tauri::command]
pub fn begin_ai_stream_session(
    prompt: String,
    state: State<'_, DbState>,
) -> Result<String, String> {
    let now = now_ts()?;
    let session_id = format!("session-{}", now_ns()?);
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let redaction_result = scan_and_redact(&prompt)?;
    log_redaction_event(&conn, "ai_stream_session", &redaction_result)?;

    conn.execute(
        "INSERT INTO ai_stream_sessions
            (session_id, prompt, total_tokens, created_at, updated_at)
         VALUES (?1, ?2, 0, ?3, ?3)",
        params![session_id, redaction_result.redacted_text, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(session_id)
}

#[tauri::command]
pub fn save_ai_token_batch(
    session_id: String,
    batch_index: i64,
    tokens: String,
    token_count: i64,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let now = now_ts()?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;

    let existing_token_count: i64 = conn
        .query_row(
            "SELECT token_count FROM ai_token_batches
             WHERE session_id = ?1 AND batch_index = ?2",
            params![session_id, batch_index],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?
        .unwrap_or(0);

    conn.execute(
        "INSERT OR REPLACE INTO ai_token_batches
            (session_id, batch_index, tokens, token_count, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![session_id, batch_index, tokens, token_count, now],
    )
    .map_err(|e| e.to_string())?;

    let delta = token_count - existing_token_count;
    conn.execute(
        "UPDATE ai_stream_sessions
         SET total_tokens = total_tokens + ?1, updated_at = ?2
         WHERE session_id = ?3",
        params![delta, now, session_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn finalize_ai_stream_session(
    session_id: String,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let now = now_ts()?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;

    conn.execute(
        "UPDATE ai_stream_sessions
         SET completed_at = ?1, updated_at = ?1
         WHERE session_id = ?2",
        params![now, session_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

// ─── Streaming command (Poe + Alibaba, OpenAI-compatible SSE) ─────────────────

#[derive(Deserialize)]
struct ChatChunkDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatChunkChoice {
    delta: Option<ChatChunkDelta>,
}

#[derive(Deserialize)]
struct ChatChunk {
    choices: Option<Vec<ChatChunkChoice>>,
}

fn parse_sse_delta(line: &str) -> Option<String> {
    let payload = line.strip_prefix("data: ")?.trim();
    if payload == "[DONE]" {
        return None;
    }
    serde_json::from_str::<ChatChunk>(payload)
        .ok()
        .and_then(|chunk| chunk.choices?.into_iter().next())
        .and_then(|choice| choice.delta?.content)
}

#[tauri::command]
pub async fn stream_ai_response(
    prompt: String,
    on_token: Channel<String>,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let redaction_result = scan_and_redact(&prompt)?;
    let settings = load_provider_settings(state)?;
    let (base_url, api_key_env, model, _label) = active_base_url_and_key(&settings);

    let api_key = resolve_env_key(&api_key_env).ok_or_else(|| {
        format!(
            "API key not found in environment variable '{}'. Set it before streaming.",
            api_key_env
        )
    })?;

    let endpoint = format!("{}/chat/completions", normalize_url(&base_url));

    let body = json!({
        "model": model,
        "stream": true,
        "messages": [
            {"role": "user", "content": redaction_result.redacted_text}
        ]
    });

    let client = Client::new();
    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request to {} failed: {}", endpoint, e))?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Provider returned HTTP {}: {}",
            status.as_u16(),
            error_body
        ));
    }

    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let bytes = chunk_result.map_err(|e| format!("Stream read error: {}", e))?;
        buffer.push_str(&String::from_utf8_lossy(&bytes));

        while let Some(newline_pos) = buffer.find('\n') {
            let line = buffer[..newline_pos].trim().to_string();
            buffer.drain(..=newline_pos);

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(token) = parse_sse_delta(&line) {
                on_token
                    .send(token)
                    .map_err(|e| format!("Channel send error: {}", e))?;
            }
        }
    }

    // Flush any remaining buffered line
    let remaining = buffer.trim();
    if !remaining.is_empty() && !remaining.starts_with(':') {
        if let Some(token) = parse_sse_delta(remaining) {
            on_token
                .send(token)
                .map_err(|e| format!("Channel send error: {}", e))?;
        }
    }

    Ok(())
}
