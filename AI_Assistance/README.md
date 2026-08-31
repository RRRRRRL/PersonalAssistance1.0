# AI Assistance Overlay (Tauri + SvelteKit + TypeScript)

A desktop overlay assistant built with Tauri v2 and SvelteKit.

The app combines:

- AI streaming chat with configurable providers (Poe / Alibaba DashScope)
- Orchestration workflows with a LangGraph Python sidecar
- Financial manager, market data engine, and portfolio risk tracking
- Voice input (Whisper STT) and output (OpenAI TTS)
- Sensitive-data redaction guard (15 pattern types) before model-facing paths
- Clipboard manager, system monitor, widget toolkit, and YouTube Music integration

## Infrastructure Contract

The project stack is locked to:

- **Poe** or **Alibaba DashScope** (AI API providers, OpenAI-compatible)
- **Tauri v2** + **SvelteKit** (desktop shell)
- **Rust** (backend command and orchestration layer)
- **SQLite** (local persistence via `rusqlite`)
- **Python / LangGraph** (orchestrator agent sidecar)

Design rules:

- Keep provider calls in Rust commands.
- Keep secrets in environment variables, never in frontend state or prompts.
- Keep persistent app state in SQLite.
- No local AI models — all inference goes through API providers.

## Prerequisites

- Node.js and npm
- Rust toolchain with Cargo
- Python 3.10+ for the orchestration agent
- A desktop environment supported by Tauri v2

The frontend can run independently in a browser, but the full desktop app
requires the Rust toolchain. The orchestration agent is optional until an
orchestration workflow is used; install its dependencies to enable that
workflow.

## Core Features

### Overlay Window

- Transparent, always-on-top overlay window
- Global hotkey toggle: **Alt+Space**
- Auto-focus behavior when shown
- Centered, non-resizable, decoration-free

### AI Streaming

- Streaming AI response channel from Rust to Svelte (OpenAI-compatible SSE)
- Configurable provider: Poe (default, `GPT-4o`) or Alibaba DashScope
- Batched token persistence (size/time-based flush)
- Stream sessions stored with token batch history

### Orchestration System

- Signal ingestion and action generation
- Rule-based automation (custom rules with templates)
- Pending action queue with apply/complete support
- Batch apply actions
- Python LangGraph sidecar for intent classification, entity extraction, and action planning
- Vector memory search via Qdrant (optional)
- Sandboxed code execution via E2B (optional)

### Financial Manager

- Purchase history tracking with category and payment method
- Credit card account tracking (balance, limit, due date)
- Bank balance tracking
- Monthly spend and utilization overview
- Recurring expense and card due analytics
- Advisor sync into orchestration queue

### Market Data Engine

- Watchlist management (add, remove, track symbols)
- Yahoo Finance adapter for equities, ETFs, and indices
- CoinGecko adapter for cryptocurrencies
- Price cache with TTL-based staleness detection
- Position tracking (open/close, quantity, avg cost, realized P&L)
- Portfolio summary with unrealized/realized P&L
- Risk engine with configurable rules (concentration, drawdown, utilization, spend limits)
- Trade journal with per-trade notes and commission tracking

### Alert Center

- Threshold-based alerts (utilization, spend, due reminders)
- Periodic/focus/visibility-triggered evaluation
- Persistent alert records with acknowledge flow

### Sensitive Data Guard

- Local redaction before model-facing processing
- Detects 15 pattern types: private keys, AWS keys, API tokens, auth headers, JWTs, passwords, OTP/2FA, CVV, SSN, bank accounts, card numbers (Luhn-validated), emails, phone numbers, IPv4 addresses, connection strings
- Severity classification (critical / high / medium / none)
- Redaction event audit table
- Frontend warning and confirm flow before sending text to AI

### Voice (STT / TTS)

- Speech-to-text via OpenAI Whisper API
- Text-to-speech via OpenAI TTS API (model: `tts-1`, voice: `nova`)
- Configurable provider settings stored in SQLite

### Widgets

- Weather summary (Open-Meteo)
- Public holidays (Nager.Date)
- URL safety scanning (URLhaus)
- Email breach check (EmailRep.io)
- Secure password generator
- Currency exchange rates (Frankfurter)
- Dictionary / word lookup (Free Dictionary API)
- Self-Review observability card (Langfuse traces)
- Markdown rendering (pulldown-cmark)

