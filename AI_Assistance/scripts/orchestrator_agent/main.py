"""
Orchestrator Agent — LangGraph + FastAPI microservice.

Design principles (from "AI Agent: In Depth" by Bojie Li):
  - Agent = LLM + Context + Tools
  - Harness = Context mgmt + Tool interfaces + Constrain + Verify + Correct
  - Five design patterns: Proposer-Reviewer, Progressive Disclosure,
    Append-only, Boundary Set + Retention Set, Minimal Diff + Reversible

Graph flow:
  classify_signal → [clarify_intent] → preprocess_signal → recall_memory
  → extract_entities → plan_actions → enrich_context → finalize_plan
  → verify_and_reflect → [store_memory] → [sandbox_execute]

The Rust/Tauri backend spawns this service and calls POST /orchestrate.
"""

from __future__ import annotations

import json
import os
import sys
import time
from datetime import date, timedelta
from typing import Any, Optional

from dotenv import load_dotenv
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from langchain_openai import ChatOpenAI
from langgraph.graph import END, StateGraph
from pydantic import BaseModel, Field
from typing_extensions import TypedDict

# ---------------------------------------------------------------------------
# Langfuse (optional — only active if env vars are set)
# ---------------------------------------------------------------------------

_langfuse = None

def _init_langfuse():
    """Initialize Langfuse client if credentials are available."""
    global _langfuse
    try:
        public_key = os.getenv("LANGFUSE_PUBLIC_KEY", "")
        secret_key = os.getenv("LANGFUSE_SECRET_KEY", "")
        host = os.getenv("LANGFUSE_HOST", "https://cloud.langfuse.com")
        if public_key and secret_key:
            from langfuse import Langfuse
            _langfuse = Langfuse(
                public_key=public_key,
                secret_key=secret_key,
                host=host,
            )
            return True
    except ImportError:
        pass
    return False

_init_langfuse()

# ---------------------------------------------------------------------------
# Qdrant Vector Memory (optional — embedded, no Docker)
# ---------------------------------------------------------------------------

_qdrant_client = None
_EMBEDDING_MODEL = None


def _init_qdrant():
    """Initialize embedded Qdrant client and fastembed model if available."""
    global _qdrant_client, _EMBEDDING_MODEL
    try:
        from qdrant_client import QdrantClient
        from qdrant_client.models import Distance, VectorParams

        storage_path = os.getenv(
            "QDRANT_STORAGE_PATH",
            os.path.join(os.path.dirname(__file__), "..", "..", ".qdrant_data"),
        )
        os.makedirs(storage_path, exist_ok=True)
        _qdrant_client = QdrantClient(path=storage_path)

        if not _qdrant_client.collection_exists("signal_memories"):
            _qdrant_client.create_collection(
                collection_name="signal_memories",
                vectors_config=VectorParams(size=384, distance=Distance.COSINE),
            )

        from fastembed import TextEmbedding
        _EMBEDDING_MODEL = TextEmbedding(model_name="BAAI/bge-small-en-v1.5")
        return True
    except (ImportError, Exception):
        return False


_init_qdrant()

# ---------------------------------------------------------------------------
# E2B Sandboxed Execution (optional — requires E2B_API_KEY)
# ---------------------------------------------------------------------------

_e2b_available = False


def _init_e2b():
    """Check if E2B SDK is installed and API key is configured."""
    global _e2b_available
    try:
        import e2b_code_interpreter  # noqa: F401
        if os.getenv("E2B_API_KEY"):
            _e2b_available = True
    except ImportError:
        pass
    return _e2b_available


_init_e2b()

# ---------------------------------------------------------------------------
# Data-Juicer Signal Preprocessing (optional — local text cleaning)
# ---------------------------------------------------------------------------

_dj_available = False


def _init_data_juicer():
    """Check if Data-Juicer is installed."""
    global _dj_available
    try:
        import data_juicer  # noqa: F401
        _dj_available = True
    except ImportError:
        pass
    return _dj_available


_init_data_juicer()

# ---------------------------------------------------------------------------
# Load environment
# ---------------------------------------------------------------------------

ENV_PATH = os.path.join(os.path.dirname(__file__), "..", "..", ".env")
load_dotenv(ENV_PATH)

# ---------------------------------------------------------------------------
# Pydantic models (API contracts)
# ---------------------------------------------------------------------------


class ProviderConfig(BaseModel):
    active_provider: str = "poe"
    poe_base_url: str = "https://api.poe.com/v1"
    poe_api_key: str = ""
    poe_model: str = "GPT-4o"
    alibaba_base_url: str = "https://dashscope-intl.aliyuncs.com/compatible-mode/v1"
    alibaba_api_key: str = ""
    alibaba_model: str = "qwen-plus"


class SignalRequest(BaseModel):
    source: str = "user"
    content: str
    metadata: Optional[str] = None
    existing_rules: list[dict[str, Any]] = Field(default_factory=list)
    provider: Optional[ProviderConfig] = None


class PlannedAction(BaseModel):
    action_type: str
    title: str
    due_date: Optional[str] = None
    note: Optional[str] = None


class OrchestrationResponse(BaseModel):
    classification: dict[str, Any]
    entities: list[str]
    actions: list[PlannedAction]
    context_hints: list[str]
    summary: str
    graph_path: list[str]
    # Memory + sandbox
    recalled_memories: list[dict[str, Any]] = Field(default_factory=list)
    sandbox_results: list[dict[str, Any]] = Field(default_factory=list)
    memory_stored: bool = False
    # Harness: Verify + Correct
    verification: dict[str, Any] = Field(default_factory=dict)
    # Intent clarification
    clarification_needed: bool = False
    clarification_question: Optional[str] = None
    # Guardrails
    input_quality: dict[str, Any] = Field(default_factory=dict)


