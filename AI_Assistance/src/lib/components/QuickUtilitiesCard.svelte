<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type Mode = "currency" | "dictionary";

  let mode = $state<Mode>("currency");

  // --- Currency ---
  let baseCurrency = $state("USD");
  let rates = $state<{ currency: string; rate: number }[]>([]);
  let ratesDate = $state("");
  let ratesBusy = $state(false);

  const DEFAULT_SYMBOLS = ["EUR", "GBP", "JPY", "CNY", "CHF"];

  // --- Dictionary ---
  let dictWord = $state("");
  let dictResult = $state<{
    word: string;
    phonetic: string;
    definitions: { part_of_speech: string; definition: string; example: string | null }[];
  } | null>(null);
  let dictBusy = $state(false);

  const CACHE_KEY_RATES = "ai_rates_cache";
  const CACHE_TTL_MS = 60 * 60 * 1000; // 1 hour

  onMount(() => {
    void loadRates();
  });

  async function loadRates() {
    if (ratesBusy) return;

    // Check cache
    try {
      const cached = localStorage.getItem(CACHE_KEY_RATES);
      if (cached) {
        const parsed = JSON.parse(cached);
        if (Date.now() - parsed.ts < CACHE_TTL_MS) {
          rates = parsed.rates;
          ratesDate = parsed.date;
          return;
        }
      }
    } catch { /* ignore */ }

    ratesBusy = true;
    try {
      const result = await invoke<{ base: string; date: string; rates: { currency: string; rate: number }[] }>(
        "get_exchange_rates",
        { base: baseCurrency, symbols: DEFAULT_SYMBOLS }
      );
      rates = result.rates;
      ratesDate = result.date;
      localStorage.setItem(CACHE_KEY_RATES, JSON.stringify({ ts: Date.now(), rates, date: result.date }));
    } catch (e) {
      console.error("Rates load failed", e);
    } finally {
      ratesBusy = false;
    }
  }

  async function doLookup() {
    if (!dictWord.trim() || dictBusy) return;
    dictBusy = true;
    dictResult = null;
    try {
      dictResult = await invoke("lookup_word", { word: dictWord.trim() });
    } catch (e) {
      console.error("Dictionary lookup failed", e);
    } finally {
      dictBusy = false;
    }
  }
</script>

<div class="space-y-2.5 text-xs text-slate-100">
  <!-- Mode toggle -->
  <div class="flex gap-1">
    <button
      type="button"
      class="flex-1 rounded-lg border px-2 py-1.5 text-[10px] font-medium transition {mode === 'currency' ? 'border-amber-300/30 bg-amber-500/15 text-amber-100' : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
      on:click={() => (mode = "currency")}
    >
      💱 Currency
    </button>
    <button
      type="button"
      class="flex-1 rounded-lg border px-2 py-1.5 text-[10px] font-medium transition {mode === 'dictionary' ? 'border-violet-300/30 bg-violet-500/15 text-violet-100' : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
      on:click={() => (mode = "dictionary")}
    >
      📖 Dictionary
    </button>
  </div>

  {#if mode === "currency"}
    <!-- Currency rates -->
    {#if ratesBusy && rates.length === 0}
      <p class="text-[10px] text-slate-400">Loading rates...</p>
    {:else}
      <div class="grid grid-cols-3 gap-1.5 sm:grid-cols-5">
        {#each rates as r}
          <div class="rounded-lg border border-white/10 bg-black/20 px-2 py-1.5 text-center">
            <p class="text-[9px] uppercase tracking-wider text-slate-400">{r.currency}</p>
            <p class="mt-0.5 text-[11px] font-semibold text-white">
              {r.rate >= 1000 ? Math.round(r.rate).toLocaleString() : r.rate.toFixed(r.rate < 1 ? 4 : 2)}
            </p>
          </div>
        {/each}
      </div>
      <p class="text-[9px] text-slate-400">Base: {baseCurrency} · {ratesDate}</p>
    {/if}

  {:else}
    <!-- Dictionary -->
    <div class="flex gap-1.5">
      <input
        bind:value={dictWord}
        type="text"
        placeholder="Look up a word..."
        class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/25 px-2.5 py-1.5 text-[11px] text-slate-100 outline-none placeholder:text-slate-400 focus:border-violet-300/50"
        on:keydown={(e) => e.key === "Enter" && doLookup()}
      />
      <button
        type="button"
        class="rounded-lg border border-violet-300/25 bg-violet-500/15 px-3 py-1.5 text-[10px] font-medium text-violet-100 hover:bg-violet-500/25 disabled:opacity-50"
        disabled={dictBusy}
        on:click={doLookup}
      >
        {dictBusy ? "..." : "Go"}
      </button>
    </div>
    {#if dictResult}
      <div class="space-y-1.5">
        <div class="flex items-baseline gap-2">
          <span class="text-sm font-semibold text-white">{dictResult.word}</span>
          {#if dictResult.phonetic}
            <span class="text-[10px] text-slate-400">{dictResult.phonetic}</span>
          {/if}
        </div>
        {#each dictResult.definitions.slice(0, 3) as def}
          <div class="rounded-lg border border-white/5 bg-black/15 px-2.5 py-1.5">
            <span class="mr-1.5 rounded bg-violet-500/20 px-1 text-[9px] text-violet-200">{def.part_of_speech}</span>
            <span class="text-[11px] text-slate-200">{def.definition}</span>
            {#if def.example}
              <p class="mt-0.5 text-[10px] italic text-slate-400">"{def.example}"</p>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  {/if}
</div>
