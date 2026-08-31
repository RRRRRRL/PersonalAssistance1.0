<script lang="ts">
  import CodingWorkspaceCard from "$lib/components/CodingWorkspaceCard.svelte";
    import AlertCenterCard from "$lib/components/AlertCenterCard.svelte";
  import CalendarCard from "$lib/components/CalendarCard.svelte";
  import DailyBriefCard from "$lib/components/DailyBriefCard.svelte";
  import FinancialSnapshotCard from "$lib/components/FinancialSnapshotCard.svelte";
  import FinancialManagerCard from "$lib/components/FinancialManagerCard.svelte";
  import PortfolioCard from "$lib/components/PortfolioCard.svelte";
  import FolderCard from "$lib/components/FolderCard.svelte";
  import OrchestrationCard from "$lib/components/OrchestrationCard.svelte";
  import QuickUtilitiesCard from "$lib/components/QuickUtilitiesCard.svelte";
  import RelatedContextPanel from "$lib/components/RelatedContextPanel.svelte";
  import RuleBuilderCard from "$lib/components/RuleBuilderCard.svelte";
  import ScratchNoteCard from "$lib/components/ScratchNoteCard.svelte";
  import SecurityToolkitCard from "$lib/components/SecurityToolkitCard.svelte";
  import StockResearchDigestCard from "$lib/components/StockResearchDigestCard.svelte";
  import YouTubeMusicCard from "$lib/components/YouTubeMusicCard.svelte";
  import ActionApprovalQueueCard from "$lib/components/ActionApprovalQueueCard.svelte";
  import WeeklyPlanningAssistantCard from "$lib/components/WeeklyPlanningAssistantCard.svelte";
  import SelfReviewCard from "$lib/components/SelfReviewCard.svelte";
  import SystemMonitorCard from "$lib/components/SystemMonitorCard.svelte";
  import ClipboardHistoryCard from "$lib/components/ClipboardHistoryCard.svelte";

  type Props = {
    aiResponseText?: string;
  };

  let { aiResponseText = "" }: Props = $props();

  const panelClass =
    "rounded-[1.75rem] border border-white/10 bg-neutral-900/75 p-4 shadow-[0_24px_80px_rgba(0,0,0,0.28)] backdrop-blur-xl sm:p-5";
  const titleClass =
    "mb-3 font-display text-[11px] uppercase tracking-[0.18em]";

</script>

