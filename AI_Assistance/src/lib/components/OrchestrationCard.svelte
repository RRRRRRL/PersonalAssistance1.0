<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface RelatedFile {
    path: string;
    name: string;
    last_accessed: number;
    access_count: number;
  }

  interface OrchestrationAction {
    id: number;
    action_type: string;
    title: string;
    due_date?: string;
    status: string;
    note?: string;
  }

  interface RecalledMemory {
    content: string;
    classification: { intent?: string; urgency?: string };
    entities: string[];
    score: number;
  }

  interface SandboxResult {
    action_title: string;
    stdout: string;
    stderr: string;
    error: string | null;
    sandboxed: boolean;
  }

  interface OrchestrationPlan {
    created_actions: OrchestrationAction[];
    related_files: RelatedFile[];
    recalled_memories?: RecalledMemory[];
    sandbox_results?: SandboxResult[];
    memory_stored?: boolean;
  }

  let source = $state("manual");
  let content = $state("");
  let queue = $state<OrchestrationAction[]>([]);
  let latestPlan = $state<OrchestrationPlan | null>(null);
  let busy = $state(false);
  let showMemories = $state(false);
  let showSandbox = $state(false);

  onMount(() => {
    void refreshQueue();

    const off = onRefresh("orchestration", () => {
      void refreshQueue();
    });

    return () => {
      off();
    };
  });

  async function refreshQueue() {
    try {
      queue = await invoke<OrchestrationAction[]>("get_orchestration_queue");
    } catch (error) {
      console.error("Failed to load orchestration queue", error);
    }
  }

  async function submitSignal() {
    if (!content.trim() || busy) {
      return;
    }

    busy = true;
    try {
      latestPlan = await invoke<OrchestrationPlan>("process_orchestration_signal", {
        signal: {
          source,
          content,
          metadata: null,
        },
      });
      content = "";
      await refreshQueue();
      emitRefresh("orchestration");
    } catch (error) {
      console.error("Failed to process signal", error);
    } finally {
      busy = false;
    }
  }

  async function quickSignal(quickSource: string, quickContent: string) {
    source = quickSource;
    content = quickContent;
    await submitSignal();
  }

  async function applyAction(id: number) {
    try {
      await invoke("apply_orchestration_action", { actionId: id });
      await refreshQueue();
      emitRefresh("orchestration");
      emitRefresh("calendar");
    } catch (error) {
      console.error("Failed to apply action", error);
    }
  }
</script>

