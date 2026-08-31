<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  type Tab = "scan" | "breach" | "network" | "password";

  let activeTab = $state<Tab>("scan");

  // --- Link Scanner ---
  let scanUrl = $state("");
  let scanResult = $state<{ message: string; is_malicious: boolean } | null>(null);
  let scanBusy = $state(false);

  // --- Breach Checker ---
  let breachEmail = $state("");
  let breachResult = $state<{ message: string; breach_count: number; breaches: { name: string }[] } | null>(null);
  let breachBusy = $state(false);

  // --- Network Info ---
  let netInfo = $state<{ ip: string; country: string; tls_version: string; http_protocol: string; asn: string } | null>(null);
  let netBusy = $state(false);

  // --- Password Generator ---
  let pwLength = $state(20);
  let pwUpper = $state(true);
  let pwNumbers = $state(true);
  let pwSymbols = $state(true);
  let pwResult = $state<{ password: string; entropy_bits: number; strength: string } | null>(null);
  let pwCopied = $state(false);

  onMount(() => {
    void loadNetworkInfo();
    void generatePassword();
  });

  async function doScan() {
    if (!scanUrl.trim() || scanBusy) return;
    scanBusy = true;
    scanResult = null;
    try {
      scanResult = await invoke("scan_url_safety", { url: scanUrl.trim() });
    } catch (e) {
      scanResult = { message: `Error: ${e}`, is_malicious: false };
    } finally {
      scanBusy = false;
    }
  }

  async function doBreachCheck() {
    if (!breachEmail.trim() || breachBusy) return;
    breachBusy = true;
    breachResult = null;
    try {
      breachResult = await invoke("check_email_breach", { email: breachEmail.trim() });
    } catch (e) {
      breachResult = { message: `Error: ${e}`, breach_count: 0, breaches: [] };
    } finally {
      breachBusy = false;
    }
  }

  async function loadNetworkInfo() {
    if (netBusy) return;
    netBusy = true;
    try {
      netInfo = await invoke("get_network_info");
    } catch (e) {
      console.error("Network info failed", e);
    } finally {
      netBusy = false;
    }
  }

  async function generatePassword() {
    try {
      pwResult = await invoke("generate_secure_password", {
        length: pwLength,
        useUppercase: pwUpper,
        useNumbers: pwNumbers,
        useSymbols: pwSymbols,
      });
      pwCopied = false;
    } catch (e) {
      console.error("Password gen failed", e);
    }
  }

  async function copyPassword() {
    if (!pwResult) return;
    await navigator.clipboard.writeText(pwResult.password);
    pwCopied = true;
    setTimeout(() => (pwCopied = false), 2000);
  }

  const tabs: { id: Tab; label: string; icon: string }[] = [
    { id: "scan", label: "URL", icon: "🔗" },
    { id: "breach", label: "Breach", icon: "🛡" },
    { id: "network", label: "Net", icon: "📡" },
    { id: "password", label: "Pass", icon: "🔑" },
  ];
</script>