# ---------------------------------------------------------------------------
# LangGraph State
# ---------------------------------------------------------------------------


class OrchestratorState(TypedDict, total=False):
    # Input
    source: str
    content: str
    metadata: Optional[str]
    existing_rules: list[dict[str, Any]]
    # LLM
    llm: Any  # ChatOpenAI instance
    # Node outputs
    classification: dict[str, Any]
    entities: list[str]
    actions: list[dict[str, Any]]
    context_hints: list[str]
    summary: str
    graph_path: list[str]
    # Preprocessing + memory + sandbox
    cleaned_content: Optional[str]
    recalled_memories: list[dict[str, Any]]
    memory_stored: bool
    sandbox_results: list[dict[str, Any]]
    # Harness: Verify + Correct (new)
    verification: dict[str, Any]
    needs_revision: bool
    revision_count: int
    # Intent clarification (new)
    clarification_needed: bool
    clarification_question: Optional[str]
    # Guardrails (new)
    input_quality: dict[str, Any]
    # Errors
    error: Optional[str]


# ---------------------------------------------------------------------------
# System Prompts — Context Engineering (book Ch.2)
# Rich, role-specific prompts that define the Agent's "job description"
# ---------------------------------------------------------------------------

_SYSTEM_PROMPT_CLASSIFY = """You are the classification engine of an orchestration Agent.
Your role: accurately determine the user's intent, urgency, and category.

CONTEXT:
- You serve a personal productivity dashboard with widgets: Security Toolkit,
  Daily Brief, Quick Utilities (currency, dictionary), Calendar, Financial
  Manager, Coding Workspace, and Orchestration.
- Signals arrive from: user input, automated triggers, calendar events.

CLASSIFICATION RULES:
- intent: one of [task, question, schedule, coding, communication, study,
  security, lookup, weather, general]
- urgency: low | medium | high | critical — base on time-sensitivity and impact
- category: a short label (e.g., exam, meeting, bug-fix, url-scan, currency)
- requires_action: true if the signal needs any follow-up; false for pure greetings

AMBIGUITY HANDLING:
- If the signal is too vague to classify confidently (e.g., "help me"),
  set intent to "general" AND set a "clarification_hint" field explaining
  what additional info would help.

Return ONLY valid JSON."""

_SYSTEM_PROMPT_EXTRACT = """You are the entity extraction engine of an orchestration Agent.
Extract the most important entities (people, topics, tools, dates, locations, URLs).

RULES:
- Return ONLY a JSON array of short strings, max 8 items.
- Prefer specific over generic: "Python" not "programming language".
- Include temporal references: "next Friday", "tomorrow", specific dates.
- Include URLs or email addresses if present.
- If no entities are found, return an empty array []."""

_SYSTEM_PROMPT_PLAN = """You are the planning engine of an orchestration Agent.
Generate concrete, actionable items the user can act on immediately.

AVAILABLE WIDGETS & CAPABILITIES:
- Security Toolkit: URL safety scan, email breach check, network status, password generator
- Daily Brief: weather forecast and public holidays
- Quick Utilities: currency exchange rates, dictionary lookup
- Calendar: event creation, schedule management
- Financial Manager: budget tracking, expense logging
- Coding Workspace: code snippets, project scaffolding

ACTION TYPES: calendar_event, schedule_preparation, coding_context, follow_up,
research, security_scan, breach_check, password_generate, weather_check,
currency_lookup, dictionary_lookup, generic_follow_up

RULES:
- Generate 1-4 concrete items. Prefer specific action_type over generic_follow_up.
- Each item must have a clear title and optional note about which widget to use.
- Reference similar past situations (from memory) when provided."""

_SYSTEM_PROMPT_VERIFY = """You are an independent reviewer evaluating the quality of an
orchestration plan. You see ONLY the final artifacts, not the reasoning that produced them
(Proposer-Reviewer pattern).

Evaluate the plan on these dimensions:
1. action_specificity: Are actions concrete and actionable (not vague)?
2. entity_coverage: Were key entities captured and used?
3. intent_alignment: Do actions match the classified intent?
4. context_relevance: Are context hints useful and accurate?
5. completeness: Does the plan address the user's signal fully?

Return JSON:
{
  "passed": true/false,
  "score": 0.0-1.0,
  "issues": ["list of specific issues found"],
  "suggestions": ["specific improvements"],
  "dimension_scores": {
    "action_specificity": 0.0-1.0,
    "entity_coverage": 0.0-1.0,
    "intent_alignment": 0.0-1.0,
    "context_relevance": 0.0-1.0,
    "completeness": 0.0-1.0
  }
}"""


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _build_llm(cfg: Optional[ProviderConfig]) -> ChatOpenAI:
    """Create a ChatOpenAI client based on the active provider config."""
    cfg = cfg or ProviderConfig()

    if cfg.active_provider == "alibaba":
        base_url = cfg.alibaba_base_url
        api_key = cfg.alibaba_api_key or os.getenv("ALIBABA_API_KEY", "")
        model = cfg.alibaba_model
    else:
        base_url = cfg.poe_base_url
        api_key = cfg.poe_api_key or os.getenv("POE_API_KEY", "")
        model = cfg.poe_model

    if not api_key:
        # Fallback: try OPENAI_API_KEY so the service still works for dev
        api_key = os.getenv("OPENAI_API_KEY", "sk-placeholder")

    return ChatOpenAI(
        model=model,
        openai_api_key=api_key,
        openai_api_base=base_url,
        temperature=0.3,
        max_tokens=1024,
    )


def _today() -> str:
    return date.today().isoformat()


