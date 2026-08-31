<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface Position {
    id: number;
    symbol: string;
    quantity: number;
    avg_cost: number;
    side: string;
    status: string;
    current_price?: number;
    unrealized_pnl?: number;
    unrealized_pnl_pct?: number;
    market_value?: number;
    realized_pnl: number;
  }

  interface PortfolioSummary {
    total_market_value: number;
    total_cost_basis: number;
    unrealized_pnl: number;
    unrealized_pnl_pct: number;
    realized_pnl: number;
    positions: Position[];
    open_count: number;
    top_movers: { ticker: string; change_percent: number; reason: string }[];
    risk_note: string;
    watchlist_overview: string;
  }

  interface WatchlistItem {
    id: number;
    symbol: string;
    name: string;
    asset_class: string;
    venue: string;
    notes?: string;
    is_active: boolean;
  }

  interface MarketQuoteView {
    symbol: string;
    price: number;
    change_percent?: number;
    volume?: number;
    high?: number;
    low?: number;
  }

  let portfolio = $state<PortfolioSummary | null>(null);
  let watchlist = $state<WatchlistItem[]>([]);
  let quotes = $state<MarketQuoteView[]>([]);
  let busy = $state(false);
  let tab = $state<"positions" | "watchlist">("positions");

  // New position form
  let posSymbol = $state("");
  let posQuantity = $state(0);
  let posCost = $state(0);

  // New watchlist form
  let wlSymbol = $state("");
  let wlName = $state("");
  let wlAssetClass = $state("equity");

  onMount(() => {
    void loadPortfolio();
    void loadWatchlist();
    void loadQuotes();

    const off = onRefresh("finance", () => {
      void loadPortfolio();
      void loadQuotes();
    });
    return () => off();
  });

  async function loadPortfolio() {
    try {
      portfolio = await invoke<PortfolioSummary>("get_portfolio_summary");
    } catch (e) {
      console.error("Failed to load portfolio", e);
    }
  }

  async function loadWatchlist() {
    try {
      watchlist = await invoke<WatchlistItem[]>("get_watchlist");
    } catch (e) {
      console.error("Failed to load watchlist", e);
    }
  }

  async function loadQuotes() {
    try {
      quotes = await invoke<MarketQuoteView[]>("get_cached_quotes");
    } catch (e) {
      console.error("Failed to load quotes", e);
    }
  }

  async function refreshData() {
    if (busy) return;
    busy = true;
    try {
      await invoke("refresh_market_data");
      await loadPortfolio();
      await loadQuotes();
      emitRefresh("finance");
    } catch (e) {
      console.error("Failed to refresh market data", e);
    } finally {
      busy = false;
    }
  }

  async function openPosition() {
    if (!posSymbol.trim() || posQuantity <= 0 || posCost <= 0 || busy) return;
    busy = true;
    try {
      await invoke("open_position", {
        input: {
          symbol: posSymbol.toUpperCase(),
          quantity: posQuantity,
          avg_cost: posCost,
          side: "long",
        },
      });
      posSymbol = "";
      posQuantity = 0;
      posCost = 0;
      await loadPortfolio();
      emitRefresh("finance");
    } catch (e) {
      console.error("Failed to open position", e);
    } finally {
      busy = false;
    }
  }

  async function closePosition(id: number) {
    if (busy) return;
    busy = true;
    try {
      await invoke("close_position", { positionId: id });
      await loadPortfolio();
      emitRefresh("finance");
    } catch (e) {
      console.error("Failed to close position", e);
    } finally {
      busy = false;
    }
  }

  async function addToWatchlist() {
    if (!wlSymbol.trim() || busy) return;
    busy = true;
    try {
      await invoke("add_watchlist_item", {
        input: {
          symbol: wlSymbol.toUpperCase(),
          name: wlName || wlSymbol.toUpperCase(),
          asset_class: wlAssetClass,
          venue: wlAssetClass === "crypto" ? "coingecko" : "yahoo",
          notes: null,
        },
      });
      wlSymbol = "";
      wlName = "";
      await loadWatchlist();
    } catch (e) {
      console.error("Failed to add watchlist item", e);
    } finally {
      busy = false;
    }
  }

  async function removeFromWatchlist(symbol: string) {
    try {
      await invoke("remove_watchlist_item", { symbol });
      await loadWatchlist();
    } catch (e) {
      console.error("Failed to remove from watchlist", e);
    }
  }

  function formatPct(value: number) {
    return `${value > 0 ? "+" : ""}${value.toFixed(2)}%`;
  }

  function pnlClass(value?: number): string {
    if (value === undefined || value === null) return "text-slate-400";
    return value >= 0 ? "text-emerald-300" : "text-rose-300";
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <!-- Tab nav -->
  <div class="flex gap-1 rounded-lg border border-white/10 bg-black/20 p-0.5">
    <button
      type="button"
      class="flex-1 rounded-md px-2 py-1 text-[10px] font-semibold transition {tab === 'positions' ? 'bg-amber-400/20 text-amber-100' : 'text-slate-400 hover:text-white'}"
      on:click={() => (tab = "positions")}
    >Positions ({portfolio?.open_count ?? 0})</button>
    <button
      type="button"
      class="flex-1 rounded-md px-2 py-1 text-[10px] font-semibold transition {tab === 'watchlist' ? 'bg-cyan-400/20 text-cyan-100' : 'text-slate-400 hover:text-white'}"
      on:click={() => (tab = "watchlist")}
    >Watchlist ({watchlist.filter(w => w.is_active).length})</button>
  </div>

  {#if tab === "positions"}
    <!-- Portfolio summary bar -->
    {#if portfolio}
      <div class="grid grid-cols-2 gap-2">
        <div class="rounded-lg border border-white/10 bg-white/5 p-2">
          <p class="text-[9px] uppercase text-slate-400">Market Value</p>
          <p class="text-sm font-semibold">${portfolio.total_market_value.toFixed(2)}</p>
        </div>
        <div class="rounded-lg border border-white/10 bg-white/5 p-2">
          <p class="text-[9px] uppercase text-slate-400">Unrealized P&L</p>
          <p class="text-sm font-semibold {pnlClass(portfolio.unrealized_pnl)}">
            {formatPct(portfolio.unrealized_pnl_pct)}
          </p>
          <p class="text-[9px] text-slate-400">${portfolio.unrealized_pnl.toFixed(2)}</p>
        </div>
      </div>

      <!-- Positions list -->
      {#if portfolio.positions.length > 0}
        <div class="space-y-1.5">
          {#each portfolio.positions as pos}
            <div class="rounded-lg border border-white/10 bg-black/20 px-2 py-1.5">
              <div class="flex items-center justify-between">
                <div>
                  <span class="font-semibold">{pos.symbol}</span>
                  <span class="ml-1 text-[9px] text-slate-400">{pos.quantity} @ ${pos.avg_cost.toFixed(2)}</span>
                </div>
                <div class="text-right">
                  <p class="text-[10px] font-semibold {pnlClass(pos.unrealized_pnl)}">
                    {pos.unrealized_pnl !== null ? `$${pos.unrealized_pnl?.toFixed(2)}` : "—"}
                  </p>
                  {#if pos.unrealized_pnl_pct !== null}
                    <p class="text-[9px] {pnlClass(pos.unrealized_pnl_pct)}">{formatPct(pos.unrealized_pnl_pct!)}</p>
                  {/if}
                </div>
              </div>
              <div class="mt-1 flex items-center justify-between">
                <span class="text-[9px] text-slate-400">
                  {pos.current_price ? `$${pos.current_price.toFixed(2)}` : "No price"}
                  {pos.market_value ? ` · MV $${pos.market_value.toFixed(0)}` : ""}
                </span>
                <button
                  type="button"
                  class="rounded border border-rose-300/20 px-1.5 py-0.5 text-[8px] text-rose-200 hover:bg-rose-500/15 disabled:opacity-50"
                  disabled={busy}
                  on:click={() => closePosition(pos.id)}
                >Close</button>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <p class="text-[10px] text-slate-400">No open positions. Add your first position below.</p>
      {/if}
    {/if}

    <!-- Open position form -->
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <p class="mb-1.5 text-[10px] uppercase text-amber-200">Open Position</p>
      <div class="grid grid-cols-3 gap-1.5">
        <input bind:value={posSymbol} class="rounded border border-white/10 bg-white/5 px-2 py-1 text-[11px]" placeholder="AAPL" />
        <input type="number" bind:value={posQuantity} step="0.01" class="rounded border border-white/10 bg-white/5 px-2 py-1 text-[11px]" placeholder="Qty" />
        <input type="number" bind:value={posCost} step="0.01" class="rounded border border-white/10 bg-white/5 px-2 py-1 text-[11px]" placeholder="Avg Cost" />
      </div>
      <button
        type="button"
        class="mt-1.5 rounded bg-amber-400/80 px-3 py-1 text-[10px] font-semibold text-slate-950 hover:bg-amber-300 disabled:opacity-50"
        disabled={busy}
        on:click={openPosition}
      >Open Position</button>
    </div>

  {:else if tab === "watchlist"}
    <!-- Watchlist items -->
    {#if watchlist.filter(w => w.is_active).length > 0}
      <div class="space-y-1.5">
        {#each watchlist.filter(w => w.is_active) as item}
          {@const quote = quotes.find(q => q.symbol === item.symbol)}
          <div class="rounded-lg border border-white/10 bg-black/20 px-2 py-1.5">
            <div class="flex items-center justify-between">
              <div>
                <span class="font-semibold">{item.symbol}</span>
                <span class="ml-1 text-[9px] text-slate-400">{item.asset_class} · {item.venue}</span>
              </div>
              <div class="flex items-center gap-1">
                {#if quote}
                  <span class="text-[10px]">${quote.price.toFixed(2)}</span>
                  {#if quote.change_percent !== null}
                    <span class="text-[9px] {pnlClass(quote.change_percent!)}">{formatPct(quote.change_percent!)}</span>
                  {/if}
                {:else}
                  <span class="text-[9px] text-slate-500">—</span>
                {/if}
                <button
                  type="button"
                  class="ml-1 rounded px-1 py-0.5 text-[8px] text-slate-500 hover:bg-white/10 hover:text-rose-300"
                  on:click={() => removeFromWatchlist(item.symbol)}
                >&times;</button>
              </div>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <p class="text-[10px] text-slate-400">Watchlist is empty. Add instruments below to track live prices.</p>
    {/if}

    <!-- Add to watchlist form -->
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <p class="mb-1.5 text-[10px] uppercase text-cyan-200">Add Instrument</p>
      <div class="grid grid-cols-2 gap-1.5">
        <input bind:value={wlSymbol} class="rounded border border-white/10 bg-white/5 px-2 py-1 text-[11px]" placeholder="AAPL or bitcoin" />
        <input bind:value={wlName} class="rounded border border-white/10 bg-white/5 px-2 py-1 text-[11px]" placeholder="Name (optional)" />
        <select bind:value={wlAssetClass} class="rounded border border-white/10 bg-white/5 px-2 py-1 text-[11px]">
          <option value="equity">Equity</option>
          <option value="etf">ETF</option>
          <option value="crypto">Crypto</option>
        </select>
      </div>
      <button
        type="button"
        class="mt-1.5 rounded bg-cyan-400/80 px-3 py-1 text-[10px] font-semibold text-slate-950 hover:bg-cyan-300 disabled:opacity-50"
        disabled={busy}
        on:click={addToWatchlist}
      >Add to Watchlist</button>
    </div>
  {/if}

  <!-- Refresh button -->
  <button
    type="button"
    class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-[10px] text-slate-300 hover:bg-white/10 disabled:opacity-50"
    disabled={busy}
    on:click={refreshData}
  >
    {busy ? "Refreshing..." : "Refresh Market Data"}
  </button>
</div>
