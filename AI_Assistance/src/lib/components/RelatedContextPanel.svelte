<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface RelatedFile {
    path: string;
    name: string;
    last_accessed: number;
    access_count: number;
  }

  interface VectorHit {
    content: {
      content: string;
      source: string;
      classification: { intent?: string };
      entities: string[];
    };
    score: number;
  }

  interface MemoryStats {
    available: boolean;
    point_count: number;
  }

  let query = $state("");
  let mode = $state<"keyword" | "semantic">("keyword");
  let fileResults = $state<RelatedFile[]>([]);
  let vectorResults = $state<VectorHit[]>([]);
  let memStats = $state<MemoryStats | null>(null);
  let qdrantAvailable = $state(false);

  onMount(async () => {
    try {
      const stats = await invoke<MemoryStats>("get_vector_memory_stats");
      memStats = stats;
      qdrantAvailable = stats.available;
      if (qdrantAvailable) mode = "semantic";
    } catch {
      qdrantAvailable = false;
    }
  });

  async function runLookup() {
    if (!query.trim()) {
      fileResults = [];
      vectorResults = [];
      return;
    }
    try {
      if (mode === "semantic" && qdrantAvailable) {
        const res = await invoke<{ available: boolean; results: VectorHit[] }>("search_vector_memories", { query, limit: 8 });
        vectorResults = res.results ?? [];
        fileResults = [];
      } else {
        fileResults = await invoke<RelatedFile[]>("suggest_related_files", { query });
        vectorResults = [];
      }
    } catch (error) {
      console.error("Related context lookup failed", error);
    }
  }

  function scoreColor(score: number): string {
    if (score >= 0.9) return "text-emerald-300";
    if (score >= 0.75) return "text-amber-300";
    return "text-slate-400";
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <!-- Mode toggle -->
  <div class="flex items-center gap-2">
    <div class="flex rounded-lg border border-white/10 bg-white/5 p-0.5 text-[10px]">
      <button
        type="button"
        class="rounded-md px-2 py-0.5 transition {mode === 'keyword' ? 'bg-white/15 text-slate-100' : 'text-slate-400 hover:text-slate-200'}"
        on:click={() => { mode = 'keyword'; fileResults = []; vectorResults = []; }}
      >Keyword</button>
      <button
        type="button"
        class="relative rounded-md px-2 py-0.5 transition {mode === 'semantic' ? 'bg-white/15 text-slate-100' : 'text-slate-400 hover:text-slate-200'} {!qdrantAvailable ? 'opacity-40 cursor-not-allowed' : ''}"
        disabled={!qdrantAvailable}
        on:click={() => { if (qdrantAvailable) { mode = 'semantic'; fileResults = []; vectorResults = []; } }}
      >Semantic {#if qdrantAvailable}<span class="ml-0.5 h-1.5 w-1.5 rounded-full bg-emerald-400 inline-block"></span>{/if}</button>
    </div>
    {#if memStats?.available}
      <span class="text-[9px] text-slate-500">{memStats.point_count} memories</span>
    {/if}
  </div>

  <div class="flex gap-2">
    <input
      bind:value={query}
      class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-1.5"
      placeholder={mode === 'semantic' ? 'Describe a past situation...' : 'Find related docs/files for current task'}
      on:keydown={(e) => e.key === 'Enter' && runLookup()}
    />
    <button type="button" class="rounded-lg bg-cyan-400/80 px-3 py-1.5 font-semibold text-slate-950 hover:bg-cyan-300" on:click={runLookup}>Find</button>
  </div>

  <ul class="space-y-2">
    {#if mode === 'semantic' && vectorResults.length > 0}
      {#each vectorResults as hit}
        <li class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5">
          <div class="flex items-start justify-between gap-2">
            <div class="min-w-0 flex-1">
              <div class="truncate font-semibold">{hit.content.content?.slice(0, 80) ?? '—'}</div>
              <div class="mt-0.5 flex items-center gap-1.5">
                <span class="rounded bg-indigo-500/15 px-1 text-[9px] text-indigo-200">{hit.content.source ?? '?'}</span>
                {#if hit.content.classification?.intent}
                  <span class="text-[9px] text-slate-400">{hit.content.classification.intent}</span>
                {/if}
              </div>
            </div>
            <span class="shrink-0 text-[10px] font-mono {scoreColor(hit.score)}">{(hit.score * 100).toFixed(0)}%</span>
          </div>
        </li>
      {/each}
    {:else if mode === 'keyword' && fileResults.length > 0}
      {#each fileResults as file}
        <li class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5">
          <div class="truncate font-semibold">{file.name}</div>
          <div class="truncate text-[10px] text-slate-300">{file.path}</div>
        </li>
      {/each}
    {:else}
      <li class="text-[10px] text-slate-400">
        {mode === 'semantic' && !qdrantAvailable
          ? 'Semantic search unavailable — install qdrant-client + fastembed.'
          : mode === 'semantic'
          ? 'No similar memories found. Process some signals first.'
          : 'No related files yet. Try terms like exam, docs, architecture, bug.'}
      </li>
    {/if}
  </ul>
</div>