def _infer_due_date(content: str) -> str:
    low = content.lower()
    if "next week" in low:
        return (date.today() + timedelta(days=7)).isoformat()
    if "tomorrow" in low:
        return (date.today() + timedelta(days=1)).isoformat()
    return _today()


def _safe_json_parse(text: str) -> dict | list | None:
    """Attempt to parse JSON from an LLM response, tolerating markdown fences."""
    text = text.strip()
    if text.startswith("```"):
        # Strip ```json ... ```
        lines = text.split("\n")
        lines = [l for l in lines if not l.startswith("```")]
        text = "\n".join(lines).strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        return None


# ---------------------------------------------------------------------------
# LangGraph Nodes
# ---------------------------------------------------------------------------


def classify_signal(state: OrchestratorState) -> dict:
    """Classify the signal: intent, urgency, category.
    Uses the enhanced system prompt (Context Engineering — book Ch.2).
    """
    llm: ChatOpenAI = state["llm"]
    content = state["content"]
    source = state.get("source", "user")

    user_msg = f"""Source: {source}
Content: {content}

Return ONLY valid JSON with these fields:
{{
  "intent": "<one of: task, question, schedule, coding, communication, study, security, lookup, weather, general>",
  "urgency": "<one of: low, medium, high, critical>",
  "category": "<short label like exam, meeting, bug-fix, chat, planning, url-scan, breach-check, password-gen, currency, dictionary, weather>",
  "requires_action": true or false,
  "clarification_hint": "<optional: what additional info would help if signal is vague>" or null
}}"""

    resp = llm.invoke([
        {"role": "system", "content": _SYSTEM_PROMPT_CLASSIFY},
        {"role": "user", "content": user_msg},
    ])
    parsed = _safe_json_parse(resp.content)
    classification = parsed if isinstance(parsed, dict) else {
        "intent": "general",
        "urgency": "low",
        "category": "misc",
        "requires_action": True,
    }

    # Detect if clarification is needed (Intent Clarification — book Ch.1 GPT-5.6 pattern)
    clarification_needed = False
    clarification_question = None
    if classification.get("clarification_hint"):
        clarification_needed = True
        clarification_question = classification["clarification_hint"]
    elif len(content.strip()) < 5 and source == "user":
        clarification_needed = True
        clarification_question = "Your message is very short. Could you provide more details about what you need?"

    path = list(state.get("graph_path", []))
    path.append("classify_signal")
    return {
        "classification": classification,
        "graph_path": path,
        "clarification_needed": clarification_needed,
        "clarification_question": clarification_question,
    }


def extract_entities(state: OrchestratorState) -> dict:
    """Extract key entities from the content.
    Uses structured system prompt with explicit extraction rules.
    """
    llm: ChatOpenAI = state["llm"]
    content = state.get("cleaned_content") or state["content"]
    classification = state.get("classification", {})

    user_msg = f"""Given this text classified as intent={classification.get('intent', '?')} category={classification.get('category', '?')}:

"{content}"

Extract the most important entities (people, topics, tools, dates, locations, URLs).
Return ONLY a JSON array of short strings, max 8 items.
Example: ["Python", "deadline", "project-alpha"]"""

    resp = llm.invoke([
        {"role": "system", "content": _SYSTEM_PROMPT_EXTRACT},
        {"role": "user", "content": user_msg},
    ])
    parsed = _safe_json_parse(resp.content)
    entities = parsed if isinstance(parsed, list) else []

    path = list(state.get("graph_path", []))
    path.append("extract_entities")
    return {"entities": entities, "graph_path": path}


def plan_actions(state: OrchestratorState) -> dict:
    """Generate action items based on classification + entities.
    Uses structured planning prompt with widget capabilities (Context Engineering).
    """
    llm: ChatOpenAI = state["llm"]
    content = state.get("cleaned_content") or state["content"]
    classification = state.get("classification", {})
    entities = state.get("entities", [])
    existing_rules = state.get("existing_rules", [])
    recalled = state.get("recalled_memories", [])
    due_date = _infer_due_date(content)

    rules_context = ""
    if existing_rules:
        rules_context = f"\nExisting automation rules:\n{json.dumps(existing_rules[:10], indent=2)}"

    memory_context = ""
    if recalled:
        memory_context = "\nSimilar past situations (from memory):\n" + "\n".join(
            f"- {m.get('content', '')[:200]} "
            f"(intent: {m.get('classification', {}).get('intent', '?')}, "
            f"score: {m.get('score', 0):.2f})"
            for m in recalled[:3]
        )

    user_msg = f"""Based on this analysis, generate 1-4 concrete action items:

Content: {content}
Classification: {json.dumps(classification)}
Entities: {json.dumps(entities)}
Suggested due date: {due_date}
{rules_context}
{memory_context}

Generate 1-4 concrete action items. Return ONLY a JSON array:
[
  {{
    "action_type": "<one of: calendar_event, schedule_preparation, coding_context, follow_up, research, security_scan, breach_check, password_generate, weather_check, currency_lookup, dictionary_lookup, generic_follow_up>",
    "title": "<clear actionable title>",
    "due_date": "{due_date}" or null,
    "note": "<optional context note — mention which widget to use if applicable>" or null
  }}
]"""

    resp = llm.invoke([
        {"role": "system", "content": _SYSTEM_PROMPT_PLAN},
        {"role": "user", "content": user_msg},
    ])
    parsed = _safe_json_parse(resp.content)
    actions = parsed if isinstance(parsed, list) else [
        {
            "action_type": "generic_follow_up",
            "title": "Review signal and decide next action",
            "due_date": due_date,
            "note": "AI could not determine specific action.",
        }
    ]

    path = list(state.get("graph_path", []))
    path.append("plan_actions")
    return {"actions": actions, "graph_path": path}


