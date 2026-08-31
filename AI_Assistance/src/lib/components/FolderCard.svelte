<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface FileHistoryEntry {
    id: number;
    path: string;
    name: string;
    last_accessed: number;
    access_count: number;
  }

  interface RecentAndFrequentFiles {
    recent: FileHistoryEntry[];
    frequent: FileHistoryEntry[];
  }

  interface FileGroupingPolicy {
    root_folder_name: string;
    older_than_days: number;
    updated_at: number;
  }

  interface FileGroupingCandidate {
    file_name: string;
    from_path: string;
    target_folder: string;
    reason: string;
  }

  interface FileGroupingPreview {
    policy: FileGroupingPolicy;
    candidate_count: number;
    candidates: FileGroupingCandidate[];
  }

  interface FileGroupingResult {
    policy: FileGroupingPolicy;
    total_candidates: number;
    moved_count: number;
    skipped_count: number;
    created_folders: string[];
    errors: string[];
  }

  let desktopFiles = $state<string[]>([]);
  let recentFiles = $state<FileHistoryEntry[]>([]);
  let frequentFiles = $state<FileHistoryEntry[]>([]);
  let rootFolderName = $state("AI_AutoGroup");
  let olderThanDays = $state(90);
  let preview = $state<FileGroupingPreview | null>(null);
  let lastRun = $state<FileGroupingResult | null>(null);
  let groupingBusy = $state(false);
  let fileActionError = $state("");

  onMount(async () => {
    try {
      desktopFiles = await invoke<string[]>("get_desktop_files");
      const data = await invoke<RecentAndFrequentFiles>("get_recent_and_frequent_files");
      const policy = await invoke<FileGroupingPolicy>("get_file_grouping_policy");
      recentFiles = data.recent;
      frequentFiles = data.frequent;
      rootFolderName = policy.root_folder_name;
      olderThanDays = policy.older_than_days;
    } catch (error) {
      console.error("Failed to load file widgets", error);
    }
  });

  async function openFile(file: FileHistoryEntry) {
    fileActionError = "";
    try {
      await invoke("open_file", { path: file.path });
      await invoke("log_file_access", { path: file.path, name: file.name });
      const data = await invoke<RecentAndFrequentFiles>("get_recent_and_frequent_files");
      recentFiles = data.recent;
      frequentFiles = data.frequent;
    } catch (error) {
      fileActionError = error instanceof Error ? error.message : String(error);
      console.error("Failed to open file", error);
    }
  }

  function iconFor(name: string): string {
    const ext = name.toLowerCase().split(".").pop() ?? "";
    if (["png", "jpg", "jpeg", "gif", "webp", "svg"].includes(ext)) return "IMG";
    if (["ts", "js", "rs", "py", "cpp", "c", "java"].includes(ext)) return "CODE";
    if (["pdf", "md", "txt", "doc", "docx"].includes(ext)) return "DOC";
    return "FILE";
  }

  async function savePolicy() {
    if (groupingBusy) {
      return;
    }

    groupingBusy = true;
    try {
      const policy = await persistPolicy();
      rootFolderName = policy.root_folder_name;
      olderThanDays = policy.older_than_days;
    } catch (error) {
      console.error("Failed to save grouping policy", error);
    } finally {
      groupingBusy = false;
    }
  }

  async function previewBatchGrouping() {
    if (groupingBusy) {
      return;
    }

    groupingBusy = true;
    try {
      await persistPolicy();
      preview = await invoke<FileGroupingPreview>("preview_file_grouping_batch");
    } catch (error) {
      console.error("Failed to preview file grouping", error);
    } finally {
      groupingBusy = false;
    }
  }

  async function runBatchGrouping() {
    if (groupingBusy) {
      return;
    }

    groupingBusy = true;
    try {
      await persistPolicy();
      lastRun = await invoke<FileGroupingResult>("run_file_grouping_batch");
      preview = await invoke<FileGroupingPreview>("preview_file_grouping_batch");
      desktopFiles = await invoke<string[]>("get_desktop_files");
      const data = await invoke<RecentAndFrequentFiles>("get_recent_and_frequent_files");
      recentFiles = data.recent;
      frequentFiles = data.frequent;
    } catch (error) {
      console.error("Failed to run file grouping batch", error);
    } finally {
      groupingBusy = false;
    }
  }

  function persistPolicy() {
    return invoke<FileGroupingPolicy>("save_file_grouping_policy", {
      input: {
        root_folder_name: rootFolderName,
        older_than_days: olderThanDays,
      },
    });
  }
</script>

