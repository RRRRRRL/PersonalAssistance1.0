<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface SystemStats {
    cpu_usage: number;
    cpu_count: number;
    cpu_brand: string;
    ram_used_gb: number;
    ram_total_gb: number;
    ram_usage_pct: number;
    disk_used_gb: number;
    disk_total_gb: number;
    disk_usage_pct: number;
    uptime_hours: number;
    process_count: number;
  }

  let stats = $state<SystemStats | null>(null);
  let loading = $state(false);

  onMount(() => {
    void refresh();
  });

  async function refresh() {
    if (loading) return;
    loading = true;
    try {
      stats = await invoke<SystemStats>("get_system_stats");
    } catch (e) {
      console.error("Failed to get system stats", e);
    } finally {
      loading = false;
    }
  }

  function barClass(pct: number): string {
    if (pct > 85) return "bg-rose-400";
    if (pct > 60) return "bg-amber-400";
    return "bg-emerald-400";
  }

  function formatUptime(hours: number): string {
    const days = Math.floor(hours / 24);
    const h = Math.floor(hours % 24);
    return days > 0 ? `${days}d ${h}h` : `${h}h`;
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  {#if stats}
    <!-- CPU -->
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <div class="flex items-center justify-between">
        <p class="text-[10px] uppercase text-cyan-200">CPU</p>
        <span class="text-[9px] text-slate-400">{stats.cpu_count} cores</span>
      </div>
      <p class="text-sm font-semibold">{stats.cpu_usage.toFixed(1)}%</p>
      <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-white/10">
        <div class="h-full rounded-full transition-all {barClass(stats.cpu_usage)}" style="width: {Math.min(stats.cpu_usage, 100)}%"></div>
      </div>
      <p class="mt-1 text-[9px] text-slate-400 truncate">{stats.cpu_brand}</p>
    </div>

    <!-- RAM -->
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <div class="flex items-center justify-between">
        <p class="text-[10px] uppercase text-violet-200">Memory</p>
        <span class="text-[9px] text-slate-400">{stats.ram_used_gb.toFixed(1)} / {stats.ram_total_gb.toFixed(1)} GB</span>
      </div>
      <p class="text-sm font-semibold">{stats.ram_usage_pct.toFixed(1)}%</p>
      <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-white/10">
        <div class="h-full rounded-full transition-all {barClass(stats.ram_usage_pct)}" style="width: {Math.min(stats.ram_usage_pct, 100)}%"></div>
      </div>
    </div>

    <!-- Disk -->
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <div class="flex items-center justify-between">
        <p class="text-[10px] uppercase text-amber-200">Disk</p>
        <span class="text-[9px] text-slate-400">{stats.disk_used_gb.toFixed(0)} / {stats.disk_total_gb.toFixed(0)} GB</span>
      </div>
      <p class="text-sm font-semibold">{stats.disk_usage_pct.toFixed(1)}%</p>
      <div class="mt-1 h-1.5 w-full overflow-hidden rounded-full bg-white/10">
        <div class="h-full rounded-full transition-all {barClass(stats.disk_usage_pct)}" style="width: {Math.min(stats.disk_usage_pct, 100)}%"></div>
      </div>
    </div>

    <!-- Footer -->
    <div class="flex items-center justify-between text-[9px] text-slate-400">
      <span>Uptime: {formatUptime(stats.uptime_hours)}</span>
      <span>{stats.process_count} processes</span>
    </div>
  {:else if loading}
    <p class="text-[11px] text-slate-300">Reading system metrics...</p>
  {:else}
    <p class="text-[11px] text-slate-400">Click refresh to load system stats.</p>
  {/if}

  <button
    type="button"
    class="w-full rounded-lg border border-white/10 bg-white/5 px-3 py-1.5 text-[10px] text-slate-300 hover:bg-white/10 disabled:opacity-50"
    disabled={loading}
    on:click={refresh}
  >
    {loading ? "Loading..." : "Refresh"}
  </button>
</div>