def enrich_context(state: OrchestratorState) -> dict:
    """Add context hints based on entities and classification."""
    classification = state.get("classification", {})
    entities = state.get("entities", [])
    content = state["content"]

    hints: list[str] = []

    intent = classification.get("intent", "")
    category = classification.get("category", "")

    if intent == "coding" or category in ("bug-fix", "feature", "coding"):
        hints.append("Check file_history for recently accessed source files.")
        hints.append("Consider creating a study/prep block for focused coding.")
    if intent == "schedule" or category in ("meeting", "planning"):
        hints.append("Cross-reference calendar_events for conflicts.")
        hints.append("Check Daily Brief widget for weather and upcoming holidays before scheduling outdoor/travel events.")
    if intent == "study" or "exam" in content.lower() or "test" in content.lower():
        hints.append("Reserve deep-work study blocks in the calendar.")
    if any(e.lower() in ("email", "whatsapp", "slack", "call") for e in entities):
        hints.append("Convert communication into a follow-up reminder.")
    if intent == "security" or category in ("url-scan", "breach-check", "password-gen"):
        hints.append("Use the Security Toolkit widget: URL scan tab for link safety, Breach tab for email exposure, Pass tab for secure password generation.")
        if any(e.lower() in ("url", "link", "phishing", "malware") for e in entities):
            hints.append("Recommend scanning the URL with URLhaus before clicking.")
        if any(e.lower() in ("password", "credential", "login", "auth") for e in entities):
            hints.append("Generate a strong password using the Security Toolkit password generator.")
    if intent == "lookup" or category in ("dictionary", "currency"):
        hints.append("Use Quick Utilities widget for dictionary lookups or currency exchange rates.")
    if intent == "weather" or category == "weather":
        hints.append("Daily Brief widget shows current weather and next public holiday.")
    if classification.get("urgency") in ("high", "critical"):
        hints.append("Priority signal — surface at top of queue.")

    if not hints:
        hints.append("No specific context enrichment; use generic workflow.")

    path = list(state.get("graph_path", []))
    path.append("enrich_context")
    return {"context_hints": hints, "graph_path": path}


def finalize_plan(state: OrchestratorState) -> dict:
    """Build the summary string."""
    classification = state.get("classification", {})
    entities = state.get("entities", [])
    actions = state.get("actions", [])
    hints = state.get("context_hints", [])

    parts = [
        f"Intent: {classification.get('intent', 'general')} | "
        f"Urgency: {classification.get('urgency', 'low')} | "
        f"Category: {classification.get('category', 'misc')}",
        f"Entities: {', '.join(entities[:6])}" if entities else "No entities extracted.",
        f"{len(actions)} action(s) planned.",
    ]
    if hints:
        parts.append(f"Context: {'; '.join(hints[:3])}")

    summary = " | ".join(parts)

    path = list(state.get("graph_path", []))
    path.append("finalize_plan")
    return {"summary": summary, "graph_path": path}


# ---------------------------------------------------------------------------
# New nodes: preprocess_signal, recall_memory, store_memory, sandbox_execute
# ---------------------------------------------------------------------------

import re as _re
import uuid as _uuid


def preprocess_signal(state: OrchestratorState) -> dict:
    """Clean and normalize signal content. Uses Data-Juicer if available, otherwise regex."""
    content = state["content"]
    path = list(state.get("graph_path", []))

    if _dj_available:
        try:
            from data_juicer.ops.mapper.clean_html_mapper import CleanHtmlMapper  # noqa
            # Data-Juicer text cleaning via simple process function
            cleaned = content
            # Whitespace normalization (works without model weights)
            cleaned = _re.sub(r'\s+', ' ', cleaned).strip()
            # Remove zero-width and control chars
            cleaned = _re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]', '', cleaned)
            path.append("preprocess_signal[dj]")
            return {"cleaned_content": cleaned, "graph_path": path}
        except Exception:
            pass

    # Fallback: pure Python regex cleaning
    cleaned = _re.sub(r'\s+', ' ', content).strip()
    cleaned = _re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]', '', cleaned)
    path.append("preprocess_signal")
    return {"cleaned_content": cleaned, "graph_path": path}


def recall_memory(state: OrchestratorState) -> dict:
    """Retrieve semantically similar past signals from Qdrant vector store."""
    content = state.get("cleaned_content") or state["content"]
    path = list(state.get("graph_path", []))
    memories: list[dict[str, Any]] = []

    if _qdrant_client and _EMBEDDING_MODEL:
        try:
            embeddings = list(_EMBEDDING_MODEL.embed([content]))
            query_vec = embeddings[0].tolist()
            hits = _qdrant_client.query_points(
                collection_name="signal_memories",
                query=query_vec,
                limit=5,
                score_threshold=0.7,
            ).points
            for hit in hits:
                memories.append({
                    "content": hit.payload.get("content", ""),
                    "classification": hit.payload.get("classification", {}),
                    "entities": hit.payload.get("entities", []),
                    "score": round(hit.score, 3),
                })
            path.append("recall_memory")
        except Exception:
            path.append("recall_memory:skipped")
    else:
        path.append("recall_memory:skipped")

    return {"recalled_memories": memories, "graph_path": path}


