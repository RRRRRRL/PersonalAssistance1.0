<script lang="ts">
  import { onMount } from "svelte";

  const STORAGE_KEY = "ai_assistance_quick_note";
  let note = $state("");
  let lastSavedAt = $state("");

  function stampSavedTime() {
    lastSavedAt = new Date().toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  onMount(() => {
    const cached = localStorage.getItem(STORAGE_KEY);
    if (cached) {
      note = cached;
      stampSavedTime();
    }
  });

  function saveNote() {
    localStorage.setItem(STORAGE_KEY, note);
    stampSavedTime();
  }

  function clearNote() {
    note = "";
    localStorage.removeItem(STORAGE_KEY);
    lastSavedAt = "";
  }
</script>

<div class="space-y-3 text-slate-100">
  <textarea
    bind:value={note}
    class="h-36 w-full resize-none rounded-xl border border-white/10 bg-black/25 p-3 text-sm text-slate-100 outline-none placeholder:text-slate-400 focus:border-rose-300/60"
    placeholder="Capture ideas, snippets, and reminders..."
  ></textarea>

  <div class="flex items-center justify-between">
    <div class="text-xs text-slate-300">
      {#if lastSavedAt}
        Saved at {lastSavedAt}
      {:else}
        Not saved yet
      {/if}
    </div>
    <div class="flex gap-2">
      <button
        type="button"
        class="rounded-lg border border-white/15 bg-white/10 px-3 py-1.5 text-xs text-slate-100 transition hover:bg-white/20"
        on:click={saveNote}
      >
        Save
      </button>
      <button
        type="button"
        class="rounded-lg border border-rose-300/25 bg-rose-500/10 px-3 py-1.5 text-xs text-rose-100 transition hover:bg-rose-500/20"
        on:click={clearNote}
      >
        Clear
      </button>
    </div>
  </div>
</div>