### YouTube Music

- Playlist browsing and search
- Playback status tracking
- Configurable provider settings

### System Monitor

- Real-time CPU, RAM, and disk usage via `sysinfo`
- Uptime and process count

### Clipboard Manager

- Cross-platform clipboard read/write via `arboard`
- Searchable clipboard history (SQLite-backed)
- Deduplication of consecutive identical copies
- Clear history support

### Calendar

- Calendar event tracking
- Deep work block support
- Integration with orchestration actions

### Self-Review / Observability

- Langfuse trace integration for AI quality monitoring
- Auto-scoring and manual review submission
- Graceful degradation when Langfuse is not configured

## Frontend Components

All dashboard and workflow components in `src/lib/components/`:

| Component                            | Description                              |
| ------------------------------------ | ---------------------------------------- |
| `SearchBar.svelte`                   | Global search with command palette       |
| `BentoDashboard.svelte`              | Main bento-grid layout container         |
| `OrchestrationCard.svelte`           | Signal queue and action management       |
| `RuleBuilderCard.svelte`             | Automation rule editor                   |
| `ActionApprovalQueueCard.svelte`     | Pending action approval flow             |
| `AlertCenterCard.svelte`             | Alert list and acknowledge flow          |
| `FinancialManagerCard.svelte`        | Purchase, card, and bank management      |
| `FinancialSnapshotCard.svelte`       | Financial overview totals                |
| `StockResearchDigestCard.svelte`     | AI-generated stock research              |
| `WeeklyPlanningAssistantCard.svelte` | AI weekly plan generator                 |
| `CalendarCard.svelte`                | Calendar event display                   |
| `FolderCard.svelte`                  | Desktop file browser and grouping        |
| `RelatedContextPanel.svelte`         | Contextual suggestions panel             |
| `ScratchNoteCard.svelte`             | Quick notes scratchpad                   |
| `CodingWorkspaceCard.svelte`         | Code snippet workspace                   |
| `ClipboardHistoryCard.svelte`        | Clipboard history browser                |
| `DailyBriefCard.svelte`              | Daily briefing summary                   |
| `PortfolioCard.svelte`               | Portfolio positions and P&L              |
| `QuickUtilitiesCard.svelte`          | Password gen, exchange rates, dictionary |
| `SecurityToolkitCard.svelte`         | URL scan, email breach, network info     |
| `SelfReviewCard.svelte`              | Langfuse observability dashboard         |
| `SystemMonitorCard.svelte`           | CPU/RAM/disk usage gauges                |
| `VoiceAssistant.svelte`              | Voice input/output interface             |
| `YouTubeMusicCard.svelte`            | YouTube Music player and playlists       |

## Rust Command Modules

All backend modules in `src-tauri/src/commands/`:

| Module              | Description                                                        |
| ------------------- | ------------------------------------------------------------------ |
| `ai.rs`             | AI streaming, provider settings, token batch persistence           |
| `advisor.rs`        | Financial CRUD, alerts, market data engine, portfolio, risk engine |
| `calendar.rs`       | Calendar event queries                                             |
| `clipboard.rs`      | Clipboard read/write and history (arboard + SQLite)                |
| `files.rs`          | Desktop file listing, AI-powered file grouping                     |
| `market_data.rs`    | Yahoo Finance and CoinGecko adapters                               |
| `orchestration.rs`  | Signal processing, rules, actions, vector memory, sandbox          |
| `sensitive.rs`      | Sensitive data redaction (15 pattern types + severity)             |
| `system_monitor.rs` | System stats (CPU, RAM, disk)                                      |
| `voice.rs`          | Whisper STT and OpenAI TTS integration                             |
| `widgets.rs`        | Weather, holidays, security tools, utilities, self-review          |
| `youtube_music.rs`  | YouTube Music playlists, search, status                            |

## Persistence (SQLite)

Main DB is initialized via `src-tauri/src/db.rs`. Tables grouped by domain:

### File Management

- `file_history` — accessed file tracking
- `file_grouping_policies` — AI grouping configuration
- `file_grouping_runs` — grouping batch run history

### Orchestration

