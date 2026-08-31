<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface CalendarEvent {
    title: string;
    start_time: string;
    end_time: string;
    is_deep_work_block: boolean;
  }

  let events = $state<CalendarEvent[]>([]);

  async function loadCalendarEvents() {
    try {
      events = await invoke<CalendarEvent[]>("get_calendar_events");
    } catch (error) {
      console.error("Failed to load calendar events", error);
    }
  }

  onMount(() => {
    void loadCalendarEvents();

    const off = onRefresh("calendar", () => {
      void loadCalendarEvents();
    });

    return () => {
      off();
    };
  });
</script>

<ul class="space-y-2 text-sm text-slate-100">
  {#each events as event}
    <li class="rounded-lg border border-white/10 bg-white/5 px-3 py-2 transition hover:bg-white/10">
      <div class="flex items-start justify-between gap-3">
        <div>
          <p class="font-semibold text-slate-100">{event.title}</p>
          <p class="text-xs text-slate-300">{event.start_time} - {event.end_time}</p>
        </div>
        {#if event.is_deep_work_block}
          <span class="rounded-full bg-violet-300/20 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.08em] text-violet-100">
            Focus
          </span>
        {/if}
      </div>
    </li>
  {/each}
</ul>
