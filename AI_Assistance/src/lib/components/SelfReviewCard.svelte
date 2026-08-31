<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface TraceScore {
    name: string;
    value: number;
    comment: string;
  }

  interface TraceSummary {
    id: string;
    name: string;
    timestamp: string | null;
    input: Record<string, unknown> | null;
    output: Record<string, unknown> | null;
    tags: string[];
    scores: TraceScore[];
  }

  interface SelfReviewResponse {
    traces: TraceSummary[];
    available: boolean;
    message: string | null;
    avg_scores: Record<string, number>;
  }

  let data = $state<SelfReviewResponse | null>(null);
  let loading = $state(false);
  let expandedTrace = $state<string | null>(null);
  let reviewMode = $state(false);
  let pendingScore = $state<{ traceId: string; value: number } | null>(null);
  let capabilities = $state<{ qdrant: boolean; e2b: boolean; data_juicer: boolean; langfuse: boolean } | null>(null);

  onMount(() => {
    void loadTraces();
    void loadCapabilities();
  });

  async function loadCapabilities() {
    try {
      capabilities = await invoke("get_integration_capabilities");
    } catch {
      capabilities = null;
    }
  }

  async function loadTraces() {
    loading = true;
    try {
      data = await invoke<SelfReviewResponse>("get_self_review_traces", { limit: 10 });
    } catch (e) {
      console.error("Self-review load failed", e);
    } finally {
      loading = false;
    }
  }

  function toggleTrace(id: string) {
    expandedTrace = expandedTrace === id ? null : id;
  }

  function formatTime(ts: string | null): string {
    if (!ts) return "";
    try {
      const d = new Date(ts);
      return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    } catch {
      return ts.slice(11, 16);
    }
  }

  function scoreColor(v: number): string {
    if (v >= 0.8) return "text-emerald-300";
    if (v >= 0.5) return "text-amber-300";
    return "text-red-300";
  }

  function scoreBarWidth(v: number): string {
    return `${Math.round(v * 100)}%`;
  }

  async function submitReview(traceId: string, value: number) {
    try {
      await invoke("submit_self_review_score", {
        traceId,
        name: "human_review",
        value,
        comment: value >= 0.8 ? "Looks good" : value >= 0.5 ? "Needs improvement" : "Poor quality",
      });
      pendingScore = null;
      await loadTraces();
    } catch (e) {
      console.error("Score submit failed", e);
    }
  }

  function getTraceInput(t: TraceSummary): string {
    if (!t.input) return "";
    const src = (t.input as Record<string, unknown>).source;
    const content = (t.input as Record<string, unknown>).content;
    return `${src ? `[${src}] ` : ""}${content ? String(content).slice(0, 80) : ""}`;
  }

  function getTraceActions(t: TraceSummary): number {
    if (!t.output) return 0;
    return (t.output as Record<string, unknown>).action_count as number ?? 0;
  }
</script>

