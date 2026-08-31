use dirs::data_local_dir;
use rusqlite::{Connection, Error, Result};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::process::Child;

pub struct DbState(pub Mutex<Connection>);

/// Holds the sidecar child process handle and runtime status.
pub struct AgentState {
    pub child: Option<Child>,
    pub port: u16,
    pub running: bool,
}

impl AgentState {
    pub fn new(port: u16) -> Self {
        Self { child: None, port, running: false }
    }
}

pub type AgentStateShared = Arc<Mutex<AgentState>>;

pub fn init_db() -> Result<Connection> {
    let mut db_dir: PathBuf = data_local_dir().unwrap_or(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    db_dir.push("ai_assistance");

    fs::create_dir_all(&db_dir).map_err(|_| Error::InvalidPath(db_dir.clone()))?;

    let db_path = db_dir.join("file_history.db");
    let conn = Connection::open(db_path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_history (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE,
            name TEXT,
            last_accessed INTEGER,
            access_count INTEGER DEFAULT 1
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_grouping_policies (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            root_folder_name TEXT NOT NULL DEFAULT 'AI_AutoGroup',
            older_than_days INTEGER NOT NULL DEFAULT 90,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS file_grouping_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            total_candidates INTEGER NOT NULL,
            moved_count INTEGER NOT NULL,
            skipped_count INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orchestration_signals (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT,
            occurred_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orchestration_actions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            signal_id INTEGER,
            action_type TEXT NOT NULL,
            title TEXT NOT NULL,
            due_date TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            note TEXT,
            created_at INTEGER NOT NULL,
            FOREIGN KEY(signal_id) REFERENCES orchestration_signals(id)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orchestration_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            source_pattern TEXT,
            keyword_pattern TEXT,
            action_type TEXT NOT NULL,
            title_template TEXT NOT NULL,
            note_template TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS calendar_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            is_deep_work_block INTEGER NOT NULL DEFAULT 0,
            source_action_id INTEGER,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_stream_sessions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT UNIQUE NOT NULL,
            prompt TEXT NOT NULL,
            total_tokens INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            completed_at INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_token_batches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            batch_index INTEGER NOT NULL,
            tokens TEXT NOT NULL,
            token_count INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            UNIQUE(session_id, batch_index)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS purchase_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            item_name TEXT NOT NULL,
            category TEXT NOT NULL,
            amount REAL NOT NULL,
            payment_method TEXT NOT NULL,
            card_name TEXT,
            purchased_at TEXT NOT NULL,
            note TEXT,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS credit_card_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_name TEXT UNIQUE NOT NULL,
            statement_balance REAL NOT NULL,
            credit_limit REAL NOT NULL,
            minimum_due REAL,
            due_date TEXT,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS bank_accounts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account_name TEXT UNIQUE NOT NULL,
            current_balance REAL NOT NULL,
            available_balance REAL,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS system_alerts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            alert_key TEXT UNIQUE NOT NULL,
            title TEXT NOT NULL,
            detail TEXT NOT NULL,
            severity TEXT NOT NULL,
            source TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'active',
            first_triggered_at INTEGER NOT NULL,
            last_triggered_at INTEGER NOT NULL,
            occurrences INTEGER NOT NULL DEFAULT 1,
            acknowledged_at INTEGER
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS sensitive_redaction_events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source TEXT NOT NULL,
            redaction_count INTEGER NOT NULL,
            detection_types TEXT NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS ai_provider_settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            active_provider TEXT NOT NULL DEFAULT 'poe',
            poe_base_url TEXT NOT NULL DEFAULT 'https://api.poe.com/v1',
            poe_api_key_env TEXT NOT NULL DEFAULT 'POE_API_KEY',
            poe_model TEXT NOT NULL DEFAULT 'GPT-4o',
            alibaba_base_url TEXT NOT NULL DEFAULT 'https://dashscope-intl.aliyuncs.com/compatible-mode/v1',
            alibaba_api_key_env TEXT NOT NULL DEFAULT 'ALIBABA_API_KEY',
            alibaba_model TEXT NOT NULL DEFAULT 'qwen-plus',
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    // Detect legacy youtube_music_settings columns BEFORE creating the new schema
    let has_legacy_col = conn
        .prepare("SELECT python_executable FROM youtube_music_settings LIMIT 0")
        .is_ok();
    if has_legacy_col {
        let _ = conn.execute_batch(
            "ALTER TABLE youtube_music_settings RENAME COLUMN python_executable TO provider_base_url;
             ALTER TABLE youtube_music_settings RENAME COLUMN auth_json_path TO provider_api_key_env;",
        );
    }

    conn.execute(
        "CREATE TABLE IF NOT EXISTS youtube_music_settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            provider_base_url TEXT NOT NULL DEFAULT 'https://api.poe.com/v1/music',
            provider_api_key_env TEXT DEFAULT 'POE_API_KEY',
            preferred_playlist_id TEXT,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS voice_settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            stt_provider TEXT NOT NULL DEFAULT 'openai',
            stt_base_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
            stt_api_key_env TEXT NOT NULL DEFAULT 'OPENAI_API_KEY',
            stt_model TEXT NOT NULL DEFAULT 'whisper-1',
            tts_provider TEXT NOT NULL DEFAULT 'openai',
            tts_base_url TEXT NOT NULL DEFAULT 'https://api.openai.com/v1',
            tts_api_key_env TEXT NOT NULL DEFAULT 'OPENAI_API_KEY',
            tts_model TEXT NOT NULL DEFAULT 'tts-1',
            tts_voice TEXT NOT NULL DEFAULT 'nova',
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orchestration_settings (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            agent_port INTEGER NOT NULL DEFAULT 8765,
            auto_start INTEGER NOT NULL DEFAULT 1,
            python_executable TEXT NOT NULL DEFAULT 'python',
            script_dir TEXT NOT NULL DEFAULT '',
            use_ai_provider INTEGER NOT NULL DEFAULT 1,
            max_retries INTEGER NOT NULL DEFAULT 2,
            updated_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS orchestration_graph_runs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            signal_content TEXT NOT NULL,
            classification TEXT NOT NULL,
            entities TEXT NOT NULL,
            actions_created INTEGER NOT NULL DEFAULT 0,
            graph_path TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            created_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS vector_memory_metadata (
            id TEXT PRIMARY KEY,
            signal_id INTEGER,
            content_hash TEXT,
            intent TEXT,
            urgency TEXT,
            stored_at INTEGER NOT NULL,
            access_count INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(signal_id) REFERENCES orchestration_signals(id)
        )",
        [],
    )?;

    // ── Market Data Engine tables (inspired by NautilusTrader DataEngine) ──

    conn.execute(
        "CREATE TABLE IF NOT EXISTS watchlist (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL DEFAULT '',
            asset_class TEXT NOT NULL DEFAULT 'equity',
            venue TEXT NOT NULL DEFAULT 'yahoo',
            notes TEXT,
            is_active INTEGER NOT NULL DEFAULT 1,
            added_at INTEGER NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS price_cache (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL,
            price REAL NOT NULL,
            change_percent REAL,
            volume REAL,
            market_cap REAL,
            high REAL,
            low REAL,
            fetched_at INTEGER NOT NULL,
            UNIQUE(symbol)
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS positions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL,
            quantity REAL NOT NULL,
            avg_cost REAL NOT NULL,
            side TEXT NOT NULL DEFAULT 'long',
            status TEXT NOT NULL DEFAULT 'open',
            opened_at INTEGER NOT NULL,
            closed_at INTEGER,
            realized_pnl REAL DEFAULT 0
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS risk_rules (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rule_key TEXT UNIQUE NOT NULL,
            name TEXT NOT NULL,
            threshold REAL NOT NULL,
            severity TEXT NOT NULL DEFAULT 'warning',
            is_active INTEGER NOT NULL DEFAULT 1,
            description TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS trade_journal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            symbol TEXT NOT NULL,
            side TEXT NOT NULL,
            quantity REAL NOT NULL,
            price REAL NOT NULL,
            commission REAL DEFAULT 0,
            note TEXT,
            position_id INTEGER,
            traded_at INTEGER NOT NULL,
            FOREIGN KEY(position_id) REFERENCES positions(id)
        )",
        [],
    )?;

    // Seed default risk rules if empty
    let rule_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM risk_rules", [], |row| row.get(0))
        .unwrap_or(0);
    if rule_count == 0 {
        let _ = conn.execute(
            "INSERT INTO risk_rules (rule_key, name, threshold, severity, is_active, description) VALUES
             ('concentration_pct', 'Single Position Concentration', 30.0, 'warning', 1, 'Alert when any single position exceeds this % of portfolio'),
             ('drawdown_pct', 'Portfolio Drawdown', 15.0, 'critical', 1, 'Alert when unrealized loss exceeds this % of cost basis'),
             ('utilization_pct', 'Credit Utilization', 30.0, 'warning', 1, 'Alert when credit utilization exceeds this %'),
             ('monthly_spend', 'Monthly Spend Limit', 2500.0, 'warning', 1, 'Alert when monthly spending exceeds this amount')",
            [],
        );
    }

    // ── Clipboard history table (arboard integration) ──
    conn.execute(
        "CREATE TABLE IF NOT EXISTS clipboard_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            content TEXT NOT NULL,
            preview TEXT NOT NULL,
            copied_at INTEGER NOT NULL
        )",
        [],
    )?;

    Ok(conn)
}
