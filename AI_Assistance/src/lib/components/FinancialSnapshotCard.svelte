<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface MarketMover {
    ticker: string;
    change_percent: number;
    reason: string;
  }

  interface FinancialSnapshot {
    estimated_portfolio_value: number;
    day_change_percent: number;
    watchlist_overview: string;
    risk_note: string;
    top_movers: MarketMover[];
  }

  let snapshot = $state<FinancialSnapshot | null>(null);
  let busy = $state(false);
  let refreshing = $state(false);

  onMount(() => {
    void refresh();
  });

  async function refresh() {
    try {
      snapshot = await invoke<FinancialSnapshot>("get_financial_snapshot");
    } catch (error) {
      console.error("Failed to load financial snapshot", error);
    }
  }

  async function refreshMarketData() {
    if (refreshing) return;
    refreshing = true;
    try {
      await invoke("refresh_market_data");
      emitRefresh("finance");
      await refresh();
    } catch (error) {
      console.error("Failed to refresh market data", error);
    } finally {
      refreshing = false;
    }
  }

  async function queueRiskReview() {
    if (!snapshot || busy) return;
    busy = true;
    try {
      await invoke("process_orchestration_signal", {
        signal: {
          source: "finance",
          content: `Risk review requested. ${snapshot.risk_note}`,
          metadata: "financial_snapshot",
        },
      });
    } catch (error) {
      console.error("Failed to queue risk review", error);
    } finally {
      busy = false;
    }
  }

  async function runRiskEngine() {
    if (busy) return;
    busy = true;
    try {
      await invoke("evaluate_risk_engine");
      emitRefresh("alerts");
      await refresh();
    } catch (error) {
      console.error("Failed to run risk engine", error);
    } finally {
      busy = false;
    }
  }

  function formatPct(value: number) {
    return `${value > 0 ? "+" : ""}${value.toFixed(2)}%`;
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  {#if snapshot}
    <div class="rounded-lg border border-white/10 bg-white/5 p-3">
      <div class="flex items-center justify-between">
        <p class="font-display text-[10px] uppercase tracking-[0.1em] text-amber-200">Portfolio</p>
        <button
          type="button"
          class="rounded border border-white/10 bg-white/5 px-1.5 py-0.5 text-[9px] text-slate-300 hover:bg-white/10 disabled:opacity-50"
          disabled={refreshing}
          on:click={refreshMarketData}
        >
          {refreshing ? "Refreshing..." : "Refresh Data"}
        </button>
      </div>
      <p class="mt-1 text-base font-semibold">${snapshot.estimated_portfolio_value.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</p>
      <p class="text-[11px] {snapshot.day_change_percent >= 0 ? 'text-emerald-300' : 'text-rose-300'}">
        {formatPct(snapshot.day_change_percent)} P&L
      </p>
      <p class="mt-2 text-[10px] text-slate-300">{snapshot.watchlist_overview}</p>
    </div>

    {#if snapshot.top_movers.length > 0}
      <div class="space-y-2">
        <p class="text-[10px] uppercase tracking-[0.1em] text-slate-400">Top Movers (Live)</p>
        {#each snapshot.top_movers as mover}
          <div class="rounded-lg border border-white/10 bg-black/20 px-2 py-1.5">
            <div class="flex items-center justify-between gap-2">
              <span class="font-semibold">{mover.ticker}</span>
              <span class={mover.change_percent >= 0 ? "text-emerald-300" : "text-rose-300"}>{formatPct(mover.change_percent)}</span>
            </div>
            <div class="text-[10px] text-slate-300">{mover.reason}</div>
          </div>
        {/each}
      </div>
    {/if}

    <div class="rounded-lg border border-amber-300/20 bg-amber-500/10 px-2 py-1.5 text-[10px] text-amber-100">
      {snapshot.risk_note}
    </div>

    <div class="flex gap-2">
      <button
        type="button"
        class="rounded-lg bg-amber-400/80 px-3 py-1.5 text-[11px] font-semibold text-slate-950 hover:bg-amber-300 disabled:opacity-60"
        disabled={busy}
        on:click={queueRiskReview}
      >
        Queue Risk Review
      </button>
      <button
        type="button"
        class="rounded-lg border border-rose-300/30 bg-rose-500/15 px-3 py-1.5 text-[11px] font-semibold text-rose-100 hover:bg-rose-500/25 disabled:opacity-60"
        disabled={busy}
        on:click={runRiskEngine}
      >
        Run Risk Engine
      </button>
    </div>
  {:else}
    <p class="text-[11px] text-slate-300">Loading financial snapshot...</p>
  {/if}
</div>