<div class="space-y-3 text-sm text-slate-100">
  <!-- Header with status -->
  <div class="flex items-center justify-between">
    <div class="flex items-center gap-2">
      {#if loading}
        <span class="inline-block h-2 w-2 animate-pulse rounded-full bg-amber-400"></span>
        <span class="text-xs text-slate-300">Loading...</span>
      {:else if data?.available}
        <span class="inline-block h-2 w-2 rounded-full bg-emerald-400"></span>
        <span class="text-xs text-emerald-200">Connected</span>
      {:else}
        <span class="inline-block h-2 w-2 rounded-full bg-slate-500"></span>
        <span class="text-xs text-slate-400">Not configured</span>
      {/if}
    </div>
    <div class="flex gap-1">
      <button
        type="button"
        class="rounded border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] text-slate-300 hover:bg-white/10"
        on:click={() => { reviewMode = !reviewMode; }}
      >
        {reviewMode ? "Done" : "Review"}
      </button>
      <button
        type="button"
        class="rounded border border-white/10 bg-white/5 px-2 py-0.5 text-[10px] text-slate-300 hover:bg-white/10"
        on:click={loadTraces}
      >
        ↻
      </button>
    </div>
  </div>

  {#if !data?.available}
    <div class="rounded-lg border border-white/10 bg-black/20 p-3 text-xs text-slate-400">
      <p class="font-semibold text-slate-300">Langfuse not configured</p>
      <p class="mt-1">Add to <code class="rounded bg-white/5 px-1">.env</code>:</p>
      <pre class="mt-1 overflow-x-auto text-[10px] text-slate-500">LANGFUSE_PUBLIC_KEY=pk-...
LANGFUSE_SECRET_KEY=sk-...
LANGFUSE_HOST=https://cloud.langfuse.com</pre>
      <p class="mt-2 text-[10px]">Self-host for full privacy or use Langfuse Cloud.</p>
    </div>
  {:else}
    <!-- Avg Scores -->
    {#if Object.keys(data?.avg_scores ?? {}).length > 0}
      <div class="rounded-lg border border-white/10 bg-black/20 p-2">
        <p class="mb-1.5 text-[10px] font-semibold uppercase tracking-wider text-violet-200">Avg Quality</p>
        <div class="space-y-1">
          {#each Object.entries(data?.avg_scores ?? {}) as [name, value]}
            <div class="flex items-center gap-2">
              <span class="w-20 truncate text-[10px] text-slate-400">{name.replace(/_/g, " ")}</span>
              <div class="relative h-1.5 flex-1 overflow-hidden rounded-full bg-white/5">
                <div
                  class="h-full rounded-full {value >= 0.8 ? 'bg-emerald-400/70' : value >= 0.5 ? 'bg-amber-400/70' : 'bg-red-400/70'}"
                  style="width: {scoreBarWidth(value)}"
                ></div>
              </div>
              <span class="w-8 text-right text-[10px] font-mono {scoreColor(value)}">{(value * 100).toFixed(0)}%</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}

    <!-- Traces list -->
    <div class="space-y-1">
      <p class="text-[10px] font-semibold uppercase tracking-wider text-violet-200">Recent Runs ({data?.traces.length ?? 0})</p>
      {#each (data?.traces ?? []) as trace}
        <button
          type="button"
          class="w-full rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-left text-[11px] transition hover:bg-white/10"
          on:click={() => toggleTrace(trace.id)}
        >
          <div class="flex items-center justify-between gap-2">
            <span class="truncate text-slate-200">{getTraceInput(trace) || trace.name}</span>
            <span class="shrink-0 text-[10px] text-slate-400">{formatTime(trace.timestamp)}</span>
          </div>
          <div class="mt-0.5 flex items-center gap-1.5">
            <span class="rounded bg-indigo-500/15 px-1 text-[9px] text-indigo-200">{trace.tags[0] ?? "—"}</span>
            <span class="text-[9px] text-slate-400">{getTraceActions(trace)} actions</span>
            {#if trace.scores.length > 0}
              {@const avg = trace.scores.reduce((s, sc) => s + sc.value, 0) / trace.scores.length}
              <span class="ml-auto text-[9px] font-mono {scoreColor(avg)}">{(avg * 100).toFixed(0)}%</span>
            {/if}
          </div>
        </button>

        {#if expandedTrace === trace.id}
          <div class="rounded-md border border-white/10 bg-black/30 p-2 text-[10px]">
            {#if trace.scores.length > 0}
              <p class="mb-1 font-semibold text-slate-300">Auto-Scores</p>
              {#each trace.scores as sc}
                <div class="flex items-center justify-between py-0.5">
                  <span class="text-slate-400">{sc.name.replace(/_/g, " ")}</span>
                  <span class="{scoreColor(sc.value)}">{sc.comment || (sc.value * 100).toFixed(0) + "%"}</span>
                </div>
              {/each}
            {/if}

            {#if reviewMode}
              <div class="mt-2 border-t border-white/10 pt-2">
                <p class="mb-1 font-semibold text-slate-300">Your Review</p>
                <div class="flex gap-1">
                  <button type="button" class="rounded bg-emerald-500/20 px-2 py-0.5 text-emerald-200 hover:bg-emerald-500/30" on:click={() => submitReview(trace.id, 1.0)}>Good</button>
                  <button type="button" class="rounded bg-amber-500/20 px-2 py-0.5 text-amber-200 hover:bg-amber-500/30" on:click={() => submitReview(trace.id, 0.5)}>OK</button>
                  <button type="button" class="rounded bg-red-500/20 px-2 py-0.5 text-red-200 hover:bg-red-500/30" on:click={() => submitReview(trace.id, 0.2)}>Poor</button>
                </div>
              </div>
            {/if}
          </div>
        {/if}
      {/each}

      {#if (data?.traces ?? []).length === 0}
        <p class="text-xs text-slate-400">No traces yet. Process some signals first.</p>
      {/if}
    </div>
  {/if}

  <!-- Integration capabilities status row -->
  {#if capabilities !== null}
    <div class="flex flex-wrap gap-1 border-t border-white/10 pt-2">
      {#each [
        { key: 'qdrant', label: 'Qdrant' },
        { key: 'e2b', label: 'E2B' },
        { key: 'data_juicer', label: 'DJ' },
        { key: 'langfuse', label: 'Langfuse' },
      ] as cap}
        <span class="flex items-center gap-0.5 text-[9px] text-slate-400">
          <span class="h-1.5 w-1.5 rounded-full {(capabilities as Record<string,boolean>)[cap.key] ? 'bg-emerald-400' : 'bg-slate-600'}"></span>
          {cap.label}
        </span>
      {/each}
    </div>
  {/if}
</div>