<div class="space-y-3 text-xs text-slate-100">
  <!-- Tab bar -->
  <div class="flex gap-1">
    {#each tabs as tab}
      <button
        type="button"
        class="flex-1 rounded-lg border px-2 py-1.5 text-[10px] font-medium transition {activeTab === tab.id
          ? 'border-sky-300/30 bg-sky-500/15 text-sky-100'
          : 'border-white/10 bg-white/5 text-slate-300 hover:bg-white/10'}"
        on:click={() => (activeTab = tab.id)}
      >
        {tab.icon} {tab.label}
      </button>
    {/each}
  </div>

  <!-- Tab: URL Scanner -->
  {#if activeTab === "scan"}
    <div class="space-y-2">
      <div class="flex gap-1.5">
        <input
          bind:value={scanUrl}
          type="url"
          placeholder="Paste URL to check..."
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/25 px-2.5 py-1.5 text-[11px] text-slate-100 outline-none placeholder:text-slate-400 focus:border-sky-300/50"
          on:keydown={(e) => e.key === "Enter" && doScan()}
        />
        <button
          type="button"
          class="rounded-lg border border-sky-300/25 bg-sky-500/15 px-3 py-1.5 text-[10px] font-medium text-sky-100 hover:bg-sky-500/25 disabled:opacity-50"
          disabled={scanBusy}
          on:click={doScan}
        >
          {scanBusy ? "..." : "Scan"}
        </button>
      </div>
      {#if scanResult}
        <p class="rounded-lg border px-2.5 py-2 text-[11px] {scanResult.is_malicious ? 'border-rose-300/30 bg-rose-500/10 text-rose-200' : 'border-emerald-300/20 bg-emerald-500/10 text-emerald-200'}">
          {scanResult.message}
        </p>
      {/if}
    </div>

  <!-- Tab: Breach Checker -->
  {:else if activeTab === "breach"}
    <div class="space-y-2">
      <div class="flex gap-1.5">
        <input
          bind:value={breachEmail}
          type="email"
          placeholder="Email to check..."
          class="min-w-0 flex-1 rounded-lg border border-white/10 bg-black/25 px-2.5 py-1.5 text-[11px] text-slate-100 outline-none placeholder:text-slate-400 focus:border-sky-300/50"
          on:keydown={(e) => e.key === "Enter" && doBreachCheck()}
        />
        <button
          type="button"
          class="rounded-lg border border-sky-300/25 bg-sky-500/15 px-3 py-1.5 text-[10px] font-medium text-sky-100 hover:bg-sky-500/25 disabled:opacity-50"
          disabled={breachBusy}
          on:click={doBreachCheck}
        >
          {breachBusy ? "..." : "Check"}
        </button>
      </div>
      {#if breachResult}
        <div class="space-y-1.5">
          <p class="rounded-lg border px-2.5 py-2 text-[11px] {breachResult.breach_count > 0 ? 'border-amber-300/30 bg-amber-500/10 text-amber-200' : 'border-emerald-300/20 bg-emerald-500/10 text-emerald-200'}">
            {breachResult.message}
          </p>
          {#if breachResult.breaches.length > 0}
            <div class="flex flex-wrap gap-1">
              {#each breachResult.breaches as b}
                <span class="rounded border border-rose-300/20 bg-rose-500/10 px-1.5 py-0.5 text-[9px] text-rose-200">{b.name}</span>
              {/each}
            </div>
          {/if}
        </div>
      {/if}
    </div>

  <!-- Tab: Network Info -->
  {:else if activeTab === "network"}
    <div class="space-y-2">
      {#if netBusy}
        <p class="text-[10px] text-slate-400">Detecting...</p>
      {:else if netInfo}
        <div class="grid grid-cols-2 gap-1.5">
          <div class="rounded-lg border border-white/10 bg-black/20 px-2.5 py-2">
            <p class="text-[9px] uppercase tracking-wider text-slate-400">IP</p>
            <p class="mt-0.5 truncate text-[11px] font-medium text-white">{netInfo.ip || "N/A"}</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-2.5 py-2">
            <p class="text-[9px] uppercase tracking-wider text-slate-400">Location</p>
            <p class="mt-0.5 text-[11px] font-medium text-white">{netInfo.country || "N/A"}</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-2.5 py-2">
            <p class="text-[9px] uppercase tracking-wider text-slate-400">TLS</p>
            <p class="mt-0.5 text-[11px] font-medium text-white">{netInfo.tls_version || "N/A"}</p>
          </div>
          <div class="rounded-lg border border-white/10 bg-black/20 px-2.5 py-2">
            <p class="text-[9px] uppercase tracking-wider text-slate-400">Protocol</p>
            <p class="mt-0.5 text-[11px] font-medium text-white">{netInfo.http_protocol || "N/A"}</p>
          </div>
        </div>
        <button
          type="button"
          class="w-full rounded-lg border border-white/10 bg-white/5 py-1.5 text-[10px] text-slate-300 hover:bg-white/10"
          on:click={loadNetworkInfo}
        >
          Refresh
        </button>
      {:else}
        <p class="text-[10px] text-slate-400">Failed to detect network info.</p>
      {/if}
    </div>

  <!-- Tab: Password Generator -->
  {:else if activeTab === "password"}
    <div class="space-y-2.5">
      <!-- Length slider -->
      <div class="flex items-center gap-2">
        <span class="text-[10px] text-slate-300">Length</span>
        <input
          bind:value={pwLength}
          type="range"
          min="8"
          max="64"
          class="flex-1 accent-sky-400"
          on:input={generatePassword}
        />
        <span class="w-6 text-right text-[11px] font-medium text-white">{pwLength}</span>
      </div>
      <!-- Toggles -->
      <div class="flex gap-2 text-[10px]">
        {#each [["A-Z", pwUpper, "pwUpper"], ["0-9", pwNumbers, "pwNumbers"], ["!@#", pwSymbols, "pwSymbols"]] as [label, val, key]}
          <button
            type="button"
            class="rounded-lg border px-2 py-1 transition {val ? 'border-sky-300/30 bg-sky-500/15 text-sky-100' : 'border-white/10 bg-white/5 text-slate-400'}"
            on:click={() => {
              if (key === "pwUpper") pwUpper = !pwUpper;
              else if (key === "pwNumbers") pwNumbers = !pwNumbers;
              else pwSymbols = !pwSymbols;
              generatePassword();
            }}
          >
            {label}
          </button>
        {/each}
      </div>
      <!-- Password display -->
      {#if pwResult}
        <div class="flex items-center gap-1.5">
          <code class="min-w-0 flex-1 truncate rounded-lg border border-white/10 bg-black/30 px-2.5 py-2 text-[11px] text-emerald-200">{pwResult.password}</code>
          <button
            type="button"
            class="shrink-0 rounded-lg border border-white/15 bg-white/10 px-2.5 py-2 text-[10px] hover:bg-white/20"
            on:click={copyPassword}
          >
            {pwCopied ? "✓" : "Copy"}
          </button>
        </div>
        <div class="flex items-center justify-between text-[10px]">
          <span class="text-slate-400">{pwResult.entropy_bits} bits entropy</span>
          <span class="{pwResult.strength === 'Very Strong' ? 'text-emerald-200' : pwResult.strength === 'Strong' ? 'text-sky-200' : pwResult.strength === 'Moderate' ? 'text-amber-200' : 'text-rose-200'}">
            {pwResult.strength}
          </span>
        </div>
      {/if}
    </div>
  {/if}
</div>
