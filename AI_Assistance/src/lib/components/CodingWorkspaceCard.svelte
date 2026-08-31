<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  type Props = {
    aiResponseText?: string;
  };

  let { aiResponseText = "" }: Props = $props();

  let renderedHtml = $state("");
  let useMarkdown = $state(true);

  const suggestions = [
    "Explain this function",
    "Refactor this component",
    "Generate test cases",
    "Find edge cases",
  ];

  // Debounced markdown rendering
  let renderTimeout: ReturnType<typeof setTimeout> | null = null;
  $effect(() => {
    if (!aiResponseText.trim()) {
      renderedHtml = "";
      return;
    }
    if (!useMarkdown) return;

    if (renderTimeout) clearTimeout(renderTimeout);
    renderTimeout = setTimeout(async () => {
      try {
        renderedHtml = await invoke<string>("render_markdown", { input: aiResponseText });
      } catch {
        renderedHtml = "";
      }
    }, 150);
  });
</script>

<div class="space-y-3 text-slate-100">
  <div class="rounded-xl border border-white/10 bg-black/25 p-3">
    <div class="mb-2 flex items-center justify-between">
      <p class="font-display text-xs uppercase tracking-[0.1em] text-emerald-200">Live AI Stream</p>
      <label class="flex items-center gap-1 text-[9px] text-slate-400 cursor-pointer">
        <input type="checkbox" bind:checked={useMarkdown} class="h-2.5 w-2.5 rounded border-white/20 bg-white/5 accent-emerald-400" />
        Markdown
      </label>
    </div>
    <div class="min-h-28 text-sm leading-6 text-slate-100">
      {#if aiResponseText.trim()}
        {#if useMarkdown && renderedHtml}
          {@html `<div class="prose-ai">${renderedHtml}</div>`}
        {:else}
          <pre class="whitespace-pre-wrap font-sans">{aiResponseText}</pre>
        {/if}
      {:else}
        <span class="text-slate-300">Run a prompt from the search bar to stream coding output here.</span>
      {/if}
    </div>
  </div>

  <div class="rounded-xl border border-white/10 bg-white/5 p-3">
    <p class="mb-2 font-display text-xs uppercase tracking-[0.1em] text-emerald-200">Coding Shortcuts</p>
    <div class="flex flex-wrap gap-2">
      {#each suggestions as item}
        <span class="rounded-lg border border-white/10 bg-neutral-800/70 px-2 py-1 text-xs text-slate-200">{item}</span>
      {/each}
    </div>
  </div>
</div>

<style>
  :global(.prose-ai h1) { font-size: 1.25rem; font-weight: 700; margin: 0.75rem 0 0.5rem; color: #e2e8f0; }
  :global(.prose-ai h2) { font-size: 1.1rem; font-weight: 600; margin: 0.6rem 0 0.4rem; color: #e2e8f0; }
  :global(.prose-ai h3) { font-size: 1rem; font-weight: 600; margin: 0.5rem 0 0.3rem; color: #e2e8f0; }
  :global(.prose-ai p) { margin: 0.4rem 0; line-height: 1.6; }
  :global(.prose-ai code) {
    background: rgba(255,255,255,0.08);
    border-radius: 0.25rem;
    padding: 0.1rem 0.35rem;
    font-size: 0.85em;
    color: #67e8f9;
  }
  :global(.prose-ai pre) {
    background: rgba(0,0,0,0.4);
    border: 1px solid rgba(255,255,255,0.1);
    border-radius: 0.5rem;
    padding: 0.75rem;
    overflow-x: auto;
    margin: 0.5rem 0;
  }
  :global(.prose-ai pre code) {
    background: none;
    padding: 0;
    color: #a5f3fc;
  }
  :global(.prose-ai ul) { list-style: disc; padding-left: 1.5rem; margin: 0.3rem 0; }
  :global(.prose-ai ol) { list-style: decimal; padding-left: 1.5rem; margin: 0.3rem 0; }
  :global(.prose-ai li) { margin: 0.15rem 0; }
  :global(.prose-ai blockquote) {
    border-left: 3px solid rgba(103,232,249,0.4);
    padding-left: 0.75rem;
    margin: 0.5rem 0;
    color: #94a3b8;
  }
  :global(.prose-ai table) { width: 100%; border-collapse: collapse; margin: 0.5rem 0; font-size: 0.85em; }
  :global(.prose-ai th), :global(.prose-ai td) {
    border: 1px solid rgba(255,255,255,0.1);
    padding: 0.3rem 0.5rem;
    text-align: left;
  }
  :global(.prose-ai th) { background: rgba(255,255,255,0.05); font-weight: 600; }
  :global(.prose-ai a) { color: #67e8f9; text-decoration: underline; }
  :global(.prose-ai strong) { color: #f1f5f9; }
  :global(.prose-ai hr) { border-color: rgba(255,255,255,0.1); margin: 0.75rem 0; }
</style>
