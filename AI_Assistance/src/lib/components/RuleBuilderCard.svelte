<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface OrchestrationRule {
    id: number;
    name: string;
    source_pattern?: string;
    keyword_pattern?: string;
    action_type: string;
    title_template: string;
    note_template?: string;
    is_active: boolean;
  }

  let rules = $state<OrchestrationRule[]>([]);
  let name = $state("");
  let sourcePattern = $state("");
  let keywordPattern = $state("");
  let actionType = $state("calendar_event");
  let titleTemplate = $state("Follow up: {content}");
  let noteTemplate = $state("Auto generated for {due_date}");

  onMount(() => {
    void refreshRules();
  });

  async function refreshRules() {
    try {
      rules = await invoke<OrchestrationRule[]>("get_orchestration_rules");
    } catch (error) {
      console.error("Failed to load rules", error);
    }
  }

  async function addRule() {
    if (!name.trim() || !actionType.trim() || !titleTemplate.trim()) {
      return;
    }

    try {
      await invoke("add_orchestration_rule", {
        rule: {
          name,
          source_pattern: sourcePattern.trim() || null,
          keyword_pattern: keywordPattern.trim() || null,
          action_type: actionType,
          title_template: titleTemplate,
          note_template: noteTemplate.trim() || null,
        },
      });

      name = "";
      sourcePattern = "";
      keywordPattern = "";
      actionType = "calendar_event";
      titleTemplate = "Follow up: {content}";
      noteTemplate = "Auto generated for {due_date}";
      await refreshRules();
    } catch (error) {
      console.error("Failed to create rule", error);
    }
  }

  async function toggleRule(rule: OrchestrationRule) {
    try {
      await invoke("set_orchestration_rule_active", {
        ruleId: rule.id,
        isActive: !rule.is_active,
      });
      await refreshRules();
    } catch (error) {
      console.error("Failed to toggle rule", error);
    }
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  <div class="grid grid-cols-2 gap-2">
    <input bind:value={name} class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5" placeholder="Rule name" />
    <input bind:value={actionType} class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5" placeholder="Action type" />
    <input bind:value={sourcePattern} class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5" placeholder="Source pattern" />
    <input bind:value={keywordPattern} class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5" placeholder="Keyword pattern" />
  </div>
  <input bind:value={titleTemplate} class="w-full rounded-lg border border-white/10 bg-white/5 px-2 py-1.5" placeholder="Title template with {content} {due_date}" />
  <input bind:value={noteTemplate} class="w-full rounded-lg border border-white/10 bg-white/5 px-2 py-1.5" placeholder="Note template (optional)" />
  <button type="button" class="rounded-lg bg-indigo-400/80 px-3 py-1.5 text-xs font-semibold text-slate-950 hover:bg-indigo-300" on:click={addRule}>Add Rule</button>

  <div class="space-y-2">
    {#each rules as rule}
      <div class="rounded-lg border border-white/10 bg-white/5 px-2 py-1.5">
        <div class="flex items-center justify-between gap-2">
          <div class="truncate font-semibold">{rule.name}</div>
          <button
            type="button"
            class="rounded border border-white/15 px-2 py-0.5 text-[10px] uppercase"
            on:click={() => toggleRule(rule)}
          >
            {rule.is_active ? "Active" : "Paused"}
          </button>
        </div>
        <div class="mt-1 text-[10px] text-slate-300">{rule.action_type} | src={rule.source_pattern || "*"} | key={rule.keyword_pattern || "*"}</div>
      </div>
    {/each}
    {#if rules.length === 0}
      <div class="text-[10px] text-slate-400">No custom rules yet.</div>
    {/if}
  </div>
</div>