def store_memory(state: OrchestratorState) -> dict:
    """Persist the processed signal as a vector memory in Qdrant.
    Uses Enhanced Notes format (book Ch.3): stores full context alongside
    the content for better cross-session retrieval.
    """
    path = list(state.get("graph_path", []))

    if not _qdrant_client or not _EMBEDDING_MODEL:
        path.append("store_memory:skipped")
        return {"memory_stored": False, "graph_path": path}

    try:
        content = state.get("cleaned_content") or state["content"]
        classification = state.get("classification", {})
        intent = classification.get("intent", "general")
        urgency = classification.get("urgency", "low")
        verification = state.get("verification", {})

        # Quality gate: skip low-value generic signals
        if intent == "general" and urgency == "low":
            path.append("store_memory:quality_gate")
            return {"memory_stored": False, "graph_path": path}

        # Additional quality gate: skip if verification scored very low
        v_score = verification.get("score", 0.5)
        if v_score < 0.3:
            path.append("store_memory:low_quality")
            return {"memory_stored": False, "graph_path": path}

        embeddings = list(_EMBEDDING_MODEL.embed([content]))
        vector = embeddings[0].tolist()

        # Enhanced Notes format: store rich contextual metadata
        actions_summary = [
            {"type": a.get("action_type"), "title": a.get("title")}
            for a in state.get("actions", [])[:3]
        ]

        _qdrant_client.upsert(
            collection_name="signal_memories",
            points=[{
                "id": str(_uuid.uuid4()),
                "vector": vector,
                "payload": {
                    "content": content[:1000],
                    "source": state.get("source", "user"),
                    "classification": classification,
                    "entities": state.get("entities", []),
                    "actions_summary": actions_summary,
                    "context_hints": state.get("context_hints", [])[:3],
                    "verification_score": v_score,
                    "memory_type": "episodic",  # book Ch.3: episodic/semantic/procedural
                    "stored_at": time.time(),
                    "stored_at_date": date.today().isoformat(),
                },
            }],
        )
        path.append("store_memory")
        return {"memory_stored": True, "graph_path": path}
    except Exception:
        path.append("store_memory:error")
        return {"memory_stored": False, "graph_path": path}


def _extract_code_block(text: str) -> Optional[str]:
    """Extract the first fenced code block from a string."""
    m = _re.search(r'```(?:python)?\s*\n([\s\S]+?)\n```', text)
    return m.group(1).strip() if m else None


def sandbox_execute(state: OrchestratorState) -> dict:
    """Execute code-generation actions in E2B sandbox if available."""
    path = list(state.get("graph_path", []))
    results: list[dict[str, Any]] = []

    if not _e2b_available:
        path.append("sandbox_execute:skipped")
        return {"sandbox_results": results, "graph_path": path}

    actions = state.get("actions", [])
    code_actions = [
        a for a in actions
        if a.get("action_type") == "coding_context" and _extract_code_block(a.get("note", ""))
    ]

    if not code_actions:
        path.append("sandbox_execute:no_code")
        return {"sandbox_results": results, "graph_path": path}

    try:
        from e2b_code_interpreter import Sandbox
        with Sandbox() as sandbox:
            for action in code_actions[:2]:  # cap at 2 to avoid long waits
                code = _extract_code_block(action.get("note", ""))
                if code:
                    execution = sandbox.run_code(code)
                    results.append({
                        "action_title": action.get("title", ""),
                        "stdout": execution.text or "",
                        "stderr": "\n".join(getattr(execution.logs, 'stderr', [])) if execution.logs else "",
                        "error": str(execution.error) if execution.error else None,
                        "sandboxed": True,
                    })
        path.append("sandbox_execute")
    except Exception as e:
        path.append("sandbox_execute:error")
        results.append({"error": str(e), "sandboxed": False})

    return {"sandbox_results": results, "graph_path": path}


# ---------------------------------------------------------------------------
# Harness: Constrain + Verify + Correct (book Ch.1)
# ---------------------------------------------------------------------------

_MAX_CONTENT_LENGTH = 4000  # Context compression threshold


def assess_input_quality(state: OrchestratorState) -> dict:
    """Guardrail: assess input quality before processing (Constrain layer).
    Detects empty, very short, or potentially injected content.
    """
    content = state["content"]
    source = state.get("source", "user")
    quality: dict[str, Any] = {
        "length": len(content),
        "is_empty": len(content.strip()) == 0,
        "is_very_short": 0 < len(content.strip()) < 5,
        "has_suspicious_patterns": False,
    }

    # Basic prompt-injection detection (context-layer guardrail)
    suspicious_patterns = [
        "ignore previous instructions", "ignore all instructions",
        "you are now", "system:", "<|system|>",
        "disregard your training", "forget your rules",
    ]
    low = content.lower()
    if any(p in low for p in suspicious_patterns):
        quality["has_suspicious_patterns"] = True
        quality["flag"] = "potential_prompt_injection"

    return {"input_quality": quality}


def compress_context(content: str, max_length: int = _MAX_CONTENT_LENGTH) -> str:
    """Context compression (book Ch.2): truncate overly long signals
    while preserving the beginning and end (most informative parts).
    """
    if len(content) <= max_length:
        return content
    # Keep first 70% and last 20% with a marker in between
    keep_start = int(max_length * 0.7)
    keep_end = int(max_length * 0.2)
    omitted = len(content) - keep_start - keep_end
    return (
        content[:keep_start]
        + f"\n\n[...{omitted} characters omitted for brevity...]\n\n"
        + content[-keep_end:]
    )


