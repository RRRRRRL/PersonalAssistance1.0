<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface OrchestrationAction {
    id: number;
    action_type: string;
    title: string;
    due_date?: string;
    status: string;
    note?: string;
  }

  let queue = $state<OrchestrationAction[]>([]);
  let selected = $state<number[]>([]);

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
      selected = selected.filter((id) => queue.some((item) => item.id === id));
    } catch (error) {
      console.error("Failed to refresh queue", error);
    }
  }

  function toggleSelection(id: number) {
    if (selected.includes(id)) {
      selected = selected.filter((item) => item !== id);
      return;
    }
    selected = [...selected, id];
  }

  async function applySelected() {
    if (selected.length === 0) {
      return;
    }

    try {
      await invoke("apply_orchestration_actions", { actionIds: selected });
      selected = [];
      await refreshQueue();
      emitRefresh("orchestration");
      emitRefresh("calendar");
    } catch (error) {
      console.error("Failed to apply selected actions", error);
    }
  }

  async function applyAll() {
    if (queue.length === 0) {
      return;
    }

    try {
      await invoke("apply_orchestration_actions", { actionIds: queue.map((item) => item.id) });
      selected = [];
      await refreshQueue();
      emitRefresh("orchestration");
      emitRefresh("calendar");
    } catch (error) {
      console.error("Failed to apply all actions", error);
    }
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <div class="flex gap-2">
    <button type="button" class="rounded-lg bg-emerald-400/80 px-3 py-1.5 font-semibold text-slate-950 hover:bg-emerald-300 disabled:opacity-50" disabled={selected.length === 0} on:click={applySelected}>Apply Selected</button>
    <button type="button" class="rounded-lg border border-white/15 bg-white/10 px-3 py-1.5 font-semibold hover:bg-white/20 disabled:opacity-50" disabled={queue.length === 0} on:click={applyAll}>Apply All</button>
  </div>

  <div class="space-y-2">
    {#each queue as action}
      <label class="flex items-start gap-2 rounded-lg border border-white/10 bg-white/5 px-2 py-1.5">
        <input type="checkbox" checked={selected.includes(action.id)} on:change={() => toggleSelection(action.id)} />
        <div class="min-w-0">
          <div class="truncate font-semibold">{action.title}</div>
          <div class="text-[10px] text-slate-300">{action.action_type}{#if action.due_date} | due {action.due_date}{/if}</div>
        </div>
      </label>
    {/each}
    {#if queue.length === 0}
      <div class="text-[10px] text-slate-400">No pending actions to approve.</div>
    {/if}
  </div>
</div>
