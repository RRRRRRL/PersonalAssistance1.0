<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface ClipboardEntry {
    id: number;
    content: string;
    preview: string;
    copied_at: number;
  }

  let history = $state<ClipboardEntry[]>([]);
  let search = $state("");
  let loading = $state(false);
  let copied = $state<string | null>(null);

  onMount(() => {
    void loadHistory();
  });

  async function loadHistory() {
    loading = true;
    try {
      history = await invoke<ClipboardEntry[]>("get_clipboard_history", {
        query: search.trim() || null,
        limit: 40,
      });
    } catch (e) {
      console.error("Failed to load clipboard history", e);
    } finally {
      loading = false;
    }
  }

  async function copyEntry(content: string) {
    try {
      await invoke("set_clipboard_text", { text: content });
      copied = content.slice(0, 30);
      setTimeout(() => (copied = null), 1500);
    } catch (e) {
      console.error("Failed to copy", e);
    }
  }

  async function clearHistory() {
    try {
      await invoke("clear_clipboard_history");
      history = [];
    } catch (e) {
      console.error("Failed to clear history", e);
    }
  }

  function timeAgo(ts: number): string {
    const diff = Math.floor(Date.now() / 1000) - ts;
    if (diff < 60) return "just now";
    if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
    if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
    return `${Math.floor(diff / 86400)}d ago`;
  }

  let searchTimeout: ReturnType<typeof setTimeout> | null = null;
  function onSearchInput() {
    if (searchTimeout) clearTimeout(searchTimeout);
    searchTimeout = setTimeout(() => loadHistory(), 300);
  }
</script>

<div class="space-y-2 text-xs text-slate-100">
  <!-- Search -->
  <div class="flex gap-1.5">
    <input
      type="text"
      bind:value={search}
      on:input={onSearchInput}
      class="flex-1 rounded-lg border border-white/10 bg-white/5 px-2 py-1 text-[11px] text-white placeholder:text-slate-500 focus:border-cyan-400/40 focus:outline-none"
      placeholder="Search history..."
    />
    <button
      type="button"
      class="rounded-lg border border-white/10 bg-white/5 px-2 py-1 text-[9px] text-slate-400 hover:bg-white/10"
      on:click={clearHistory}
    >Clear</button>
  </div>

  <!-- Copied feedback -->
  {#if copied}
    <div class="rounded-lg border border-emerald-300/20 bg-emerald-500/10 px-2 py-1 text-[10px] text-emerald-200">
      Copied: {copied}...
    </div>
  {/if}

  <!-- History list -->
  <div class="max-h-64 space-y-1 overflow-y-auto">
    {#if loading}
      <p class="text-[10px] text-slate-400">Loading...</p>
    {:else if history.length === 0}
      <p class="text-[10px] text-slate-400">
        {search ? "No matches found." : "Clipboard history is empty. Copy text to build history."}
      </p>
    {:else}
      {#each history as entry}
        <button
          type="button"
          class="w-full rounded-lg border border-white/10 bg-black/20 px-2 py-1.5 text-left hover:bg-white/5 transition-colors"
          on:click={() => copyEntry(entry.content)}
        >
          <p class="truncate text-[11px] text-slate-200">{entry.preview}</p>
          <p class="text-[9px] text-slate-500">{timeAgo(entry.copied_at)}</p>
        </button>
      {/each}
    {/if}
  </div>

  <!-- Manual paste from system clipboard -->
  <button
    type="button"
    class="w-full rounded-lg border border-cyan-300/20 bg-cyan-500/10 px-3 py-1.5 text-[10px] text-cyan-100 hover:bg-cyan-500/20 disabled:opacity-50"
    disabled={loading}
    on:click={loadHistory}
  >
    Refresh History
  </button>
</div>
