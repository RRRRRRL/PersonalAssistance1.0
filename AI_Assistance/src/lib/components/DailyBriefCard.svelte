<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface WeatherSummary {
    temperature_c: number;
    feels_like_c: number;
    humidity_pct: number;
    wind_kmh: number;
    weather_code: number;
    is_day: boolean;
    description: string;
    icon: string;
  }

  interface HolidayEntry {
    date: string;
    name: string;
    local_name: string;
  }

  let weather = $state<WeatherSummary | null>(null);
  let nextHoliday = $state<HolidayEntry | null>(null);
  let loading = $state(true);
  let locationLabel = $state("");

  const STORAGE_KEY_LAT = "ai_brief_lat";
  const STORAGE_KEY_LON = "ai_brief_lon";
  const STORAGE_KEY_COUNTRY = "ai_brief_country";
  const CACHE_KEY = "ai_brief_cache";
  const CACHE_TTL_MS = 30 * 60 * 1000; // 30 min

  onMount(() => {
    void loadBrief();
  });

  async function loadBrief() {
    loading = true;

    // Try cache first
    try {
      const cached = localStorage.getItem(CACHE_KEY);
      if (cached) {
        const parsed = JSON.parse(cached);
        if (Date.now() - parsed.ts < CACHE_TTL_MS) {
          weather = parsed.weather;
          nextHoliday = parsed.nextHoliday;
          locationLabel = parsed.locationLabel ?? "";
          loading = false;
          return;
        }
      }
    } catch { /* ignore */ }

    const lat = parseFloat(localStorage.getItem(STORAGE_KEY_LAT) ?? "40.71");
    const lon = parseFloat(localStorage.getItem(STORAGE_KEY_LON) ?? "-74.01");
    const country = localStorage.getItem(STORAGE_KEY_COUNTRY) ?? "US";

    try {
      const [w, holidays] = await Promise.all([
        invoke<WeatherSummary>("get_weather_summary", { lat, lon }),
        invoke<HolidayEntry[]>("get_public_holidays", {
          year: new Date().getFullYear(),
          country,
        }),
      ]);

      weather = w;

      const today = new Date().toISOString().slice(0, 10);
      const upcoming = holidays
        .filter((h) => h.date >= today)
        .sort((a, b) => a.date.localeCompare(b.date));
      nextHoliday = upcoming[0] ?? null;
      locationLabel = `${lat.toFixed(1)}°, ${lon.toFixed(1)}°`;

      localStorage.setItem(
        CACHE_KEY,
        JSON.stringify({ ts: Date.now(), weather: w, nextHoliday: nextHoliday, locationLabel })
      );
    } catch (e) {
      console.error("Daily brief load failed", e);
    } finally {
      loading = false;
    }
  }

  function daysUntil(dateStr: string): number {
    const target = new Date(dateStr + "T00:00:00");
    const now = new Date();
    now.setHours(0, 0, 0, 0);
    return Math.ceil((target.getTime() - now.getTime()) / 86400000);
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  {#if loading}
    <div class="flex items-center gap-2 text-[10px] text-slate-400">
      <span class="inline-block h-3 w-3 animate-spin rounded-full border border-slate-400 border-t-transparent"></span>
      Loading daily brief...
    </div>
  {:else}
    <!-- Weather row -->
    {#if weather}
      <div class="flex items-center gap-3">
        <span class="text-2xl leading-none">{weather.icon}</span>
        <div class="min-w-0 flex-1">
          <div class="flex items-baseline gap-2">
            <span class="text-xl font-semibold text-white">{Math.round(weather.temperature_c)}°C</span>
            <span class="text-[10px] text-slate-300">{weather.description}</span>
          </div>
          <div class="mt-0.5 flex gap-3 text-[10px] text-slate-300">
            <span>Feels {Math.round(weather.feels_like_c)}°</span>
            <span>💧 {weather.humidity_pct}%</span>
            <span>💨 {Math.round(weather.wind_kmh)} km/h</span>
          </div>
        </div>
      </div>
    {/if}

    <!-- Divider -->
    <div class="border-t border-white/5"></div>

    <!-- Next holiday row -->
    {#if nextHoliday}
      {@const days = daysUntil(nextHoliday.date)}
      <div class="flex items-center justify-between">
        <div class="min-w-0">
          <p class="truncate font-medium text-white">{nextHoliday.name}</p>
          <p class="text-[10px] text-slate-300">{nextHoliday.date}</p>
        </div>
        <span class="shrink-0 rounded-lg border border-teal-300/20 bg-teal-400/10 px-2 py-1 text-[10px] font-medium text-teal-200">
          {days === 0 ? "Today!" : days === 1 ? "Tomorrow" : `${days}d away`}
        </span>
      </div>
    {:else}
      <p class="text-[10px] text-slate-400">No upcoming holidays found.</p>
    {/if}
  {/if}
</div>