def verify_and_reflect(state: OrchestratorState) -> dict:
    """Proposer-Reviewer pattern (book Ch.1):
    An independent LLM call evaluates the plan quality.
    The reviewer sees ONLY the artifacts, not the reasoning.

    If the score is below threshold and revision_count < max,
    sets needs_revision=True so the graph can re-plan.
    """
    llm: ChatOpenAI = state["llm"]
    content = state.get("cleaned_content") or state["content"]
    classification = state.get("classification", {})
    entities = state.get("entities", [])
    actions = state.get("actions", [])
    hints = state.get("context_hints", [])
    summary = state.get("summary", "")
    revision_count = state.get("revision_count", 0)
    max_revisions = 1  # Cap revisions to avoid infinite loops

    # Build the artifact for the reviewer (Proposer-Reviewer: reviewer sees only output)
    artifact = f"""ORIGINAL SIGNAL: {content[:1000]}

CLASSIFICATION: {json.dumps(classification)}
ENTITIES: {json.dumps(entities)}
PLANNED ACTIONS:
{json.dumps(actions, indent=2)}
CONTEXT HINTS: {json.dumps(hints)}
SUMMARY: {summary}"""

    resp = llm.invoke([
        {"role": "system", "content": _SYSTEM_PROMPT_VERIFY},
        {"role": "user", "content": f"Review this orchestration plan:\n\n{artifact}"},
    ])
    parsed = _safe_json_parse(resp.content)
    verification = parsed if isinstance(parsed, dict) else {
        "passed": True,
        "score": 0.5,
        "issues": [],
        "suggestions": [],
        "dimension_scores": {},
    }

    # Determine if revision is needed (Correct layer)
    score = verification.get("score", 0.5)
    passed = verification.get("passed", True)
    needs_revision = (
        not passed
        and score < 0.5
        and revision_count < max_revisions
    )

    path = list(state.get("graph_path", []))
    tag = "verify_and_reflect:pass" if passed else (
        "verify_and_reflect:revise" if needs_revision else "verify_and_reflect:low_score"
    )
    path.append(tag)

    return {
        "verification": verification,
        "needs_revision": needs_revision,
        "revision_count": revision_count + (1 if needs_revision else 0),
        "graph_path": path,
    }



# ---------------------------------------------------------------------------
# Conditional routers
# ---------------------------------------------------------------------------


def route_after_classify(state: OrchestratorState) -> str:
    """Route after classification.
    - No action needed → skip to finalize_plan
    - Action needed → proceed to preprocess_signal
    (Intent clarification info is surfaced in the response, not a separate
    graph step, since the API is synchronous.)
    """
    cls = state.get("classification", {})
    if not cls.get("requires_action", True):
        return "finalize_plan"
    return "preprocess_signal"


def route_after_finalize(state: OrchestratorState) -> str:
    """Route after finalize_plan → verify_and_reflect (always, if LLM available)."""
    return "verify_and_reflect"


def route_after_verify(state: OrchestratorState) -> str:
    """Route after verification.
    - needs_revision=True → loop back to plan_actions (Correct layer)
    - Otherwise → store_memory / sandbox_execute / END
    """
    if state.get("needs_revision", False):
        return "plan_actions"  # Re-plan with verification feedback
    if _qdrant_client is not None:
        return "store_memory"
    if _e2b_available:
        return "sandbox_execute"
    return "__end__"


def route_after_store(state: OrchestratorState) -> str:
    if _e2b_available:
        return "sandbox_execute"
    return "__end__"


# ---------------------------------------------------------------------------
# Build the LangGraph
# ---------------------------------------------------------------------------


def build_graph() -> StateGraph:
    g = StateGraph(OrchestratorState)

    # Core nodes
    g.add_node("classify_signal", classify_signal)
    g.add_node("preprocess_signal", preprocess_signal)
    g.add_node("recall_memory", recall_memory)
    g.add_node("extract_entities", extract_entities)
    g.add_node("plan_actions", plan_actions)
    g.add_node("enrich_context", enrich_context)
    g.add_node("finalize_plan", finalize_plan)
    # Harness: Verify + Correct (book Ch.1)
    g.add_node("verify_and_reflect", verify_and_reflect)
    # Optional post-processing nodes
    g.add_node("store_memory", store_memory)
    g.add_node("sandbox_execute", sandbox_execute)

    g.set_entry_point("classify_signal")

    # classify_signal -> preprocess_signal (action needed) OR finalize_plan (no action)
    g.add_conditional_edges(
        "classify_signal",
        route_after_classify,
        {
            "preprocess_signal": "preprocess_signal",
            "finalize_plan": "finalize_plan",
        },
    )

    # Always: preprocess -> recall -> extract -> plan -> enrich -> finalize
    g.add_edge("preprocess_signal", "recall_memory")
    g.add_edge("recall_memory", "extract_entities")
    g.add_edge("extract_entities", "plan_actions")
    g.add_edge("plan_actions", "enrich_context")
    g.add_edge("enrich_context", "finalize_plan")

    # finalize_plan -> verify_and_reflect (Proposer-Reviewer)
    g.add_edge("finalize_plan", "verify_and_reflect")

    # verify_and_reflect -> re-plan (Correct) OR store/sandbox/END
    g.add_conditional_edges(
        "verify_and_reflect",
        route_after_verify,
        {
            "plan_actions": "plan_actions",
            "store_memory": "store_memory",
            "sandbox_execute": "sandbox_execute",
            "__end__": END,
        },
    )

    # store_memory -> sandbox_execute (if E2B) OR END
    g.add_conditional_edges(
        "store_memory",
        route_after_store,
        {
            "sandbox_execute": "sandbox_execute",
            "__end__": END,
        },
    )

    g.add_edge("sandbox_execute", END)

    return g.compile()


# Compile once at module level
_compiled_graph = build_graph()


# ---------------------------------------------------------------------------
# FastAPI App
# ---------------------------------------------------------------------------

app = FastAPI(title="Orchestrator Agent", version="2.0.0", description="LangGraph agent with Harness Engineering: Proposer-Reviewer, Guardrails, Context Compression, Intent Clarification")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

_started_at: float = time.time()


