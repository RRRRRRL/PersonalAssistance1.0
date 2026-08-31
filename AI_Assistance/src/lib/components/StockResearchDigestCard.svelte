<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface StockDigestItem {
    ticker: string;
    headline: string;
    sentiment: string;
    action_hint: string;
  }

  let tickerInput = $state("NVDA,MSFT,AAPL");
  let digest = $state<StockDigestItem[]>([]);
  let busy = $state(false);

  onMount(() => {
    void loadDigest();
  });

  async function loadDigest() {
    busy = true;
    try {
      const tickers = tickerInput
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean);
      digest = await invoke<StockDigestItem[]>("get_stock_research_digest", {
        tickers,
      });
    } catch (error) {
      console.error("Failed to load stock digest", error);
    } finally {
      busy = false;
    }
  }

  async function queueDigestReview(item: StockDigestItem) {
    try {
      await invoke("process_orchestration_signal", {
        signal: {
          source: "stock_digest",
          content: `${item.ticker}: ${item.headline}. ${item.action_hint}`,
          metadata: `sentiment=${item.sentiment}`,
        },
      });
    } catch (error) {
      console.error("Failed to queue stock review action", error);
    }
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <div class="flex gap-2">
    <input
      bind:value={tickerInput}
      class="w-full rounded-lg border border-white/10 bg-white/5 px-2 py-1.5"
      placeholder="Tickers e.g. NVDA,MSFT,AAPL"
    />
    <button type="button" class="rounded-lg bg-cyan-400/80 px-3 py-1.5 font-semibold text-slate-950 hover:bg-cyan-300" disabled={busy} on:click={loadDigest}>
      Refresh
    </button>
  </div>

  <div class="space-y-2">
    {#each digest as item}
      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <div class="flex items-center justify-between gap-2">
          <span class="font-semibold">{item.ticker}</span>
          <span class="rounded border border-white/15 bg-black/20 px-1.5 py-0.5 text-[10px]">{item.sentiment}</span>
        </div>
        <p class="mt-1 text-[11px] text-slate-200">{item.headline}</p>
        <p class="mt-1 text-[10px] text-slate-300">{item.action_hint}</p>
        <button type="button" class="mt-2 rounded border border-cyan-300/30 bg-cyan-500/15 px-2 py-1 text-[10px] text-cyan-100 hover:bg-cyan-500/25" on:click={() => queueDigestReview(item)}>
          Queue Research Follow-up
        </button>
      </div>
    {/each}
  </div>
</div>
