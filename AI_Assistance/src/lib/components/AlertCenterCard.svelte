<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface SystemAlert {
    id: number;
    title: string;
    detail: string;
    severity: string;
    source: string;
    status: string;
    occurrences: number;
    last_triggered_at: number;
  }

  let alerts = $state<SystemAlert[]>([]);
  let busy = $state(false);
  const AUTO_EVAL_INTERVAL_MS = 120000;

  onMount(() => {
    void evaluateNow();

    const onFocus = () => {
      void evaluateNow();
    };

    const onVisibilityChange = () => {
      if (document.visibilityState === "visible") {
        void evaluateNow();
      }
    };

    window.addEventListener("focus", onFocus);
    document.addEventListener("visibilitychange", onVisibilityChange);

    const interval = setInterval(() => {
      void evaluateNow();
    }, AUTO_EVAL_INTERVAL_MS);

    return () => {
      window.removeEventListener("focus", onFocus);
      document.removeEventListener("visibilitychange", onVisibilityChange);
      clearInterval(interval);
    };
  });

  async function refreshAlerts() {
    try {
      alerts = await invoke<SystemAlert[]>("get_active_alerts");
    } catch (error) {
      console.error("Failed to load active alerts", error);
    }
  }

  async function evaluateNow() {
    if (busy) {
      return;
    }
    busy = true;
    try {
      alerts = await invoke<SystemAlert[]>("evaluate_alerts");
    } catch (error) {
      console.error("Failed to evaluate alerts", error);
    } finally {
      busy = false;
    }
  }

  async function acknowledge(id: number) {
    try {
      await invoke("acknowledge_alert", { alertId: id });
      await refreshAlerts();
    } catch (error) {
      console.error("Failed to acknowledge alert", error);
    }
  }

  function severityClass(level: string): string {
    if (level === "critical") return "text-rose-200 border-rose-300/30 bg-rose-500/10";
    if (level === "warning") return "text-amber-200 border-amber-300/30 bg-amber-500/10";
    return "text-cyan-200 border-cyan-300/30 bg-cyan-500/10";
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <div class="flex items-center justify-between gap-2">
    <p class="text-[10px] uppercase tracking-[0.1em] text-rose-200">Active Alerts</p>
    <button
      type="button"
      class="rounded border border-white/15 bg-white/10 px-2 py-1 text-[10px] hover:bg-white/20 disabled:opacity-60"
      disabled={busy}
      on:click={evaluateNow}
    >
      Evaluate Now
    </button>
  </div>

  <ul class="space-y-2">
    {#each alerts as alert}
      <li class="rounded-lg border px-2 py-2 {severityClass(alert.severity)}">
        <div class="flex items-start justify-between gap-2">
          <div class="min-w-0">
            <p class="truncate font-semibold">{alert.title}</p>
            <p class="mt-1 text-[10px] opacity-90">{alert.detail}</p>
            <p class="mt-1 text-[10px] opacity-80">{alert.source} | {alert.severity} | hits {alert.occurrences}</p>
          </div>
          <button
            type="button"
            class="rounded border border-white/20 bg-black/20 px-2 py-1 text-[10px] hover:bg-black/35"
            on:click={() => acknowledge(alert.id)}
          >
            Ack
          </button>
        </div>
      </li>
    {/each}
    {#if alerts.length === 0}
      <li class="text-[10px] text-slate-400">No active alerts. Click Evaluate Now to generate threshold-based alerts.</li>
    {/if}
  </ul>
</div>
