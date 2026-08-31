use crate::commands::sensitive::{log_redaction_event, scan_and_redact};
use crate::db::{AgentStateShared, DbState};
use chrono::{Duration, Local};
use reqwest::Client;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::env;
use std::process::{Command as ProcessCommand, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

// ─── Structs ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct OrchestrationSignalInput {
    pub source: String,
    pub content: String,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RelatedFile {
    pub path: String,
    pub name: String,
    pub last_accessed: i64,
    pub access_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrchestrationAction {
    pub id: i64,
    pub action_type: String,
    pub title: String,
    pub due_date: Option<String>,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrchestrationPlan {
    pub created_actions: Vec<OrchestrationAction>,
    pub related_files: Vec<RelatedFile>,
    /// Populated when LangGraph agent was used
    pub graph_path: Option<Vec<String>>,
    pub summary: Option<String>,
    pub used_agent: bool,
    // New: memory + sandbox results
    pub recalled_memories: Vec<Value>,
    pub sandbox_results: Vec<Value>,
    pub memory_stored: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OrchestrationRule {
    pub id: i64,
    pub name: String,
    pub source_pattern: Option<String>,
    pub keyword_pattern: Option<String>,
    pub action_type: String,
    pub title_template: String,
    pub note_template: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrchestrationRuleInput {
    pub name: String,
    pub source_pattern: Option<String>,
    pub keyword_pattern: Option<String>,
    pub action_type: String,
    pub title_template: String,
    pub note_template: Option<String>,
}

// ─── Agent settings ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct OrchestrationAgentSettings {
    pub agent_port: u16,
    pub auto_start: bool,
    pub python_executable: String,
    pub script_dir: String,
    pub use_ai_provider: bool,
    pub max_retries: u8,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OrchestrationAgentSettingsInput {
    pub agent_port: Option<u16>,
    pub auto_start: Option<bool>,
    pub python_executable: Option<String>,
    pub script_dir: Option<String>,
    pub use_ai_provider: Option<bool>,
    pub max_retries: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentStatus {
    pub running: bool,
    pub port: u16,
    pub uptime_ms: Option<u64>,
    pub settings: OrchestrationAgentSettings,
}

/// Response from the LangGraph /orchestrate endpoint
#[derive(Debug, Deserialize)]
struct LangGraphResponse {
    classification: Value,
    entities: Vec<String>,
    actions: Vec<LangGraphAction>,
    context_hints: Vec<String>,
    summary: String,
    graph_path: Vec<String>,
    #[serde(default)]
    recalled_memories: Vec<Value>,
    #[serde(default)]
    sandbox_results: Vec<Value>,
    #[serde(default)]
    memory_stored: bool,
}

#[derive(Debug, Deserialize)]
struct LangGraphAction {
    action_type: String,
    title: String,
    due_date: Option<String>,
    note: Option<String>,
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn now_ts() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn inferred_due_date(content: &str) -> String {
    let normalized = content.to_lowercase();
    let date = if normalized.contains("next week") {
        Local::now().date_naive() + Duration::days(7)
    } else if normalized.contains("tomorrow") {
        Local::now().date_naive() + Duration::days(1)
    } else {
        Local::now().date_naive()
    };
    date.format("%Y-%m-%d").to_string()
}

fn extract_keywords(content: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut keywords = Vec::new();
    for part in content
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
    {
        if part.len() < 4 {
            continue;
        }
        if seen.insert(part.to_string()) {
            keywords.push(part.to_string());
        }
        if keywords.len() >= 8 {
            break;
        }
    }
    keywords
}

fn suggest_related_files_internal(conn: &Connection, query: &str) -> Result<Vec<RelatedFile>, String> {
    let keywords = extract_keywords(query);
    if keywords.is_empty() {
        return Ok(Vec::new());
    }
    let mut file_set = HashSet::new();
    let mut related_files = Vec::new();
    for keyword in keywords {
        let pattern = format!("%{}%", keyword);
        let mut stmt = conn
            .prepare(
                "SELECT path, name, last_accessed, access_count
                 FROM file_history
                 WHERE lower(name) LIKE ?1 OR lower(path) LIKE ?1
                 ORDER BY access_count DESC, last_accessed DESC
                 LIMIT 5",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![pattern], |row| {
                Ok(RelatedFile {
                    path: row.get(0)?,
                    name: row.get(1)?,
                    last_accessed: row.get(2)?,
                    access_count: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?;
        for row in rows {
            let file = row.map_err(|e| e.to_string())?;
            if file_set.insert(file.path.clone()) {
                related_files.push(file);
            }
            if related_files.len() >= 5 {
                return Ok(related_files);
            }
        }
    }
    Ok(related_files)
}

fn insert_action(
    conn: &Connection,
    signal_id: i64,
    action_type: &str,
    title: &str,
    due_date: Option<&str>,
    note: Option<&str>,
    created_at: i64,
) -> Result<OrchestrationAction, String> {
    conn.execute(
        "INSERT INTO orchestration_actions
         (signal_id, action_type, title, due_date, status, note, created_at)
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)",
        params![signal_id, action_type, title, due_date, note, created_at],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    Ok(OrchestrationAction {
        id,
        action_type: action_type.to_string(),
        title: title.to_string(),
        due_date: due_date.map(|v| v.to_string()),
        status: "pending".to_string(),
        note: note.map(|v| v.to_string()),
    })
}

fn matches_rule(
    rule_source: &Option<String>,
    rule_keyword: &Option<String>,
    signal_source: &str,
    signal_content: &str,
) -> bool {
    let source_match = match rule_source {
        Some(pattern) if !pattern.trim().is_empty() => {
            signal_source.contains(&pattern.to_lowercase())
        }
        _ => true,
    };
    let keyword_match = match rule_keyword {
        Some(pattern) if !pattern.trim().is_empty() => {
            signal_content.contains(&pattern.to_lowercase())
        }
        _ => true,
    };
    source_match && keyword_match
}

fn render_template(template: &str, content: &str, due_date: &str) -> String {
    template
        .replace("{content}", content)
        .replace("{due_date}", due_date)
}

fn load_agent_settings(conn: &Connection) -> OrchestrationAgentSettings {
    conn.query_row(
        "SELECT agent_port, auto_start, python_executable, script_dir,
                use_ai_provider, max_retries, updated_at
         FROM orchestration_settings WHERE id = 1",
        [],
        |row| {
            Ok(OrchestrationAgentSettings {
                agent_port: row.get::<_, i64>(0)? as u16,
                auto_start: row.get::<_, i64>(1)? != 0,
                python_executable: row.get(2)?,
                script_dir: row.get(3)?,
                use_ai_provider: row.get::<_, i64>(4)? != 0,
                max_retries: row.get::<_, i64>(5)? as u8,
                updated_at: row.get(6)?,
            })
        },
    )
    .optional()
    .ok()
    .flatten()
    .unwrap_or(OrchestrationAgentSettings {
        agent_port: 8765,
        auto_start: true,
        python_executable: "python".to_string(),
        script_dir: String::new(),
        use_ai_provider: true,
        max_retries: 2,
        updated_at: 0,
    })
}

// ─── Agent health check ───────────────────────────────────────────────────────

async fn check_agent_health(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{}/health", port);
    match Client::new().get(&url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

// ─── LangGraph call ───────────────────────────────────────────────────────────

async fn call_langgraph(
    port: u16,
    source: &str,
    content: &str,
    metadata: Option<&str>,
    rules: &[OrchestrationRule],
) -> Result<LangGraphResponse, String> {
    let url = format!("http://127.0.0.1:{}/orchestrate", port);

    // Build provider config for the agent
    let mut provider = json!({});
    if let Ok(poe_key) = env::var("POE_API_KEY") {
        if !poe_key.trim().is_empty() {
            provider["poe_api_key"] = json!(poe_key);
        }
    }
    if let Ok(ali_key) = env::var("ALIBABA_API_KEY") {
        if !ali_key.trim().is_empty() {
            provider["alibaba_api_key"] = json!(ali_key);
        }
    }

    let rules_json: Vec<Value> = rules
        .iter()
        .map(|r| {
            json!({
                "name": r.name,
                "action_type": r.action_type,
                "source_pattern": r.source_pattern,
                "keyword_pattern": r.keyword_pattern,
            })
        })
        .collect();

    let body = json!({
        "source": source,
        "content": content,
        "metadata": metadata,
        "existing_rules": rules_json,
        "provider": if provider.as_object().map_or(false, |o| !o.is_empty()) {
            Some(provider)
        } else {
            None
        }
    });

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("LangGraph agent unreachable: {}", e))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(format!("LangGraph agent error: {}", err_body));
    }

    resp.json::<LangGraphResponse>()
        .await
        .map_err(|e| format!("Failed to parse LangGraph response: {}", e))
}

// ─── Fallback keyword-based processing ────────────────────────────────────────

fn process_signal_fallback(
    conn: &Connection,
    signal: &OrchestrationSignalInput,
    sanitized_content: &str,
    signal_id: i64,
    now: i64,
) -> Result<Vec<OrchestrationAction>, String> {
    let normalized_content = sanitized_content.to_lowercase();
    let normalized_source = signal.source.to_lowercase();
    let due_date = inferred_due_date(sanitized_content);
    let mut created_actions = Vec::new();

    let mut rule_stmt = conn
        .prepare(
            "SELECT id, name, source_pattern, keyword_pattern, action_type, title_template, note_template, is_active
             FROM orchestration_rules WHERE is_active = 1 ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;

    let rules = rule_stmt
        .query_map([], |row| {
            Ok(OrchestrationRule {
                id: row.get(0)?,
                name: row.get(1)?,
                source_pattern: row.get(2)?,
                keyword_pattern: row.get(3)?,
                action_type: row.get(4)?,
                title_template: row.get(5)?,
                note_template: row.get(6)?,
                is_active: row.get::<_, i64>(7)? > 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    for rule in rules {
        if matches_rule(
            &rule.source_pattern,
            &rule.keyword_pattern,
            &normalized_source,
            &normalized_content,
        ) {
            let title = render_template(&rule.title_template, sanitized_content, &due_date);
            let note = rule
                .note_template
                .as_ref()
                .map(|t| render_template(t, sanitized_content, &due_date));
            created_actions.push(insert_action(
                &conn, signal_id, &rule.action_type, &title, Some(&due_date), note.as_deref(), now,
            )?);
        }
    }

    if created_actions.is_empty()
        && (normalized_content.contains("exam") || normalized_content.contains("test"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "schedule_preparation",
            "Create next-week preparation blocks for upcoming exam",
            Some(&due_date),
            Some("Auto-created by fallback: reserve study sessions and prep notes."),
            now,
        )?);
    }

    if created_actions.is_empty()
        && (normalized_content.contains("coding")
            || normalized_content.contains("bug")
            || normalized_content.contains("feature")
            || normalized_content.contains("docs"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "coding_context",
            "Collect related docs/files and prepare coding notes",
            Some(&due_date),
            Some("Auto-created by fallback: gather references for coding task."),
            now,
        )?);
    }

    if created_actions.is_empty()
        && (normalized_content.contains("whatsapp")
            || normalized_content.contains("message")
            || normalized_content.contains("meeting")
            || normalized_content.contains("call"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "calendar_event",
            "Create calendar follow-up from communication signal",
            Some(&due_date),
            Some("Auto-created by fallback: communication converted into a calendar reminder."),
            now,
        )?);
    }

    if created_actions.is_empty()
        && (normalized_content.contains("url")
            || normalized_content.contains("link")
            || normalized_content.contains("phish")
            || normalized_content.contains("malware")
            || normalized_content.contains("suspicious"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "security_scan",
            "Scan URL for malware/phishing using Security Toolkit",
            Some(&due_date),
            Some("Auto-created by fallback: use the Security Toolkit URL scanner tab to check the link before clicking."),
            now,
        )?);
    }

    if created_actions.is_empty()
        && (normalized_content.contains("password")
            || normalized_content.contains("credential")
            || normalized_content.contains("breach")
            || normalized_content.contains("leak")
            || normalized_content.contains("exposed"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "breach_check",
            "Check credentials and generate secure passwords via Security Toolkit",
            Some(&due_date),
            Some("Auto-created by fallback: use the Breach tab to check email exposure and the Pass tab to generate strong passwords."),
            now,
        )?);
    }

    if created_actions.is_empty()
        && (normalized_content.contains("weather")
            || normalized_content.contains("holiday")
            || normalized_content.contains("forecast"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "weather_check",
            "Check weather and upcoming holidays in Daily Brief",
            Some(&due_date),
            Some("Auto-created by fallback: the Daily Brief widget shows current weather conditions and next public holiday."),
            now,
        )?);
    }

    if created_actions.is_empty()
        && (normalized_content.contains("exchange")
            || normalized_content.contains("currency")
            || normalized_content.contains("rate")
            || normalized_content.contains("convert")
            || normalized_content.contains("define")
            || normalized_content.contains("meaning")
            || normalized_content.contains("definition"))
    {
        created_actions.push(insert_action(
            &conn, signal_id, "currency_lookup",
            "Use Quick Utilities for currency rates or dictionary lookup",
            Some(&due_date),
            Some("Auto-created by fallback: toggle between Currency and Dictionary modes in the Quick Utilities widget."),
            now,
        )?);
    }

    if created_actions.is_empty() {
        created_actions.push(insert_action(
            &conn, signal_id, "generic_follow_up",
            "Review signal and decide next action",
            Some(&due_date),
            Some("No specific rule matched. Manual review requested."),
            now,
        )?);
    }

    Ok(created_actions)
}

// ─── Main orchestration command ───────────────────────────────────────────────

#[tauri::command]
pub async fn process_orchestration_signal(
    signal: OrchestrationSignalInput,
    state: State<'_, DbState>,
    agent: State<'_, AgentStateShared>,
) -> Result<OrchestrationPlan, String> {
    let now = now_ts()?;
    let content_scan = scan_and_redact(&signal.content)?;
    let sanitized_content = content_scan.redacted_text.clone();
    let sanitized_metadata = match signal.metadata {
        Some(meta) => Some(scan_and_redact(&meta)?.redacted_text),
        None => None,
    };

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    log_redaction_event(&conn, "orchestration_signal", &content_scan)?;

    conn.execute(
        "INSERT INTO orchestration_signals (source, content, metadata, occurred_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![signal.source, sanitized_content, sanitized_metadata, now, now],
    )
    .map_err(|e| e.to_string())?;
    let signal_id = conn.last_insert_rowid();

    let settings = load_agent_settings(&conn);
    let agent_port = {
        let ag = agent.lock().map_err(|_| "Agent lock poisoned")?;
        ag.port
    };

    // Gather existing rules for the agent
    let rules_for_agent: Vec<OrchestrationRule> = {
        let mut stmt = conn
            .prepare(
                "SELECT id, name, source_pattern, keyword_pattern, action_type, title_template, note_template, is_active
                 FROM orchestration_rules WHERE is_active = 1 ORDER BY id ASC",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_map([], |row| {
            Ok(OrchestrationRule {
                id: row.get(0)?,
                name: row.get(1)?,
                source_pattern: row.get(2)?,
                keyword_pattern: row.get(3)?,
                action_type: row.get(4)?,
                title_template: row.get(5)?,
                note_template: row.get(6)?,
                is_active: row.get::<_, i64>(7)? > 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect()
    };

    drop(conn); // release lock before async HTTP call

    // Try LangGraph agent first
    let agent_healthy = check_agent_health(agent_port).await;
    let start_ms = now_ms();

    let (created_actions, graph_path, summary, used_agent, recalled_memories, sandbox_results, memory_stored) = if agent_healthy {
        match call_langgraph(agent_port, &signal.source, &sanitized_content, sanitized_metadata.as_deref(), &rules_for_agent).await {
            Ok(lg_resp) => {
                let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
                let mut actions = Vec::new();
                for a in &lg_resp.actions {
                    actions.push(insert_action(
                        &conn,
                        signal_id,
                        &a.action_type,
                        &a.title,
                        a.due_date.as_deref(),
                        a.note.as_deref(),
                        now,
                    )?);
                }
                let elapsed = now_ms() - start_ms;
                let classification_str = serde_json::to_string(&lg_resp.classification).unwrap_or_default();
                let entities_str = serde_json::to_string(&lg_resp.entities).unwrap_or_default();
                let graph_path_str = serde_json::to_string(&lg_resp.graph_path).unwrap_or_default();
                let _ = conn.execute(
                    "INSERT INTO orchestration_graph_runs
                     (signal_content, classification, entities, actions_created, graph_path, duration_ms, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![sanitized_content, classification_str, entities_str, actions.len() as i64, graph_path_str, elapsed as i64, now],
                );
                (actions, Some(lg_resp.graph_path), Some(lg_resp.summary), true,
                 lg_resp.recalled_memories, lg_resp.sandbox_results, lg_resp.memory_stored)
            }
            Err(_) => {
                // Fallback to keyword matching
                let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
                let actions = process_signal_fallback(&conn, &signal, &sanitized_content, signal_id, now)?;
                (actions, None, None, false, Vec::new(), Vec::new(), false)
            }
        }
    } else {
        // Agent not running — use fallback
        let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
        let actions = process_signal_fallback(&conn, &signal, &sanitized_content, signal_id, now)?;
        (actions, None, None, false, Vec::new(), Vec::new(), false)
    };

    let related_files = {
        let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
        suggest_related_files_internal(&conn, &sanitized_content.to_lowercase())?
    };

    Ok(OrchestrationPlan {
        created_actions,
        related_files,
        graph_path,
        summary,
        used_agent,
        recalled_memories,
        sandbox_results,
        memory_stored,
    })
}

// ─── Rule management commands ─────────────────────────────────────────────────

#[tauri::command]
pub fn get_orchestration_rules(state: State<'_, DbState>) -> Result<Vec<OrchestrationRule>, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let mut stmt = conn
        .prepare(
            "SELECT id, name, source_pattern, keyword_pattern, action_type, title_template, note_template, is_active
             FROM orchestration_rules ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rules = stmt
        .query_map([], |row| {
            Ok(OrchestrationRule {
                id: row.get(0)?,
                name: row.get(1)?,
                source_pattern: row.get(2)?,
                keyword_pattern: row.get(3)?,
                action_type: row.get(4)?,
                title_template: row.get(5)?,
                note_template: row.get(6)?,
                is_active: row.get::<_, i64>(7)? > 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    Ok(rules)
}

#[tauri::command]
pub fn add_orchestration_rule(
    rule: OrchestrationRuleInput,
    state: State<'_, DbState>,
) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT INTO orchestration_rules
         (name, source_pattern, keyword_pattern, action_type, title_template, note_template, is_active, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7)",
        params![
            rule.name,
            rule.source_pattern,
            rule.keyword_pattern,
            rule.action_type,
            rule.title_template,
            rule.note_template,
            now
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn set_orchestration_rule_active(
    rule_id: i64,
    is_active: bool,
    state: State<'_, DbState>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "UPDATE orchestration_rules SET is_active = ?1 WHERE id = ?2",
        params![if is_active { 1 } else { 0 }, rule_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Queue management commands ────────────────────────────────────────────────

#[tauri::command]
pub fn get_orchestration_queue(state: State<'_, DbState>) -> Result<Vec<OrchestrationAction>, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let mut stmt = conn
        .prepare(
            "SELECT id, action_type, title, due_date, status, note
             FROM orchestration_actions WHERE status = 'pending'
             ORDER BY created_at DESC LIMIT 25",
        )
        .map_err(|e| e.to_string())?;
    let actions = stmt
        .query_map([], |row| {
            Ok(OrchestrationAction {
                id: row.get(0)?,
                action_type: row.get(1)?,
                title: row.get(2)?,
                due_date: row.get(3)?,
                status: row.get(4)?,
                note: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();
    Ok(actions)
}

#[tauri::command]
pub fn complete_orchestration_action(action_id: i64, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    conn.execute(
        "UPDATE orchestration_actions SET status = 'done' WHERE id = ?1",
        params![action_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn apply_orchestration_action(action_id: i64, state: State<'_, DbState>) -> Result<String, String> {
    let now = now_ts()?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;

    let (action_type, title, due_date): (String, String, Option<String>) = conn
        .query_row(
            "SELECT action_type, title, due_date FROM orchestration_actions WHERE id = ?1",
            params![action_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    if action_type == "calendar_event" || action_type == "schedule_preparation" {
        let date_str = due_date.unwrap_or_else(|| Local::now().date_naive().format("%Y-%m-%d").to_string());
        let (start_time, end_time, deep_work) = if action_type == "schedule_preparation" {
            (format!("{} 19:00", date_str), format!("{} 20:30", date_str), 1_i64)
        } else {
            (format!("{} 10:00", date_str), format!("{} 10:30", date_str), 0_i64)
        };
        conn.execute(
            "INSERT INTO calendar_events
             (title, start_time, end_time, is_deep_work_block, source_action_id, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![title, start_time, end_time, deep_work, action_id, now],
        )
        .map_err(|e| e.to_string())?;
    }

    // Widget-launch actions: log that the user should open the relevant widget
    let widget_action_types = [
        "security_scan", "breach_check", "password_generate",
        "weather_check", "currency_lookup", "dictionary_lookup",
    ];
    if widget_action_types.contains(&action_type.as_str()) {
        let widget_name = match action_type.as_str() {
            "security_scan" | "breach_check" | "password_generate" => "Security Toolkit",
            "weather_check" => "Daily Brief",
            "currency_lookup" | "dictionary_lookup" => "Quick Utilities",
            _ => "Dashboard",
        };
        conn.execute(
            "UPDATE orchestration_actions SET note = COALESCE(note || ' ', '') || ?1 WHERE id = ?2",
            params![format!("→ Open the {} widget to act on this.", widget_name), action_id],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute(
        "UPDATE orchestration_actions SET status = 'done' WHERE id = ?1",
        params![action_id],
    )
    .map_err(|e| e.to_string())?;

    Ok("Action applied".to_string())
}

#[tauri::command]
pub fn apply_orchestration_actions(action_ids: Vec<i64>, state: State<'_, DbState>) -> Result<usize, String> {
    let mut applied = 0_usize;
    for id in action_ids {
        if apply_orchestration_action(id, state.clone()).is_ok() {
            applied += 1;
        }
    }
    Ok(applied)
}

#[tauri::command]
pub fn suggest_related_files(query: String, state: State<'_, DbState>) -> Result<Vec<RelatedFile>, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    suggest_related_files_internal(&conn, &query)
}

// ─── Agent settings commands ──────────────────────────────────────────────────

#[tauri::command]
pub fn get_orchestration_agent_settings(
    state: State<'_, DbState>,
) -> Result<OrchestrationAgentSettings, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    Ok(load_agent_settings(&conn))
}

#[tauri::command]
pub fn set_orchestration_agent_settings(
    input: OrchestrationAgentSettingsInput,
    state: State<'_, DbState>,
) -> Result<OrchestrationAgentSettings, String> {
    let now = now_ts()?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let current = load_agent_settings(&conn);

    let port = input.agent_port.unwrap_or(current.agent_port);
    let auto_start = input.auto_start.unwrap_or(current.auto_start);
    let python_exec = input
        .python_executable
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(current.python_executable);
    let script_dir = input
        .script_dir
        .filter(|v| !v.trim().is_empty())
        .unwrap_or(current.script_dir);
    let use_provider = input.use_ai_provider.unwrap_or(current.use_ai_provider);
    let max_retries = input.max_retries.unwrap_or(current.max_retries);

    conn.execute(
        "INSERT INTO orchestration_settings
            (id, agent_port, auto_start, python_executable, script_dir, use_ai_provider, max_retries, updated_at)
         VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(id) DO UPDATE SET
            agent_port = excluded.agent_port,
            auto_start = excluded.auto_start,
            python_executable = excluded.python_executable,
            script_dir = excluded.script_dir,
            use_ai_provider = excluded.use_ai_provider,
            max_retries = excluded.max_retries,
            updated_at = excluded.updated_at",
        params![
            port as i64,
            if auto_start { 1 } else { 0 },
            python_exec,
            script_dir,
            if use_provider { 1 } else { 0 },
            max_retries as i64,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Update the agent state port
    drop(conn);

    Ok(OrchestrationAgentSettings {
        agent_port: port,
        auto_start,
        python_executable: python_exec,
        script_dir: script_dir,
        use_ai_provider: use_provider,
        max_retries,
        updated_at: now,
    })
}

// ─── Agent lifecycle commands ─────────────────────────────────────────────────

#[tauri::command]
pub async fn start_orchestration_agent(
    state: State<'_, DbState>,
    agent: State<'_, AgentStateShared>,
) -> Result<String, String> {
    // Check if already running
    {
        let ag = agent.lock().map_err(|_| "Agent lock poisoned")?;
        if ag.running {
            let port = ag.port;
            drop(ag);
            if check_agent_health(port).await {
                return Ok(format!("Agent already running on port {}", port));
            }
        }
    }

    let settings = {
        let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
        load_agent_settings(&conn)
    };

    let port = settings.agent_port;
    let python = &settings.python_executable;

    // Resolve script directory
    let script_dir = if settings.script_dir.is_empty() {
        // Default: relative to the app's resource directory
        let exe_dir = env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|pp| pp.to_path_buf()))
            .unwrap_or_else(|| env::current_dir().unwrap_or_default());
        let mut dir = exe_dir.clone();
        // Try common locations
        let candidates = [
            dir.join("scripts").join("orchestrator_agent"),
            dir.join("..").join("scripts").join("orchestrator_agent"),
            dir.join("..").join("..").join("scripts").join("orchestrator_agent"),
        ];
        let mut found = None;
        for c in &candidates {
            if c.join("main.py").exists() {
                found = Some(c.parent().unwrap().to_path_buf());
                break;
            }
        }
        found.unwrap_or_else(|| {
            // Last resort: look from workspace root
            let mut d = env::current_dir().unwrap_or_default();
            d.push("scripts");
            d
        })
    } else {
        std::path::PathBuf::from(&settings.script_dir)
    };

    let main_py = script_dir.join("orchestrator_agent").join("main.py");
    if !main_py.exists() {
        return Err(format!(
            "main.py not found at {:?}. Set script_dir in agent settings.",
            main_py
        ));
    }

    let mut cmd = ProcessCommand::new(python);
    cmd.arg(main_py.to_string_lossy().to_string())
        .env("ORCHESTRATOR_PORT", port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Forward API keys
    for key in &["POE_API_KEY", "ALIBABA_API_KEY", "OPENAI_API_KEY", "E2B_API_KEY", "QDRANT_STORAGE_PATH", "LANGFUSE_PUBLIC_KEY", "LANGFUSE_SECRET_KEY", "LANGFUSE_HOST"] {
        if let Ok(val) = env::var(key) {
            cmd.env(key, val);
        }
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn Python agent: {}", e))?;

    {
        let mut ag = agent.lock().map_err(|_| "Agent lock poisoned")?;
        ag.child = Some(child);
        ag.port = port;
        ag.running = true;
    }

    // Wait for the agent to become healthy (up to 10 seconds)
    for _ in 0..20 {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        if check_agent_health(port).await {
            return Ok(format!("LangGraph agent started on port {}", port));
        }
    }

    // Even if health check hasn't passed yet, the process may still be starting
    Ok(format!(
        "Agent process spawned on port {} (still warming up...)",
        port
    ))
}

#[tauri::command]
pub async fn stop_orchestration_agent(
    agent: State<'_, AgentStateShared>,
) -> Result<String, String> {
    let mut ag = agent.lock().map_err(|_| "Agent lock poisoned")?;

    if let Some(mut child) = ag.child.take() {
        let _ = child.kill();
        let _ = child.wait();
    }

    ag.running = false;
    Ok("Agent stopped".to_string())
}

#[tauri::command]
pub async fn get_orchestration_agent_status(
    state: State<'_, DbState>,
    agent: State<'_, AgentStateShared>,
) -> Result<AgentStatus, String> {
    let ag = agent.lock().map_err(|_| "Agent lock poisoned")?;
    let port = ag.port;
    let running_flag = ag.running;
    drop(ag);

    let healthy = check_agent_health(port).await;
    let settings = {
        let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
        load_agent_settings(&conn)
    };

    Ok(AgentStatus {
        running: healthy,
        port,
        uptime_ms: None, // Would need start timestamp tracking
        settings,
    })
}

// ─── Vector Memory + Sandbox + Capabilities commands ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorMemoryHit {
    pub content: Value,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorMemoryResult {
    pub available: bool,
    pub results: Vec<VectorMemoryHit>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VectorMemoryStats {
    pub available: bool,
    pub point_count: u64,
    pub vector_count: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SandboxResult {
    pub available: bool,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub error: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IntegrationCapabilities {
    pub qdrant: bool,
    pub e2b: bool,
    pub data_juicer: bool,
    pub langfuse: bool,
}

fn agent_base_url(agent: &State<'_, AgentStateShared>) -> Result<String, String> {
    let ag = agent.lock().map_err(|_| "Agent lock poisoned")?;
    Ok(format!("http://127.0.0.1:{}", ag.port))
}

#[tauri::command]
pub async fn search_vector_memories(
    query: String,
    limit: Option<u32>,
    agent: State<'_, AgentStateShared>,
) -> Result<VectorMemoryResult, String> {
    let base = agent_base_url(&agent)?;
    let url = format!("{}/memory/search?query={}&limit={}", base, urlencoding(&query), limit.unwrap_or(5));
    let client = Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e| e.to_string())?;
    let resp = client.post(&url).send().await.map_err(|e| e.to_string())?;
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    let available = body["available"].as_bool().unwrap_or(false);
    let message = body["message"].as_str().map(|s| s.to_string());
    let results: Vec<VectorMemoryHit> = body["results"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .map(|r| VectorMemoryHit {
            content: r["content"].clone(),
            score: r["score"].as_f64().unwrap_or(0.0),
        })
        .collect();
    Ok(VectorMemoryResult { available, results, message })
}

#[tauri::command]
pub async fn get_vector_memory_stats(
    agent: State<'_, AgentStateShared>,
) -> Result<VectorMemoryStats, String> {
    let base = agent_base_url(&agent)?;
    let url = format!("{}/memory/stats", base);
    let client = Client::builder().timeout(std::time::Duration::from_secs(10)).build().map_err(|e| e.to_string())?;
    match client.get(&url).send().await {
        Ok(resp) => {
            let body: Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(VectorMemoryStats {
                available: body["available"].as_bool().unwrap_or(false),
                point_count: body["point_count"].as_u64().unwrap_or(0),
                vector_count: body["vector_count"].as_u64().unwrap_or(0),
                message: body["message"].as_str().map(|s| s.to_string()),
            })
        }
        Err(_) => Ok(VectorMemoryStats { available: false, point_count: 0, vector_count: 0, message: Some("Agent unreachable".into()) }),
    }
}

#[tauri::command]
pub async fn run_sandbox_code(
    code: String,
    agent: State<'_, AgentStateShared>,
) -> Result<SandboxResult, String> {
    let base = agent_base_url(&agent)?;
    let url = format!("{}/sandbox/run?code={}", base, urlencoding(&code));
    let client = Client::builder().timeout(std::time::Duration::from_secs(60)).build().map_err(|e| e.to_string())?;
    match client.post(&url).send().await {
        Ok(resp) => {
            let body: Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(SandboxResult {
                available: body["available"].as_bool().unwrap_or(false),
                stdout: body["stdout"].as_str().map(|s| s.to_string()),
                stderr: body["stderr"].as_str().map(|s| s.to_string()),
                error: body["error"].as_str().map(|s| s.to_string()),
                message: body["message"].as_str().map(|s| s.to_string()),
            })
        }
        Err(_) => Ok(SandboxResult { available: false, stdout: None, stderr: None, error: None, message: Some("Agent unreachable".into()) }),
    }
}

#[tauri::command]
pub async fn get_integration_capabilities(
    agent: State<'_, AgentStateShared>,
) -> Result<IntegrationCapabilities, String> {
    let base = agent_base_url(&agent)?;
    let url = format!("{}/capabilities", base);
    let client = Client::builder().timeout(std::time::Duration::from_secs(5)).build().map_err(|e| e.to_string())?;
    match client.get(&url).send().await {
        Ok(resp) => {
            let body: Value = resp.json().await.map_err(|e| e.to_string())?;
            let integrations = &body["integrations"];
            Ok(IntegrationCapabilities {
                qdrant: integrations["qdrant"].as_bool().unwrap_or(false),
                e2b: integrations["e2b"].as_bool().unwrap_or(false),
                data_juicer: integrations["data_juicer"].as_bool().unwrap_or(false),
                langfuse: integrations["langfuse"].as_bool().unwrap_or(false),
            })
        }
        Err(_) => Ok(IntegrationCapabilities { qdrant: false, e2b: false, data_juicer: false, langfuse: false }),
    }
}

/// Simple percent-encoding for query params (spaces + special chars)
fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u32),
        })
        .collect()
}
