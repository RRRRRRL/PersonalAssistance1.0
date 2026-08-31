<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface YouTubeMusicSettings {
    provider_base_url: string;
    provider_api_key_env?: string | null;
    preferred_playlist_id?: string | null;
    updated_at: number;
  }

  interface YouTubeMusicStatus {
    settings: YouTubeMusicSettings;
    provider_configured: boolean;
    api_key_present: boolean;
    provider_reachable: boolean;
    authenticated: boolean;
    error?: string | null;
  }

  interface YouTubeMusicPlaylist {
    title: string;
    playlist_id: string;
    track_count?: number | null;
    privacy?: string | null;
    url: string;
  }

  interface YouTubeMusicSearchResult {
    title: string;
    artist?: string | null;
    album?: string | null;
    item_type: string;
    video_id?: string | null;
    browse_id?: string | null;
    url?: string | null;
  }

  let status = $state<YouTubeMusicStatus | null>(null);
  let playlists = $state<YouTubeMusicPlaylist[]>([]);
  let results = $state<YouTubeMusicSearchResult[]>([]);
  let providerBaseUrl = $state("https://api.poe.com/v1/music");
  let providerApiKeyEnv = $state("POE_API_KEY");
  let preferredPlaylistId = $state("");
  let searchQuery = $state("");
  let busy = $state(false);
  let errorMessage = $state("");

  onMount(() => {
    void refreshStatus();

    const off = onRefresh("music", () => {
      void refreshStatus();
    });

    return () => {
      off();
    };
  });

  async function refreshStatus() {
    try {
      status = await invoke<YouTubeMusicStatus>("get_youtube_music_status");
      providerBaseUrl = status.settings.provider_base_url || "https://api.poe.com/v1/music";
      providerApiKeyEnv = status.settings.provider_api_key_env ?? "POE_API_KEY";
      preferredPlaylistId = status.settings.preferred_playlist_id ?? "";
      errorMessage = status.error ?? "";
    } catch (error) {
      console.error("Failed to load YouTube Music status", error);
      errorMessage = "Failed to load provider status.";
    }
  }

  async function saveSettings() {
    if (busy) {
      return;
    }

    busy = true;
    try {
      await invoke("save_youtube_music_settings", {
        input: {
          provider_base_url: providerBaseUrl,
          provider_api_key_env: providerApiKeyEnv || null,
          preferred_playlist_id: preferredPlaylistId || null,
        },
      });
      await refreshStatus();
      emitRefresh("music");
    } catch (error) {
      console.error("Failed to save YouTube Music settings", error);
      errorMessage = "Failed to save settings.";
    } finally {
      busy = false;
    }
  }

  async function loadPlaylists() {
    if (busy) {
      return;
    }

    busy = true;
    try {
      playlists = await invoke<YouTubeMusicPlaylist[]>("get_youtube_music_playlists");
      errorMessage = "";
    } catch (error) {
      console.error("Failed to load YouTube Music playlists", error);
      errorMessage = "Unable to load playlists from provider. Using fallback shortcuts.";
    } finally {
      busy = false;
    }
  }

  async function searchMusic() {
    if (busy || !searchQuery.trim()) {
      return;
    }

    busy = true;
    try {
      results = await invoke<YouTubeMusicSearchResult[]>("search_youtube_music", {
        query: searchQuery,
      });
      errorMessage = "";
    } catch (error) {
      console.error("Failed to search YouTube Music", error);
      errorMessage = "Unable to search via provider. Using shortcut fallback.";
    } finally {
      busy = false;
    }
  }

  function statusTone(ready: boolean) {
    return ready ? "text-emerald-300" : "text-amber-300";
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <div class="grid grid-cols-2 gap-2">
    <div class="rounded-lg border border-white/10 bg-white/5 p-2">
      <p class="text-[10px] uppercase text-red-200">Provider</p>
      <p class={`text-sm font-semibold ${statusTone(Boolean(status?.provider_configured))}`}>
        {status?.provider_configured ? "configured" : "missing"}
      </p>
    </div>
    <div class="rounded-lg border border-white/10 bg-white/5 p-2">
      <p class="text-[10px] uppercase text-amber-200">API Key</p>
      <p class={`text-sm font-semibold ${statusTone(Boolean(status?.api_key_present))}`}>
        {status?.api_key_present ? "present" : "missing"}
      </p>
    </div>
    <div class="rounded-lg border border-white/10 bg-white/5 p-2">
      <p class="text-[10px] uppercase text-cyan-200">Reachability</p>
      <p class={`text-sm font-semibold ${statusTone(Boolean(status?.provider_reachable))}`}>
        {status?.provider_reachable ? "online" : "offline"}
      </p>
    </div>
    <div class="rounded-lg border border-white/10 bg-white/5 p-2">
      <p class="text-[10px] uppercase text-rose-200">Session</p>
      <p class={`text-sm font-semibold ${statusTone(Boolean(status?.authenticated))}`}>
        {status?.authenticated ? "connected" : "not connected"}
      </p>
    </div>
  </div>

  <div class="rounded-lg border border-white/10 bg-black/20 p-3">
    <p class="mb-2 text-[10px] uppercase text-slate-300">Provider Settings</p>
    <div class="grid gap-2 md:grid-cols-2">
      <input bind:value={providerBaseUrl} class="rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="POE provider endpoint URL" />
      <input bind:value={providerApiKeyEnv} class="rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="API key env name (example: POE_API_KEY)" />
      <input bind:value={preferredPlaylistId} class="rounded border border-white/10 bg-white/5 px-2 py-1 md:col-span-2" placeholder="Optional preferred playlist id" />
    </div>
    <div class="mt-2 flex flex-wrap gap-2">
      <button type="button" class="rounded bg-red-400/80 px-3 py-1.5 text-[11px] font-semibold text-slate-950 hover:bg-red-300 disabled:opacity-60" disabled={busy} on:click={saveSettings}>
        Save Settings
      </button>
      <button type="button" class="rounded border border-white/15 bg-white/10 px-3 py-1.5 text-[11px] font-semibold text-slate-100 hover:bg-white/15 disabled:opacity-60" disabled={busy} on:click={refreshStatus}>
        Refresh Status
      </button>
      <a class="rounded border border-red-300/25 bg-red-500/10 px-3 py-1.5 text-[11px] font-semibold text-red-100 hover:bg-red-500/20" href="https://music.youtube.com" target="_blank" rel="noreferrer">
        Open YouTube Music
      </a>
    </div>
    <p class="mt-2 text-[10px] text-slate-400">
      This module uses Rust backend provider calls. Keep the API key in an environment variable and store only the env name here.
    </p>
    {#if errorMessage}
      <p class="mt-2 text-[10px] text-amber-200">{errorMessage}</p>
    {/if}
  </div>

  <div class="grid gap-3 md:grid-cols-[1.1fr_0.9fr]">
    <div class="rounded-lg border border-white/10 bg-black/20 p-3">
      <div class="mb-2 flex items-center justify-between gap-2">
        <p class="text-[10px] uppercase text-amber-200">Library Playlists</p>
        <button type="button" class="rounded border border-amber-300/30 bg-amber-500/15 px-2 py-1 text-[10px] text-amber-100 hover:bg-amber-500/25 disabled:opacity-60" disabled={busy} on:click={loadPlaylists}>
          Load Playlists
        </button>
      </div>

      <div class="space-y-2">
        {#if playlists.length > 0}
          {#each playlists as playlist}
            <div class="rounded-lg border border-white/10 bg-white/5 p-2">
              <div class="flex items-start justify-between gap-2">
                <div>
                  <p class="font-semibold text-slate-100">{playlist.title}</p>
                  <p class="text-[10px] text-slate-400">
                    {playlist.track_count ?? 0} tracks{playlist.privacy ? ` • ${playlist.privacy}` : ""}
                  </p>
                </div>
                <a class="text-[10px] text-red-200 hover:text-red-100" href={playlist.url} target="_blank" rel="noreferrer">Open</a>
              </div>
            </div>
          {/each}
        {:else}
          <p class="text-[10px] text-slate-400">No playlists loaded yet.</p>
        {/if}
      </div>
    </div>

    <div class="rounded-lg border border-white/10 bg-black/20 p-3">
      <p class="mb-2 text-[10px] uppercase text-cyan-200">Search Songs</p>
      <div class="flex gap-2">
        <input bind:value={searchQuery} class="flex-1 rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Search YouTube Music" />
        <button type="button" class="rounded border border-cyan-300/30 bg-cyan-500/15 px-3 py-1 text-[10px] text-cyan-100 hover:bg-cyan-500/25 disabled:opacity-60" disabled={busy} on:click={searchMusic}>
          Search
        </button>
      </div>

      <div class="mt-3 space-y-2">
        {#if results.length > 0}
          {#each results as item}
            <div class="rounded-lg border border-white/10 bg-white/5 p-2">
              <div class="flex items-start justify-between gap-2">
                <div>
                  <p class="font-semibold text-slate-100">{item.title}</p>
                  <p class="text-[10px] text-slate-400">
                    {item.artist ?? "Unknown artist"}{item.album ? ` • ${item.album}` : ""}
                  </p>
                </div>
                <span class="text-[10px] uppercase text-slate-400">{item.item_type}</span>
              </div>
              {#if item.url}
                <a class="mt-1 inline-block text-[10px] text-cyan-200 hover:text-cyan-100" href={item.url} target="_blank" rel="noreferrer">
                  Open Track
                </a>
              {/if}
            </div>
          {/each}
        {:else}
          <p class="text-[10px] text-slate-400">Search results will appear here.</p>
        {/if}
      </div>
    </div>
  </div>
</div>