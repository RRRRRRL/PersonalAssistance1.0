use crate::db::DbState;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::multipart::{Form, Part};
use reqwest::Client;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

// ─── Types ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct VoiceSettings {
    pub stt_provider: String,
    pub stt_base_url: String,
    pub stt_api_key_env: String,
    pub stt_model: String,
    pub tts_provider: String,
    pub tts_base_url: String,
    pub tts_api_key_env: String,
    pub tts_model: String,
    pub tts_voice: String,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VoiceSettingsInput {
    pub stt_provider: Option<String>,
    pub stt_base_url: Option<String>,
    pub stt_api_key_env: Option<String>,
    pub stt_model: Option<String>,
    pub tts_provider: Option<String>,
    pub tts_base_url: Option<String>,
    pub tts_api_key_env: Option<String>,
    pub tts_model: Option<String>,
    pub tts_voice: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VoiceStatus {
    pub settings: VoiceSettings,
    pub stt_api_key_present: bool,
    pub tts_api_key_present: bool,
    pub stt_ready: bool,
    pub tts_ready: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub language: Option<String>,
    pub duration_seconds: Option<f64>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn now_ts() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

fn resolve_env_key(env_name: &str) -> Option<String> {
    env::var(env_name).ok().filter(|v| !v.trim().is_empty())
}

fn normalize_url(base: &str) -> String {
    base.trim().trim_end_matches('/').to_string()
}

fn default_settings() -> VoiceSettings {
    VoiceSettings {
        stt_provider: "openai".to_string(),
        stt_base_url: "https://api.openai.com/v1".to_string(),
        stt_api_key_env: "OPENAI_API_KEY".to_string(),
        stt_model: "whisper-1".to_string(),
        tts_provider: "openai".to_string(),
        tts_base_url: "https://api.openai.com/v1".to_string(),
        tts_api_key_env: "OPENAI_API_KEY".to_string(),
        tts_model: "tts-1".to_string(),
        tts_voice: "nova".to_string(),
        updated_at: 0,
    }
}

fn load_voice_settings(state: State<'_, DbState>) -> Result<VoiceSettings, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.query_row(
        "SELECT stt_provider, stt_base_url, stt_api_key_env, stt_model,
                tts_provider, tts_base_url, tts_api_key_env, tts_model, tts_voice, updated_at
         FROM voice_settings WHERE id = 1",
        [],
        |row| {
            Ok(VoiceSettings {
                stt_provider: row.get(0)?,
                stt_base_url: row.get(1)?,
                stt_api_key_env: row.get(2)?,
                stt_model: row.get(3)?,
                tts_provider: row.get(4)?,
                tts_base_url: row.get(5)?,
                tts_api_key_env: row.get(6)?,
                tts_model: row.get(7)?,
                tts_voice: row.get(8)?,
                updated_at: row.get(9)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())?
    .map(Ok)
    .unwrap_or_else(|| Ok(default_settings()))
}

// ─── Commands ─────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_voice_settings(state: State<'_, DbState>) -> Result<VoiceStatus, String> {
    let settings = load_voice_settings(state)?;
    let stt_api_key_present = resolve_env_key(&settings.stt_api_key_env).is_some();
    let tts_api_key_present = resolve_env_key(&settings.tts_api_key_env).is_some();
    Ok(VoiceStatus {
        stt_ready: stt_api_key_present && !settings.stt_base_url.is_empty(),
        tts_ready: tts_api_key_present && !settings.tts_base_url.is_empty(),
        stt_api_key_present,
        tts_api_key_present,
        settings,
    })
}

#[tauri::command]
pub fn set_voice_settings(
    input: VoiceSettingsInput,
    state: State<'_, DbState>,
) -> Result<VoiceSettings, String> {
    let now = now_ts()?;
    let current = load_voice_settings(state.clone())?;

    let updated = VoiceSettings {
        stt_provider: input.stt_provider
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.stt_provider),
        stt_base_url: input.stt_base_url
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.stt_base_url),
        stt_api_key_env: input.stt_api_key_env
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.stt_api_key_env),
        stt_model: input.stt_model
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.stt_model),
        tts_provider: input.tts_provider
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.tts_provider),
        tts_base_url: input.tts_base_url
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.tts_base_url),
        tts_api_key_env: input.tts_api_key_env
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.tts_api_key_env),
        tts_model: input.tts_model
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.tts_model),
        tts_voice: input.tts_voice
            .map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
            .unwrap_or(current.tts_voice),
        updated_at: now,
    };

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "INSERT INTO voice_settings
            (id, stt_provider, stt_base_url, stt_api_key_env, stt_model,
             tts_provider, tts_base_url, tts_api_key_env, tts_model, tts_voice, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(id) DO UPDATE SET
            stt_provider    = excluded.stt_provider,
            stt_base_url    = excluded.stt_base_url,
            stt_api_key_env = excluded.stt_api_key_env,
            stt_model       = excluded.stt_model,
            tts_provider    = excluded.tts_provider,
            tts_base_url    = excluded.tts_base_url,
            tts_api_key_env = excluded.tts_api_key_env,
            tts_model       = excluded.tts_model,
            tts_voice       = excluded.tts_voice,
            updated_at      = excluded.updated_at",
        params![
            updated.stt_provider, updated.stt_base_url, updated.stt_api_key_env,
            updated.stt_model, updated.tts_provider, updated.tts_base_url,
            updated.tts_api_key_env, updated.tts_model, updated.tts_voice, now,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(updated)
}

// ─── Speech-to-Text (Whisper-compatible) ──────────────────────────────────────

#[tauri::command]
pub async fn transcribe_audio(
    audio_base64: String,
    mime_type: Option<String>,
    state: State<'_, DbState>,
) -> Result<TranscriptionResult, String> {
    let settings = load_voice_settings(state)?;
    let api_key = resolve_env_key(&settings.stt_api_key_env).ok_or_else(|| {
        format!("STT API key not found in env var '{}'", settings.stt_api_key_env)
    })?;

    let audio_bytes = B64.decode(&audio_base64)
        .map_err(|e| format!("Invalid base64 audio: {}", e))?;

    let ext = mime_type
        .as_deref()
        .and_then(|m| {
            if m.contains("webm") { Some("webm") }
            else if m.contains("ogg") { Some("ogg") }
            else if m.contains("wav") { Some("wav") }
            else if m.contains("mp4") { Some("mp4") }
            else if m.contains("mpeg") { Some("mp3") }
            else { None }
        })
        .unwrap_or("webm");

    let filename = format!("audio.{}", ext);
    let endpoint = format!("{}/audio/transcriptions", normalize_url(&settings.stt_base_url));

    let audio_part = Part::bytes(audio_bytes)
        .file_name(filename)
        .mime_str(mime_type.as_deref().unwrap_or("audio/webm"))
        .map_err(|e| e.to_string())?;

    let form = Form::new()
        .part("file", audio_part)
        .text("model", settings.stt_model.clone())
        .text("response_format", "verbose_json");

    let client = Client::new();
    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .multipart(form)
        .send()
        .await
        .map_err(|e| format!("STT request failed: {}", e))?;

    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("STT returned HTTP {}: {}", status.as_u16(), body));
    }

    // Try to parse verbose JSON response; fall back to plain text
    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&body) {
        let text = json_val.get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let language = json_val.get("language")
            .and_then(|v| v.as_str())
            .map(String::from);
        let duration = json_val.get("duration")
            .and_then(|v| v.as_f64());
        Ok(TranscriptionResult { text, language, duration_seconds: duration })
    } else {
        Ok(TranscriptionResult { text: body.trim().to_string(), language: None, duration_seconds: None })
    }
}

// ─── Text-to-Speech (OpenAI TTS-compatible) ───────────────────────────────────

#[tauri::command]
pub async fn synthesize_speech(
    text: String,
    state: State<'_, DbState>,
) -> Result<String, String> {
    let settings = load_voice_settings(state)?;
    let api_key = resolve_env_key(&settings.tts_api_key_env).ok_or_else(|| {
        format!("TTS API key not found in env var '{}'", settings.tts_api_key_env)
    })?;

    if text.trim().is_empty() {
        return Err("No text provided for speech synthesis".to_string());
    }

    let endpoint = format!("{}/audio/speech", normalize_url(&settings.tts_base_url));

    let body = json!({
        "model": settings.tts_model,
        "voice": settings.tts_voice,
        "input": text.trim(),
        "response_format": "mp3"
    });

    let client = Client::new();
    let response = client
        .post(&endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("TTS request failed: {}", e))?;

    let status = response.status();
    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        return Err(format!("TTS returned HTTP {}: {}", status.as_u16(), error_body));
    }

    let audio_bytes = response.bytes().await
        .map_err(|e| format!("Failed to read TTS audio: {}", e))?;

    Ok(B64.encode(&audio_bytes))
}