<div class="space-y-3 text-sm text-slate-100">
  <div class="rounded-xl border border-white/10 bg-black/20 p-3">
    <p class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-indigo-200">Signal Router</p>
    <div class="flex flex-col gap-2">
      <input
        bind:value={source}
        class="rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-xs text-slate-100 outline-none"
        placeholder="source (exam, coding, whatsapp)"
      />
      <textarea
        bind:value={content}
        class="h-20 resize-none rounded-lg border border-white/10 bg-white/5 px-3 py-2 text-xs text-slate-100 outline-none placeholder:text-slate-400"
        placeholder="Describe the signal. Example: Exam next week for calculus."
      ></textarea>
      <button
        type="button"
        class="rounded-lg bg-indigo-400/80 px-3 py-2 text-xs font-semibold text-slate-950 transition hover:bg-indigo-300 disabled:opacity-60"
        disabled={busy || !content.trim()}
        on:click={submitSignal}
      >
        Process Signal
      </button>
    </div>
  </div>

  <div class="grid grid-cols-1 gap-2 sm:grid-cols-3">
    <button class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 text-xs hover:bg-white/10" type="button" on:click={() => quickSignal("exam", "Exam schedule next week, prepare focus blocks and notes")}>Exam Trigger</button>
    <button class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 text-xs hover:bg-white/10" type="button" on:click={() => quickSignal("coding", "Coding task requires architecture docs and related files")}>Coding Trigger</button>
    <button class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 text-xs hover:bg-white/10" type="button" on:click={() => quickSignal("whatsapp", "WhatsApp message asks for a sync meeting")}>Comms Trigger</button>
  </div>

  {#if latestPlan}
    <div class="rounded-xl border border-white/10 bg-white/5 p-3">
      <p class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-indigo-200">Latest Plan</p>
      <ul class="space-y-2">
        {#each latestPlan.created_actions as action}
          <li class="rounded-md border border-white/10 bg-black/20 px-2 py-1.5 text-xs">
            <span class="font-semibold">{action.title}</span>
            {#if action.due_date}
              <span class="text-slate-300"> (due {action.due_date})</span>
            {/if}
          </li>
        {/each}
      </ul>
      {#if latestPlan.related_files.length > 0}
        <p class="mt-3 mb-1 text-xs text-slate-300">Related files</p>
        <ul class="space-y-1">
          {#each latestPlan.related_files as file}
            <li class="truncate text-xs text-slate-200">{file.name}</li>
          {/each}
        </ul>
      {/if}

      {#if (latestPlan.recalled_memories?.length ?? 0) > 0}
        <button
          type="button"
          class="mt-3 flex w-full items-center justify-between text-xs text-violet-200 hover:text-violet-100"
          on:click={() => { showMemories = !showMemories; }}
        >
          <span>Recalled Memories ({latestPlan.recalled_memories!.length})</span>
          <span class="text-[10px]">{showMemories ? '▲' : '▼'}</span>
        </button>
        {#if showMemories}
          <ul class="mt-1 space-y-1">
            {#each latestPlan.recalled_memories! as mem}
              <li class="rounded border border-white/10 bg-black/20 px-2 py-1 text-[10px]">
                <div class="flex items-start justify-between gap-2">
                  <span class="truncate text-slate-300">{mem.content.slice(0, 90)}…</span>
                  <span class="shrink-0 font-mono {mem.score >= 0.9 ? 'text-emerald-300' : mem.score >= 0.75 ? 'text-amber-300' : 'text-slate-400'}">{(mem.score * 100).toFixed(0)}%</span>
                </div>
                {#if mem.classification?.intent}
                  <span class="rounded bg-indigo-500/15 px-1 text-[9px] text-indigo-200">{mem.classification.intent}</span>
                {/if}
              </li>
            {/each}
          </ul>
        {/if}
      {/if}

      {#if (latestPlan.sandbox_results?.length ?? 0) > 0}
        <button
          type="button"
          class="mt-3 flex w-full items-center justify-between text-xs text-amber-200 hover:text-amber-100"
          on:click={() => { showSandbox = !showSandbox; }}
        >
          <span>Sandbox Output ({latestPlan.sandbox_results!.length})</span>
          <span class="text-[10px]">{showSandbox ? '▲' : '▼'}</span>
        </button>
        {#if showSandbox}
          {#each latestPlan.sandbox_results! as result}
            <div class="mt-1 rounded border border-amber-300/20 bg-black/30 p-2 text-[10px]">
              <p class="mb-1 font-semibold text-amber-200">{result.action_title}</p>
              {#if result.stdout}
                <pre class="overflow-x-auto whitespace-pre-wrap text-slate-300">{result.stdout}</pre>
              {/if}
              {#if result.error}
                <pre class="overflow-x-auto whitespace-pre-wrap text-red-300">{result.error}</pre>
              {/if}
            </div>
          {/each}
        {/if}
      {/if}

      {#if latestPlan.memory_stored}
        <p class="mt-2 text-[9px] text-slate-500">Memory stored in vector DB</p>
      {/if}
    </div>
  {/if}

  <div class="rounded-xl border border-white/10 bg-black/20 p-3">
    <p class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-indigo-200">Pending Actions</p>
    <ul class="space-y-2">
      {#each queue as action}
        <li class="rounded-md border border-white/10 bg-white/5 px-2 py-1.5 text-xs">
          <div class="flex items-center justify-between gap-2">
            <span class="truncate">{action.title}</span>
            <span class="rounded border border-indigo-200/20 bg-indigo-500/10 px-1.5 py-0.5 text-[10px] uppercase text-indigo-100">{action.action_type}</span>
            <button
              type="button"
              class="rounded border border-emerald-300/30 bg-emerald-500/15 px-2 py-1 text-[10px] text-emerald-100 hover:bg-emerald-500/25"
              on:click={() => applyAction(action.id)}
            >
              Apply
            </button>
          </div>
        </li>
      {/each}
      {#if queue.length === 0}
        <li class="text-xs text-slate-300">No pending orchestration actions.</li>
      {/if}
    </ul>
  </div>
</div>