- `orchestration_signals` — ingested signal records
- `orchestration_actions` — generated action queue
- `orchestration_rules` — automation rule definitions
- `orchestration_settings` — sidecar agent configuration
- `orchestration_graph_runs` — LangGraph execution traces

### AI

- `ai_stream_sessions` — chat session metadata
- `ai_token_batches` — persisted token batches
- `ai_provider_settings` — active provider configuration

### Financial

- `purchase_history` — tracked purchases
- `credit_card_accounts` — card balances and limits
- `bank_accounts` — bank balance tracking

### Market Data

- `watchlist` — tracked symbols
- `price_cache` — latest quote per symbol
- `positions` — open/closed trading positions
- `risk_rules` — configurable risk thresholds
- `trade_journal` — individual trade records

### Calendar

- `calendar_events` — scheduled events

### Alerts & Security

- `system_alerts` — threshold-based alert records
- `sensitive_redaction_events` — redaction audit log

### Settings & Utilities

- `youtube_music_settings` — YouTube Music config
- `voice_settings` — STT/TTS provider config
- `vector_memory_metadata` — Qdrant vector memory index
- `clipboard_history` — clipboard content history

## Python Orchestrator Agent

The LangGraph sidecar (`scripts/orchestrator_agent/main.py`) runs as a Python process managed by the Rust backend:

- **Signal classification** — categorizes incoming signals
- **Entity extraction** — identifies key entities in signal content
- **Action planning** — generates orchestration actions from classified signals
- **Verification** — validates generated actions before queue insertion
- **Vector memory** — stores and retrieves signal embeddings via Qdrant (optional)
- **Sandbox execution** — runs generated code in E2B sandboxes (optional)

## Optional Integrations

| Integration  | Purpose                                | Env Variable                                 |
| ------------ | -------------------------------------- | -------------------------------------------- |
| **Qdrant**   | Vector memory for orchestration search | `QDRANT_STORAGE_PATH`                        |
| **E2B**      | Sandboxed code execution               | `E2B_API_KEY`                                |
| **Langfuse** | LLM observability and self-review      | `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY` |

All optional integrations degrade gracefully when not configured.

## Environment Variables

Create `AI_Assistance/.env` for API keys and optional integration settings.
Keys are read by the Rust backend and forwarded only to the Python agent when
it is started; they are not stored in SQLite.

| Variable              | Used for                                                       | Required    |
| --------------------- | -------------------------------------------------------------- | ----------- |
| `POE_API_KEY`         | Poe chat, orchestration, and YouTube Music provider            | For Poe     |
| `ALIBABA_API_KEY`     | Alibaba DashScope chat and orchestration                       | For Alibaba |
| `OPENAI_API_KEY`      | Whisper speech-to-text and OpenAI text-to-speech               | For voice   |
| `LANGFUSE_PUBLIC_KEY` | Langfuse observability                                         | Optional    |
| `LANGFUSE_SECRET_KEY` | Langfuse observability                                         | Optional    |
| `LANGFUSE_HOST`       | Custom Langfuse host; defaults to `https://cloud.langfuse.com` | Optional    |
| `QDRANT_STORAGE_PATH` | Embedded Qdrant storage location                               | Optional    |
| `E2B_API_KEY`         | E2B sandbox execution                                          | Optional    |

Provider URLs, model names, voice settings, the Python executable, agent port,
and sidecar script directory are stored through the app settings UI and in
SQLite. The orchestration agent defaults to port `8765` and the Python
executable `python`.

## Performance Notes

- Token writes are batched before persistence (size/time-based flush)
- Alert evaluation avoids overlapping calls
- Refresh bus coordinates cross-card refreshes to reduce redundant invokes
- Stream session token accounting is idempotent for replaced batches
- Market data cache stores latest quote per symbol in SQLite

## Security Notes

- Sensitive prompts are scanned/redacted locally before AI stream processing (15 pattern types with severity classification)
- Orchestration signal content is sanitized before storage and forwarding to LangGraph sidecar
- Redaction events are logged for auditability
- Do not store raw secrets in project config or code — use environment variables
- API keys are referenced by env var name, never stored in SQLite directly

## Model Context And Prompts

Prompt templates for AI-facing features are defined in the Rust source code. Key principles:

