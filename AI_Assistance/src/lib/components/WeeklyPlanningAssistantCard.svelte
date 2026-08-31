<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface WeeklyCalendarItem {
    title: string;
    start_time: string;
    end_time: string;
  }

  interface WeeklyActionItem {
    title: string;
    due_date?: string;
    action_type: string;
  }

  interface WeeklyPlanningAssistant {
    week_label: string;
    priorities: string[];
    calendar_items: WeeklyCalendarItem[];
    pending_actions: WeeklyActionItem[];
    recommendation: string;
  }

  let plan = $state<WeeklyPlanningAssistant | null>(null);
  let busy = $state(false);

  onMount(() => {
    void refreshPlan();

    const offOrch = onRefresh("orchestration", () => {
      void refreshPlan();
    });
    const offCalendar = onRefresh("calendar", () => {
      void refreshPlan();
    });

    return () => {
      offOrch();
      offCalendar();
    };
  });

  async function refreshPlan() {
    busy = true;
    try {
      plan = await invoke<WeeklyPlanningAssistant>("get_weekly_planning_assistant");
    } catch (error) {
      console.error("Failed to load weekly plan", error);
    } finally {
      busy = false;
    }
  }

  async function queueWeeklyPlan() {
    if (!plan) {
      return;
    }

    try {
      await invoke("process_orchestration_signal", {
        signal: {
          source: "weekly_planner",
          content: `Weekly plan for ${plan.week_label}: ${plan.recommendation}`,
          metadata: `priorities=${plan.priorities.length};actions=${plan.pending_actions.length}`,
        },
      });
      emitRefresh("orchestration");
    } catch (error) {
      console.error("Failed to queue weekly planning action", error);
    }
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <div class="flex items-center justify-between">
    <p class="font-display text-[10px] uppercase tracking-[0.1em] text-violet-200">Weekly Horizon</p>
    <button type="button" class="rounded border border-white/15 bg-white/10 px-2 py-1 text-[10px] hover:bg-white/20" disabled={busy} on:click={refreshPlan}>Refresh</button>
  </div>

  {#if plan}
    <p class="text-[11px] font-semibold">{plan.week_label}</p>

    <div class="rounded-lg border border-white/10 bg-white/5 p-2">
      <p class="mb-1 text-[10px] uppercase text-violet-200">Priorities</p>
      <ul class="space-y-1">
        {#each plan.priorities as priority}
          <li class="text-[11px] text-slate-200">- {priority}</li>
        {/each}
      </ul>
    </div>

    <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
      <div class="rounded-lg border border-white/10 bg-black/20 p-2">
        <p class="mb-1 text-[10px] uppercase text-violet-200">Calendar</p>
        <ul class="space-y-1">
          {#each plan.calendar_items.slice(0, 4) as item}
            <li class="text-[10px] text-slate-200">{item.start_time} - {item.title}</li>
          {/each}
          {#if plan.calendar_items.length === 0}
            <li class="text-[10px] text-slate-400">No events in DB for this week.</li>
          {/if}
        </ul>
      </div>

      <div class="rounded-lg border border-white/10 bg-black/20 p-2">
        <p class="mb-1 text-[10px] uppercase text-violet-200">Pending Actions</p>
        <ul class="space-y-1">
          {#each plan.pending_actions.slice(0, 4) as item}
            <li class="text-[10px] text-slate-200">{item.title}</li>
          {/each}
          {#if plan.pending_actions.length === 0}
            <li class="text-[10px] text-slate-400">No pending actions.</li>
          {/if}
        </ul>
      </div>
    </div>

    <div class="rounded-lg border border-violet-300/20 bg-violet-500/10 px-2 py-1.5 text-[10px] text-violet-100">
      {plan.recommendation}
    </div>

    <button type="button" class="rounded-lg bg-violet-400/80 px-3 py-1.5 text-[11px] font-semibold text-slate-950 hover:bg-violet-300" on:click={queueWeeklyPlan}>
      Queue Weekly Planning Action
    </button>
  {:else}
    <p class="text-[11px] text-slate-300">Loading weekly planning assistant...</p>
  {/if}
</div>