<div class="space-y-4 text-sm text-slate-200">
  <section>
    <h3 class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-slate-300">Desktop Snapshot</h3>
    <ul class="space-y-2">
      {#each desktopFiles.slice(0, 5) as file}
        <li class="flex items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 transition hover:bg-white/10">
          <span class="rounded bg-cyan-400/20 px-1.5 py-0.5 text-[10px] font-semibold text-cyan-200">{iconFor(file)}</span>
          <span class="truncate">{file}</span>
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <h3 class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-slate-300">Recent</h3>
    <ul class="space-y-2">
      {#each recentFiles as file}
        <li>
          <button
            class="flex w-full items-center gap-2 rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 text-left transition hover:bg-white/10"
            on:click={() => openFile(file)}
            type="button"
          >
            <span class="rounded bg-emerald-400/20 px-1.5 py-0.5 text-[10px] font-semibold text-emerald-200">{iconFor(file.name)}</span>
            <span class="truncate">{file.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  </section>

  <section>
    <h3 class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-slate-300">Frequent</h3>
    <ul class="space-y-2">
      {#each frequentFiles as file}
        <li>
          <button
            class="flex w-full items-center justify-between gap-2 rounded-lg border border-white/10 bg-white/5 px-2 py-1.5 text-left transition hover:bg-white/10"
            on:click={() => openFile(file)}
            type="button"
          >
            <span class="truncate">{file.name}</span>
            <span class="shrink-0 rounded bg-amber-400/20 px-1.5 py-0.5 text-[10px] font-semibold text-amber-200">{file.access_count}x</span>
          </button>
        </li>
      {/each}
    </ul>
    {#if fileActionError}
      <p class="mt-2 text-[11px] text-rose-200" role="alert">{fileActionError}</p>
    {/if}
  </section>

  <section class="rounded-lg border border-white/10 bg-white/5 p-3">
    <h3 class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-slate-300">Batch Auto Grouping</h3>
    <p class="mb-2 text-[11px] text-slate-300">
      Groups desktop files older than a threshold into a managed folder by related name or type.
    </p>
    <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
      <input
        bind:value={rootFolderName}
        class="rounded border border-white/10 bg-black/20 px-2 py-1.5 text-[12px]"
        placeholder="Root folder name"
      />
      <input
        type="number"
        bind:value={olderThanDays}
        class="rounded border border-white/10 bg-black/20 px-2 py-1.5 text-[12px]"
        min="7"
        max="3650"
      />
    </div>

    <div class="mt-2 flex flex-wrap gap-2">
      <button type="button" class="rounded border border-cyan-300/30 bg-cyan-500/15 px-2 py-1 text-[10px] text-cyan-100 hover:bg-cyan-500/25 disabled:opacity-60" disabled={groupingBusy} on:click={savePolicy}>
        Save Policy
      </button>
      <button type="button" class="rounded border border-amber-300/30 bg-amber-500/15 px-2 py-1 text-[10px] text-amber-100 hover:bg-amber-500/25 disabled:opacity-60" disabled={groupingBusy} on:click={previewBatchGrouping}>
        Preview Batch
      </button>
      <button type="button" class="rounded border border-emerald-300/30 bg-emerald-500/15 px-2 py-1 text-[10px] text-emerald-100 hover:bg-emerald-500/25 disabled:opacity-60" disabled={groupingBusy} on:click={runBatchGrouping}>
        Run Grouping
      </button>
    </div>

    {#if preview}
      <div class="mt-3 rounded border border-white/10 bg-black/20 p-2 text-[11px] text-slate-200">
        <p class="font-semibold text-amber-200">Preview: {preview.candidate_count} candidate files</p>
        <ul class="mt-1 space-y-1">
          {#each preview.candidates.slice(0, 5) as candidate}
            <li class="truncate">
              {candidate.file_name} -> {candidate.target_folder}
            </li>
          {/each}
          {#if preview.candidate_count > 5}
            <li class="text-slate-400">+ {preview.candidate_count - 5} more</li>
          {/if}
        </ul>
      </div>
    {/if}

    {#if lastRun}
      <div class="mt-2 rounded border border-white/10 bg-black/20 p-2 text-[11px] text-slate-200">
        <p class="font-semibold text-emerald-200">Last Run</p>
        <p>Total: {lastRun.total_candidates}, moved: {lastRun.moved_count}, skipped: {lastRun.skipped_count}</p>
        {#if lastRun.errors.length > 0}
          <p class="mt-1 text-rose-200">Errors: {lastRun.errors.length}</p>
        {/if}
      </div>
    {/if}
  </section>
</div>