- **System prompt**: local desktop assistant role; never request secrets; concise actionable output
- **Chat**: user goal + redacted context → direct answer + next action + optional follow-up
- **Orchestration planner**: signal + rules + pending actions → JSON action plan
- **Financial advisor**: spend + balance + utilization + due items → risk flags + optimization actions
- **Alert explanation**: alert type + metrics → urgency + mitigation
- **Weekly planning**: calendar + pending actions + themes → day-by-day plan

All model prompts append a sensitive data safeguard clause: never reconstruct redacted values.

See `MODEL_ASSIGNMENTS.md` for the module-to-model mapping.

## Project Structure

```
AI_Assistance/
├── src/                        # SvelteKit frontend
│   ├── lib/components/         # 24 Svelte components (bento dashboard)
│   ├── lib/stores/             # Svelte stores
│   └── routes/                 # SvelteKit routes and layouts
├── src-tauri/                  # Rust backend and Tauri config
│   ├── src/
│   │   ├── commands/           # 12 command modules
│   │   ├── db.rs               # SQLite initialization (25+ tables)
│   │   ├── lib.rs              # Command registration and app startup
│   │   └── main.rs             # Binary entry point
│   ├── Cargo.toml              # Rust dependencies
│   └── tauri.conf.json         # Overlay window configuration
├── scripts/orchestrator_agent/ # Python LangGraph sidecar
├── .env                        # Local API keys and integration config (create this)
└── package.json                # Frontend dependencies
```

## Config

### App Identity

- Product name: `ai_assistance`
- Version: `0.1.0`
- Bundle identifier: `benny`

### Tauri Build Config

Configured in `src-tauri/tauri.conf.json`:

- beforeDevCommand: `npm run dev`
- devUrl: `http://localhost:1420`
- beforeBuildCommand: `npm run build`
- frontendDist: `../build`

### Window Behavior

Configured in `src-tauri/tauri.conf.json`:

- Size: 750×500
- Transparent and decoration-free
- Starts hidden
- Always on top and not shown in taskbar
- Centered and non-resizable

### Frontend Dev Server

Configured in `vite.config.js`:

- Port: 1420
- strictPort: true
- HMR port: 1421 (when using `TAURI_DEV_HOST`)
- `src-tauri` is ignored by watcher

### Orchestration Agent

The Rust backend starts the Python agent when requested by the frontend. It
executes `scripts/orchestrator_agent/main.py`, sets `ORCHESTRATOR_PORT` to
`8765` by default, and checks the service health before sending orchestration
requests. Set a custom executable or script directory in the orchestration
agent settings if `python` is not on `PATH` or the app is packaged elsewhere.

### Rust Dependencies (Backend)

Configured in `src-tauri/Cargo.toml`:

- `tauri` (v2) + tray-icon support
- `tauri-plugin-global-shortcut`, `tauri-plugin-opener`
- `rusqlite` (bundled SQLite)
- `reqwest` (JSON, streaming, multipart), `tokio`, `futures`
- `chrono`, `regex`, `serde`/`serde_json`
- `dirs`, `base64`, `rand`, `sysinfo`
- `arboard` (clipboard), `pulldown-cmark` (markdown)

## Boot Up

Run these commands from the project root.

### 1. Install Dependencies

```bash
npm install
```

Install the Python sidecar dependencies in a virtual environment:

```bash
python -m venv .venv
# Windows PowerShell
.\.venv\Scripts\Activate.ps1
# macOS/Linux
source .venv/bin/activate
pip install -r scripts/orchestrator_agent/requirements.txt
```

The Qdrant, E2B, and Data-Juicer packages in `requirements.txt` are optional
and remain commented out by default.

### 2. Frontend Only (Browser)

```bash
npm run dev
```

Then open: `http://localhost:1420`

### 3. Full Desktop Overlay (Tauri)

```bash
npm run tauri dev
```

Notes:

- This launches Vite and the Rust backend together.
- The Python orchestrator sidecar is started by the app when orchestration is
  initialized; it uses the configured Python executable and agent settings.
- The app window starts hidden and is toggled with **Alt+Space**.

### 4. Build Production

```bash
npm run build
npm run tauri build
```

## Recommended IDE Setup

- VS Code
- Svelte extension
- Tauri extension
- rust-analyzer extension