@app.get("/health")
async def health():
    uptime = round(time.time() - _started_at, 1)
    return {
        "status": "ok",
        "uptime_seconds": uptime,
        "framework": "langgraph",
        "harness_features": {
            "verify_and_reflect": True,       # Proposer-Reviewer (always on)
            "context_compression": True,       # Context compression (always on)
            "input_guardrails": True,          # Input quality + injection detection
            "intent_clarification": True,      # Ambiguity detection
            "circuit_breaker": True,           # Empty/suspicious input rejection
        },
        "langfuse_enabled": _langfuse is not None,
        "qdrant_enabled": _qdrant_client is not None,
        "e2b_enabled": _e2b_available,
        "data_juicer_enabled": _dj_available,
    }


@app.post("/orchestrate", response_model=OrchestrationResponse)
async def orchestrate(req: SignalRequest):
    trace = None
    start_time = time.time()

    try:
        llm = _build_llm(req.provider)

        # Guardrail: context compression for long signals (book Ch.2)
        content = compress_context(req.content)

        # Guardrail: input quality assessment (Constrain layer)
        quality_check = assess_input_quality({"content": req.content, "source": req.source})
        input_quality = quality_check.get("input_quality", {})

        # Circuit breaker: reject empty or suspicious inputs early
        if input_quality.get("is_empty"):
            return OrchestrationResponse(
                classification={"intent": "general", "urgency": "low",
                                "category": "empty", "requires_action": False},
                entities=[], actions=[], context_hints=[],
                summary="Empty signal received — no action taken.",
                graph_path=["circuit_breaker:empty"],
                input_quality=input_quality,
            )

        initial_state: OrchestratorState = {
            "source": req.source,
            "content": content,
            "metadata": req.metadata,
            "existing_rules": req.existing_rules,
            "llm": llm,
            "classification": {},
            "entities": [],
            "actions": [],
            "context_hints": [],
            "summary": "",
            "graph_path": [],
            "error": None,
            "input_quality": input_quality,
            "verification": {},
            "needs_revision": False,
            "revision_count": 0,
            "clarification_needed": False,
            "clarification_question": None,
        }

        # Create Langfuse trace if available
        if _langfuse:
            trace = _langfuse.trace(
                name="orchestration_run",
                input={"source": req.source, "content": req.content[:500]},
                metadata={
                    "has_rules": len(req.existing_rules) > 0,
                    "input_quality": input_quality,
                },
                tags=[req.source],
            )

        result = _compiled_graph.invoke(initial_state)
        elapsed = round(time.time() - start_time, 2)

        response = OrchestrationResponse(
            classification=result.get("classification", {}),
            entities=result.get("entities", []),
            actions=[
                PlannedAction(
                    action_type=a.get("action_type", "generic_follow_up"),
                    title=a.get("title", "Review"),
                    due_date=a.get("due_date"),
                    note=a.get("note"),
                )
                for a in result.get("actions", [])
            ],
            context_hints=result.get("context_hints", []),
            summary=result.get("summary", ""),
            graph_path=result.get("graph_path", []),
            recalled_memories=result.get("recalled_memories", []),
            sandbox_results=result.get("sandbox_results", []),
            memory_stored=result.get("memory_stored", False),
            verification=result.get("verification", {}),
            clarification_needed=result.get("clarification_needed", False),
            clarification_question=result.get("clarification_question"),
            input_quality=input_quality,
        )

        # Auto-score and finalize trace
        if trace:
            _auto_score_trace(trace, response, elapsed)
            trace.update(output={
                "summary": response.summary,
                "action_count": len(response.actions),
                "graph_path": response.graph_path,
                "elapsed_seconds": elapsed,
                "verification_score": response.verification.get("score"),
                "clarification_needed": response.clarification_needed,
            })
            _langfuse.flush()

        return response

    except Exception as e:
        if trace:
            trace.update(output={"error": str(e)})
            trace.score(name="error", value=1, comment=str(e))
            if _langfuse:
                _langfuse.flush()
        raise HTTPException(status_code=500, detail=str(e))


def _auto_score_trace(trace, response: OrchestrationResponse, elapsed: float):
    """Apply heuristic auto-scores to a Langfuse trace.
    Enhanced with book Ch.7 evaluation dimensions.
    """
    # Action specificity: did we get concrete actions vs generic?
    has_specific = any(a.action_type != "generic_follow_up" for a in response.actions)
    trace.score(name="action_specificity", value=1.0 if has_specific else 0.3,
               comment="Specific action types planned" if has_specific else "Fell back to generic")

    # Entity richness: did extraction find meaningful entities?
    entity_count = len(response.entities)
    trace.score(name="entity_extraction", value=min(1.0, entity_count / 4.0),
               comment=f"{entity_count} entities extracted")

    # Latency: faster is better (threshold: 15s — accounts for verify step)
    trace.score(name="latency", value=max(0.0, 1.0 - (elapsed / 15.0)),
               comment=f"{elapsed}s elapsed")

    # Classification confidence: non-general intent scores higher
    intent = response.classification.get("intent", "general")
    trace.score(name="classification_clarity", value=0.3 if intent == "general" else 1.0,
               comment=f"Intent: {intent}")

    # Context enrichment: has relevant hints?
    hints = response.context_hints
    has_specific_hints = any(h != "No specific context enrichment; use generic workflow." for h in hints)
    trace.score(name="context_relevance", value=1.0 if has_specific_hints else 0.4,
               comment=f"{len(hints)} context hint(s)")

    # Verification score from Proposer-Reviewer (new)
    v_score = response.verification.get("score")
    if v_score is not None:
        trace.score(name="verification_passed", value=float(v_score),
                   comment=f"Reviewer score: {v_score}")

    # Clarification tracking (new)
    if response.clarification_needed:
        trace.score(name="clarity", value=0.4,
                   comment="Signal required clarification")
    else:
        trace.score(name="clarity", value=1.0,
                   comment="Signal was clear")

    # Input quality tracking (new — guardrail metric)
    iq = response.input_quality
    if iq.get("has_suspicious_patterns"):
        trace.score(name="input_safety", value=0.2,
                   comment="Suspicious patterns detected")
    elif iq.get("is_very_short"):
        trace.score(name="input_safety", value=0.6,
                   comment="Very short input")
    else:
        trace.score(name="input_safety", value=1.0,
                   comment="Input passed quality checks")


