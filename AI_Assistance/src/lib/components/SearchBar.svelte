<script lang="ts">
  import { Channel, invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import VoiceAssistant from "./VoiceAssistant.svelte";

  type Props = {
    onStreamUpdate?: (value: string) => void;
  };

  let { onStreamUpdate = () => {} }: Props = $props();

  let searchInput: HTMLInputElement | null = null;
  let prompt = $state("");
  let aiResponseText = $state("");
  let isStreaming = $state(false);
  let sensitiveWarning = $state("");
  let confirmForPrompt = $state("");

  // Voice state
  let showVoice = $state(false);
  let ttsRequest = $state<{ text: string } | null>(null);
  let lastResponseForTts = $state("");

  // Agent state
  interface AgentStatus {
    running: boolean;
    port: number;
    uptime_ms: number | null;
    settings: {
      agent_port: number;
      auto_start: boolean;
      python_executable: string;
      use_ai_provider: boolean;
    };
  }
  let agentStatus = $state<AgentStatus | null>(null);
  let agentToggling = $state(false);

  async function loadAgentStatus() {
    try {
      agentStatus = await invoke<AgentStatus>("get_orchestration_agent_status");
    } catch (e) {
      console.error("Failed to load agent status", e);
    }
  }

  async function toggleAgent() {
    if (agentToggling) return;
    agentToggling = true;
    try {
      if (agentStatus?.running) {
        await invoke("stop_orchestration_agent");
      } else {
        await invoke("start_orchestration_agent");
      }
      // Wait a bit then refresh status
      setTimeout(() => loadAgentStatus(), 1500);
    } catch (e) {
      console.error("Failed to toggle agent", e);
    } finally {
      agentToggling = false;
    }
  }

  function handleVoiceTranscription(text: string) {
    prompt = text;
    showVoice = false;
    setTimeout(() => {
      const form = document.querySelector("form");
      if (form && text.trim()) {
        form.requestSubmit();
      }
    }, 400);
  }

  function handleVoiceSpeaking(speaking: boolean) {
    if (!speaking) {
      ttsRequest = null;
    }
  }

  // Provider state
  interface AiProviderStatus {
    active_provider: string;
    active_provider_label: string;
    active_model: string;
    poe_api_key_present: boolean;
    alibaba_api_key_present: boolean;
    settings: {
      active_provider: string;
      poe_base_url: string;
      poe_api_key_env: string;
      poe_model: string;
      alibaba_base_url: string;
      alibaba_api_key_env: string;
      alibaba_model: string;
    };
  }

  let providerStatus = $state<AiProviderStatus | null>(null);
  let showProviderMenu = $state(false);
  let switchingProvider = $state(false);

  async function loadProviderStatus() {
    try {
      providerStatus = await invoke<AiProviderStatus>("get_ai_provider_settings");
    } catch (e) {
      console.error("Failed to load provider settings", e);
    }
  }

  async function switchProvider(provider: "poe" | "alibaba") {
    if (switchingProvider) return;
    switchingProvider = true;
    showProviderMenu = false;
    try {
      await invoke("set_ai_provider_settings", {
        input: { active_provider: provider },
      });
      await loadProviderStatus();
    } catch (e) {
      console.error("Failed to switch provider", e);
    } finally {
      switchingProvider = false;
    }
  }

  interface SensitiveScanResult {
    redacted_text: string;
    has_sensitive: boolean;
    redaction_count: number;
    detections: string[];
    severity: string;
  }

  $effect(() => {
    onStreamUpdate(aiResponseText);
  });

  onMount(() => {
    loadProviderStatus();
    loadAgentStatus();
    const appWindow = getCurrentWindow();

    const unlistenFocusPromise = appWindow.onFocusChanged(({ payload }) => {
      if (payload) {
        setTimeout(() => searchInput?.focus(), 10);
      }
    });

    const onKeydown = async (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        await appWindow.hide();
      }
    };

    window.addEventListener("keydown", onKeydown);

    return () => {
      unlistenFocusPromise.then((unlisten) => unlisten());
      window.removeEventListener("keydown", onKeydown);
    };
  });

  async function handleSubmit(event: SubmitEvent) {
    event.preventDefault();

    if (!prompt.trim() || isStreaming) {
      return;
    }

    aiResponseText = "";
    isStreaming = true;

    const scan = await invoke<SensitiveScanResult>("inspect_sensitive_text", { text: prompt });
    if (scan.has_sensitive && confirmForPrompt !== prompt) {
      const sevLabel = scan.severity === 'critical' ? ' [CRITICAL]' : scan.severity === 'high' ? ' [HIGH]' : '';
      sensitiveWarning = `Sensitive content detected${sevLabel} (${scan.detections.join(", ")}). Submit again to continue with redacted text only.`;
      confirmForPrompt = prompt;
      isStreaming = false;
      return;
    }

    const effectivePrompt = scan.redacted_text;
    sensitiveWarning = scan.has_sensitive
      ? "Sensitive content was redacted before processing."
      : "";
    confirmForPrompt = "";

    let sessionId = "";
    let tokenBuffer: string[] = [];
    let batchIndex = 0;
    let persistChain: Promise<void> = Promise.resolve();

    const flushTokenBatch = (force = false) => {
      if (tokenBuffer.length === 0) {
        return;
      }
      if (!force && tokenBuffer.length < 6) {
        return;
      }

      const chunk = tokenBuffer.join("");
      const count = tokenBuffer.length;
      tokenBuffer = [];
      batchIndex += 1;

      persistChain = persistChain
        .then(() =>
          invoke("save_ai_token_batch", {
            sessionId,
            batchIndex,
            tokens: chunk,
            tokenCount: count,
          })
        )
        .then(() => undefined)
        .catch((error) => {
          console.error("Failed to persist token batch", error);
        });
    };

    let flushInterval: number | null = null;

    const onToken = new Channel<string>();
    onToken.onmessage = (token) => {
      aiResponseText += token;
      tokenBuffer.push(token);
    };

    try {
      sessionId = await invoke<string>("begin_ai_stream_session", { prompt: effectivePrompt });

      flushInterval = setInterval(() => {
        flushTokenBatch();
      }, 350);

      await invoke("stream_ai_response", {
        prompt: effectivePrompt,
        on_token: onToken,
      });
    } catch (error) {
      aiResponseText = `Unable to stream AI response: ${String(error)}`;
    } finally {
      if (flushInterval !== null) {
        clearInterval(flushInterval);
      }

      if (sessionId) {
        flushTokenBatch(true);
        await persistChain;
        await invoke("finalize_ai_stream_session", { sessionId });
      }

      isStreaming = false;
      await loadProviderStatus();

      // Auto-speak the AI response via TTS
      if (aiResponseText.trim()) {
        lastResponseForTts = aiResponseText;
        ttsRequest = { text: aiResponseText };
        showVoice = true;
      }
    }
  }

  let providerLabel = $derived(
    providerStatus
      ? `${providerStatus.active_provider_label} · ${providerStatus.active_model}`
      : "Loading..."
  );

  let poeReady = $derived(providerStatus?.poe_api_key_present ?? false);
  let alibabaReady = $derived(providerStatus?.alibaba_api_key_present ?? false);
  let agentRunning = $derived(agentStatus?.running ?? false);
