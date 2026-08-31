# AI Model Assignments per Module

All modules use API-based models only. No local models.

---

## Models in Use

| Model | Provider | Cost (per 1M tokens) | Purpose |
|---|---|---|---|
| **GPT-4o-mini** | OpenAI / Poe | $0.15 in / $0.60 out | Primary chat, coding, file grouping |
| **Qwen-Plus** | Alibaba DashScope | ~$0.40 in / $1.20 out | Alternative provider for financial advisor, structured analysis |
| **whisper-1** | OpenAI | $0.006 per minute | Speech-to-text |
| **tts-1** | OpenAI | $15.00 per 1M chars | Text-to-speech |

---

## Module → Model Map

### 1. AI Stream (`ai.rs`)
- **Model:** GPT-4o-mini (via Poe)
- **Why:** Fast streaming, strong coding ability, 10x cheaper than GPT-4o. General-purpose chat and code generation.

### 2. Voice — Speech-to-Text (`voice.rs`)
- **Model:** whisper-1 (OpenAI)
- **Why:** Industry standard for STT. Already configured. No better API alternative at this price.

### 3. Voice — Text-to-Speech (`voice.rs`)
- **Model:** tts-1 (OpenAI, voice: nova)
- **Why:** Low latency streaming audio. Already configured. tts-1-hd only needed for audiobook quality.

### 4. Orchestration Agent (`orchestration.rs` / Python sidecar)
- **Model:** GPT-4o-mini
- **Why:** Intent classification and keyword extraction are low-reasoning tasks. GPT-4o-mini handles structured output reliably and keeps per-call cost minimal for frequent background agent loops.

### 5. Financial Advisor (`advisor.rs`)
- **Model:** Active AI provider (same as `ai.rs` — defaults to GPT-4o-mini via Poe)
- **Why:** Weekly planning digests, stock research summaries, and risk assessments need moderate reasoning. Uses the same configurable provider as chat. Qwen-Plus (Alibaba) can be selected as the active provider for stronger bilingual (CN/EN) financial analysis.

### 6. File Grouping (`files.rs`)
- **Model:** GPT-4o-mini
- **Why:** File categorization is a classification task — GPT-4o-mini excels at structured output with minimal cost.

### 7. Sensitive Data Scanner (`sensitive.rs`)
- **Model:** None (regex only)
- **Why:** PII scanning must be deterministic and instant. Regex is faster and more reliable than any LLM for pattern matching.

### 8. Widgets (`widgets.rs`)
- **Model:** None
- **Why:** Weather, exchange rates, holidays, password generation, email breach checks — all pure REST API calls. No AI needed.

### 9. YouTube Music (`youtube_music.rs`)
- **Model:** None
- **Why:** Pure data fetching from YouTube/Invidious APIs.

### 10. Calendar (`calendar.rs`)
- **Model:** None (currently)
- **Future option:** GPT-4o-mini for natural language event parsing (e.g., "meet John next Friday at 3pm").

### 11. Market Data (`market_data.rs`)
- **Model:** None
- **Why:** Pure API adapters (Yahoo Finance, CoinGecko). No AI needed.

### 12. System Monitor (`system_monitor.rs`)
- **Model:** None
- **Why:** Pure sysinfo metrics from OS. No AI needed.

### 13. Clipboard (`clipboard.rs`)
- **Model:** None
- **Why:** Pure storage and retrieval. No AI needed.

---

## Summary

| Needs AI | Count | Models Used |
|---|---|---|
| Yes | 5 modules | GPT-4o-mini, Qwen-Plus, whisper-1, tts-1 |
| No | 8 modules | N/A |
| **Total API keys needed** | **2** | OpenAI (Poe), Alibaba DashScope |