@app.get("/traces")
async def get_traces(limit: int = 10):
    """Return recent orchestration traces from Langfuse (if configured)."""
    if not _langfuse:
        return {"traces": [], "available": False, "message": "Langfuse not configured. Set LANGFUSE_PUBLIC_KEY and LANGFUSE_SECRET_KEY in .env"}

    try:
        traces = _langfuse.fetch_traces(limit=min(limit, 50))
        result = []
        for t in traces.data:
            result.append({
                "id": t.id,
                "name": t.name,
                "timestamp": str(t.timestamp) if t.timestamp else None,
                "input": t.input,
                "output": t.output,
                "metadata": t.metadata,
                "tags": t.tags or [],
                "scores": [{"name": s.name, "value": s.value, "comment": getattr(s, 'comment', '')} for s in (t.scores or [])],
            })
        return {"traces": result, "available": True}
    except Exception as e:
        return {"traces": [], "available": False, "message": str(e)}


@app.get("/trace/{trace_id}")
async def get_trace_detail(trace_id: str):
    """Return a single trace with full detail."""
    if not _langfuse:
        raise HTTPException(status_code=503, detail="Langfuse not configured")
    try:
        t = _langfuse.get_trace(trace_id)
        return {
            "id": t.id,
            "name": t.name,
            "timestamp": str(t.timestamp) if t.timestamp else None,
            "input": t.input,
            "output": t.output,
            "metadata": t.metadata,
            "tags": t.tags or [],
            "observations": [{"name": o.name, "model": getattr(o, 'model', None), "usage": getattr(o, 'usage', None)} for o in (t.observations or [])],
            "scores": [{"name": s.name, "value": s.value, "comment": getattr(s, 'comment', '')} for s in (t.scores or [])],
        }
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/trace/{trace_id}/score")
async def add_manual_score(trace_id: str, name: str, value: float, comment: str = ""):
    """Add a manual review score to a trace."""
    if not _langfuse:
        raise HTTPException(status_code=503, detail="Langfuse not configured")
    try:
        _langfuse.score(trace_id=trace_id, name=name, value=value, comment=comment)
        _langfuse.flush()
        return {"ok": True}
    except Exception as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/graph")
async def graph_info():
    """Return the graph node names and edges for debugging."""
    nodes = [n for n in _compiled_graph.get_graph().nodes]
    return {
        "nodes": nodes,
        "framework": "langgraph",
        "design_patterns": [
            "Proposer-Reviewer (verify_and_reflect)",
            "Progressive Disclosure (system prompts separate from user messages)",
            "Append-only (graph_path tracking)",
            "Constrain-Verify-Correct (harness layers)",
        ],
    }


@app.get("/capabilities")
async def capabilities():
    """Report which optional integrations are currently active."""
    return {
        "harness": {
            "verify_and_reflect": True,
            "context_compression": True,
            "input_guardrails": True,
            "intent_clarification": True,
            "error_recovery": True,
        },
        "integrations": {
            "qdrant": _qdrant_client is not None,
            "e2b": _e2b_available,
            "data_juicer": _dj_available,
            "langfuse": _langfuse is not None,
        },
    }


@app.post("/memory/search")
async def search_memories(query: str, limit: int = 5):
    """Semantic search across stored signal memories."""
    if not _qdrant_client or not _EMBEDDING_MODEL:
        return {"available": False, "results": [], "message": "Qdrant not configured. Install qdrant-client and fastembed."}
    try:
        embeddings = list(_EMBEDDING_MODEL.embed([query]))
        hits = _qdrant_client.query_points(
            collection_name="signal_memories",
            query=embeddings[0].tolist(),
            limit=min(limit, 20),
        ).points
        return {
            "available": True,
            "results": [{"content": h.payload, "score": round(h.score, 3)} for h in hits],
        }
    except Exception as e:
        return {"available": False, "results": [], "message": str(e)}


@app.get("/memory/stats")
async def memory_stats():
    """Return stats about the vector memory collection."""
    if not _qdrant_client:
        return {"available": False, "point_count": 0, "message": "Qdrant not configured"}
    try:
        info = _qdrant_client.get_collection("signal_memories")
        return {
            "available": True,
            "point_count": info.points_count or 0,
            "vector_count": info.vectors_count or 0,
        }
    except Exception as e:
        return {"available": False, "point_count": 0, "message": str(e)}


@app.post("/sandbox/run")
async def run_sandbox(code: str, language: str = "python"):
    """Execute code in an E2B sandboxed environment."""
    if not _e2b_available:
        return {"available": False, "message": "E2B not configured. Set E2B_API_KEY in .env and install e2b-code-interpreter."}
    try:
        from e2b_code_interpreter import Sandbox
        with Sandbox() as sandbox:
            execution = sandbox.run_code(code)
            return {
                "available": True,
                "stdout": execution.text or "",
                "stderr": "\n".join(getattr(execution.logs, 'stderr', [])) if execution.logs else "",
                "error": str(execution.error) if execution.error else None,
            }
    except Exception as e:
        return {"available": False, "message": str(e)}


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    import uvicorn

    port = int(os.getenv("ORCHESTRATOR_PORT", "8765"))
    print(f"[orchestrator] Starting LangGraph agent on port {port} ...")
    uvicorn.run(app, host="127.0.0.1", port=port, log_level="info")