</script>

<form on:submit={handleSubmit} class="rounded-2xl border border-white/15 bg-black/35 p-3 backdrop-blur-xl shadow-glass">
  <div class="flex items-center gap-3">
    <!-- Provider selector -->
    <div class="relative flex-shrink-0">
      <button
        type="button"
        onclick={() => (showProviderMenu = !showProviderMenu)}
        class="flex items-center gap-1.5 rounded-xl border border-white/10 bg-white/5 px-2.5 py-3 text-xs text-white/80 transition hover:border-cyan-300/50 hover:bg-white/10"
        title="Switch AI provider"
      >
        <span class="inline-block h-1.5 w-1.5 rounded-full {providerStatus?.active_provider === 'alibaba' ? 'bg-orange-400' : 'bg-cyan-400'}"></span>
        <span class="max-w-[90px] truncate">{providerLabel}</span>
        <svg class="h-3 w-3 opacity-60" viewBox="0 0 12 12" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 5l3 3 3-3"/>
        </svg>
      </button>

      {#if showProviderMenu}
        <div class="absolute left-0 top-full z-50 mt-1.5 w-52 overflow-hidden rounded-xl border border-white/15 bg-slate-900/95 shadow-xl backdrop-blur-xl">
          <button
            type="button"
            onclick={() => switchProvider('poe')}
            class="flex w-full items-center gap-2.5 px-3 py-2.5 text-left text-xs transition hover:bg-white/10 {providerStatus?.settings.active_provider === 'poe' ? 'text-cyan-300' : 'text-white/80'}"
          >
            <span class="inline-block h-1.5 w-1.5 rounded-full bg-cyan-400"></span>
            <div class="flex-1">
              <div class="font-semibold">Poe</div>
              <div class="text-[10px] text-white/50">{providerStatus?.settings.poe_model ?? 'GPT-4o'}</div>
            </div>
            {#if poeReady}
              <span class="text-[10px] text-emerald-400">ready</span>
            {:else}
              <span class="text-[10px] text-amber-400">no key</span>
            {/if}
          </button>
          <button
            type="button"
            onclick={() => switchProvider('alibaba')}
            class="flex w-full items-center gap-2.5 px-3 py-2.5 text-left text-xs transition hover:bg-white/10 {providerStatus?.settings.active_provider === 'alibaba' ? 'text-orange-300' : 'text-white/80'}"
          >
            <span class="inline-block h-1.5 w-1.5 rounded-full bg-orange-400"></span>
            <div class="flex-1">
              <div class="font-semibold">Alibaba Qwen</div>
              <div class="text-[10px] text-white/50">{providerStatus?.settings.alibaba_model ?? 'qwen-plus'}</div>
            </div>
            {#if alibabaReady}
              <span class="text-[10px] text-emerald-400">ready</span>
            {:else}
              <span class="text-[10px] text-amber-400">no key</span>
            {/if}
          </button>
        </div>
      {/if}
    </div>

    <input
      bind:this={searchInput}
      bind:value={prompt}
      class="w-full rounded-xl border border-white/10 bg-white/5 px-4 py-3 font-body text-sm text-white outline-none transition focus:border-cyan-300/60"
      placeholder="Ask AI or run a command..."
      autocomplete="off"
      spellcheck="false"
    />
    <button
      type="submit"
      class="rounded-xl bg-cyan-400/80 px-4 py-3 font-display text-sm font-semibold text-slate-900 transition hover:bg-cyan-300 disabled:cursor-not-allowed disabled:opacity-60"
      disabled={isStreaming || !prompt.trim()}
    >
      {isStreaming ? "..." : "Send"}
    </button>
    <!-- Mic button -->
    <button
      type="button"
      onclick={() => (showVoice = !showVoice)}
      class="flex items-center justify-center rounded-xl border px-3 py-3 transition
        {showVoice
        ? 'border-rose-400/60 bg-rose-400/20 text-rose-300'
        : 'border-white/10 bg-white/5 text-white/70 hover:border-cyan-300/40 hover:text-white'}"
      title="Voice assistant"
    >
      <svg class="h-4 w-4" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 1a3 3 0 00-3 3v8a3 3 0 006 0V4a3 3 0 00-3-3z"/>
        <path d="M19 10v2a7 7 0 01-14 0v-2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <line x1="12" y1="19" x2="12" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <line x1="8" y1="23" x2="16" y2="23" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
    </button>
    <!-- LangGraph agent toggle -->
    <button
      type="button"
      onclick={toggleAgent}
      disabled={agentToggling}
      class="flex items-center gap-1.5 rounded-xl border px-2.5 py-3 transition
        {agentRunning
        ? 'border-emerald-400/50 bg-emerald-400/10 text-emerald-300'
        : 'border-white/10 bg-white/5 text-white/50 hover:border-violet-300/40 hover:text-white/80'}
        disabled:cursor-wait"
      title="LangGraph orchestrator agent (port {agentStatus?.port ?? 8765})"
    >
      <span class="relative flex h-2 w-2">
        {#if agentRunning}
          <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75"></span>
          <span class="relative inline-flex h-2 w-2 rounded-full bg-emerald-500"></span>
        {:else}
          <span class="relative inline-flex h-2 w-2 rounded-full bg-white/40"></span>
        {/if}
      </span>
      <svg class="h-3.5 w-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
        <path d="M12 2a4 4 0 014 4c0 1.5-.8 2.8-2 3.5v1h-4v-1A4 4 0 0112 2z"/>
        <path d="M8 14h8m-4-3v6m-3 3h6"/>
        <circle cx="6" cy="18" r="2"/>
        <circle cx="18" cy="18" r="2"/>
        <path d="M8 18h8M6 16v-4m12 4v-4"/>
      </svg>
      <span class="text-[10px] font-medium">{agentRunning ? 'ON' : 'OFF'}</span>
    </button>
  </div>
  {#if sensitiveWarning}
    <p class="mt-2 text-xs text-amber-200">{sensitiveWarning}</p>
  {/if}
</form>

{#if showVoice}
  <div class="mt-2 rounded-2xl border border-white/10 bg-black/30 p-4 backdrop-blur-xl">
    <VoiceAssistant
      onTranscription={handleVoiceTranscription}
      onSpeaking={handleVoiceSpeaking}
      speak={ttsRequest}
    />
  </div>
{/if}