<section class="space-y-4 lg:space-y-5">
  <div class="rounded-[2rem] border border-white/10 bg-[radial-gradient(circle_at_top_left,rgba(56,189,248,0.18),transparent_34%),radial-gradient(circle_at_top_right,rgba(251,191,36,0.16),transparent_28%),rgba(10,10,10,0.72)] p-5 shadow-[0_30px_120px_rgba(0,0,0,0.35)] backdrop-blur-2xl sm:p-6">
    <div class="flex flex-col gap-4 xl:flex-row xl:items-end xl:justify-between">
      <div class="max-w-3xl space-y-2">
        <p class="font-display text-[11px] uppercase tracking-[0.28em] text-cyan-200/90">Intelligence Dashboard</p>
        <h1 class="text-2xl font-semibold tracking-tight text-white sm:text-3xl">Responsive command surface for planning, finance, orchestration, and media context.</h1>
        <p class="max-w-2xl text-sm text-slate-300 sm:text-[15px]">
          The layout adapts from a stacked mobile feed into a multi-lane desktop board so high-signal modules stay visible without compressing the working cards.
        </p>
      </div>

      <div class="grid grid-cols-2 gap-2 sm:grid-cols-4 xl:min-w-[420px]">
        <div class="rounded-2xl border border-cyan-300/15 bg-cyan-400/10 px-3 py-3">
          <p class="text-[10px] uppercase tracking-[0.15em] text-cyan-200">AI</p>
          <p class="mt-1 text-sm font-semibold text-white">Live Workspace</p>
        </div>
        <div class="rounded-2xl border border-indigo-300/15 bg-indigo-400/10 px-3 py-3">
          <p class="text-[10px] uppercase tracking-[0.15em] text-indigo-200">Flow</p>
          <p class="mt-1 text-sm font-semibold text-white">Orchestration</p>
        </div>
        <div class="rounded-2xl border border-amber-300/15 bg-amber-400/10 px-3 py-3">
          <p class="text-[10px] uppercase tracking-[0.15em] text-amber-200">Money</p>
          <p class="mt-1 text-sm font-semibold text-white">Finance Ops</p>
        </div>
        <div class="rounded-2xl border border-rose-300/15 bg-rose-400/10 px-3 py-3">
          <p class="text-[10px] uppercase tracking-[0.15em] text-rose-200">Signal</p>
          <p class="mt-1 text-sm font-semibold text-white">Alert Center</p>
        </div>
      </div>
    </div>
  </div>

  <div class="grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-12 xl:auto-rows-[minmax(220px,auto)] xl:gap-5">
  <article class={`${panelClass} xl:col-span-5 xl:row-span-2`}>
    <h2 class={`${titleClass} text-emerald-200`}>Coding and AI Workspace</h2>
    <CodingWorkspaceCard {aiResponseText} />
  </article>

  <article class={`${panelClass} xl:col-span-4 xl:row-span-2`}>
    <h2 class={`${titleClass} text-indigo-200`}>Inbox and Orchestration</h2>
    <OrchestrationCard />
  </article>

  <article class={`${panelClass} xl:col-span-3`}>
    <h2 class={`${titleClass} text-rose-200`}>Alert Center</h2>
    <AlertCenterCard />
  </article>

  <article class={`${panelClass} xl:col-span-3`}>
    <h2 class={`${titleClass} text-cyan-200`}>Calendar and Focus</h2>
    <div class="text-slate-100">
      <CalendarCard />
    </div>
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-amber-200`}>Financial Snapshot</h2>
    <FinancialSnapshotCard />
  </article>

  <article class={`${panelClass} xl:col-span-4 xl:row-span-2`}>
    <h2 class={`${titleClass} text-orange-200`}>Portfolio & Watchlist</h2>
    <PortfolioCard />
  </article>

  <article class={`${panelClass} xl:col-span-5 xl:row-span-2`}>
    <h2 class={`${titleClass} text-yellow-200`}>Financial Manager</h2>
    <FinancialManagerCard />
  </article>

  <article class={`${panelClass} xl:col-span-4 xl:row-span-2`}>
    <h2 class={`${titleClass} text-red-200`}>YouTube Music</h2>
    <YouTubeMusicCard />
  </article>

  <article class={`${panelClass} xl:col-span-3`}>
    <h2 class={`${titleClass} text-cyan-200`}>Stock Research Digest</h2>
    <StockResearchDigestCard />
  </article>

  <article class={`${panelClass} xl:col-span-7 xl:row-span-2`}>
    <h2 class={`${titleClass} text-violet-200`}>Weekly Planning Assistant</h2>
    <WeeklyPlanningAssistantCard />
  </article>

  <article class={`${panelClass} xl:col-span-5 xl:row-span-2`}>
    <h2 class={`${titleClass} text-violet-200`}>Rule Builder</h2>
    <RuleBuilderCard />
  </article>

  <article class={`${panelClass} xl:col-span-3`}>
    <h2 class={`${titleClass} text-violet-200`}>Self Review</h2>
    <SelfReviewCard />
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-emerald-200`}>Action Approval Queue</h2>
    <ActionApprovalQueueCard />
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-lime-200`}>Related Context</h2>
    <RelatedContextPanel />
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-amber-200`}>Folders and Files</h2>
    <div class="text-slate-100">
      <FolderCard />
    </div>
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-rose-200`}>Quick Scratch Note</h2>
    <ScratchNoteCard />
  </article>

  <article class={`${panelClass} xl:col-span-3`}>
    <h2 class={`${titleClass} text-teal-200`}>Daily Brief</h2>
    <DailyBriefCard />
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-sky-200`}>Security Toolkit</h2>
    <SecurityToolkitCard />
  </article>

  <article class={`${panelClass} xl:col-span-5`}>
    <h2 class={`${titleClass} text-amber-200`}>Quick Utilities</h2>
    <QuickUtilitiesCard />
  </article>

  <article class={`${panelClass} xl:col-span-3`}>
    <h2 class={`${titleClass} text-teal-200`}>System Monitor</h2>
    <SystemMonitorCard />
  </article>

  <article class={`${panelClass} xl:col-span-4`}>
    <h2 class={`${titleClass} text-indigo-200`}>Clipboard History</h2>
    <ClipboardHistoryCard />
  </article>
  </div>
</section>
