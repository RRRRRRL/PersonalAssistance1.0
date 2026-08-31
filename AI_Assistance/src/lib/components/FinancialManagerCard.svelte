<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { emitRefresh, onRefresh } from "$lib/stores/refreshBus";
  import { onMount } from "svelte";

  interface PurchaseRecord {
    id: number;
    item_name: string;
    category: string;
    amount: number;
    payment_method: string;
    card_name?: string;
    purchased_at: string;
    note?: string;
  }

  interface CreditCardAccount {
    card_name: string;
    statement_balance: number;
    credit_limit: number;
    minimum_due?: number;
    due_date?: string;
  }

  interface BankAccountBalance {
    account_name: string;
    current_balance: number;
    available_balance?: number;
  }

  interface FinancialManagerOverview {
    monthly_purchase_total: number;
    total_credit_card_balance: number;
    total_bank_balance: number;
    credit_utilization_percent: number;
    recent_purchases: PurchaseRecord[];
    credit_cards: CreditCardAccount[];
    bank_accounts: BankAccountBalance[];
  }

  interface RecurringExpenseInsight {
    item_name: string;
    category: string;
    average_amount: number;
    occurrences: number;
    last_purchase_date: string;
    estimated_next_purchase_date: string;
  }

  interface CardDueReminder {
    card_name: string;
    due_date: string;
    days_left: number;
    statement_balance: number;
    minimum_due?: number;
  }

  interface FinancialManagerAlerts {
    recurring_expenses: RecurringExpenseInsight[];
    due_reminders: CardDueReminder[];
    alert_summary: string;
  }

  let overview = $state<FinancialManagerOverview | null>(null);
  let alerts = $state<FinancialManagerAlerts | null>(null);
  let busy = $state(false);

  let purchaseItem = $state("");
  let purchaseCategory = $state("general");
  let purchaseAmount = $state(0);
  let paymentMethod = $state("debit");
  let purchaseCard = $state("");
  let purchaseDate = $state(new Date().toISOString().slice(0, 10));

  let cardName = $state("Main Card");
  let cardBalance = $state(0);
  let cardLimit = $state(10000);
  let cardDue = $state("");

  let bankName = $state("Primary Checking");
  let bankCurrent = $state(0);
  let bankAvailable = $state(0);

  onMount(() => {
    void refreshOverview();
    void refreshAlerts();

    const offFinance = onRefresh("finance", () => {
      void refreshOverview();
    });
    const offAlerts = onRefresh("alerts", () => {
      void refreshAlerts();
    });

    return () => {
      offFinance();
      offAlerts();
    };
  });

  async function refreshOverview() {
    try {
      overview = await invoke<FinancialManagerOverview>("get_financial_manager_overview");
    } catch (error) {
      console.error("Failed to load financial manager overview", error);
    }
  }

  async function refreshAlerts() {
    try {
      alerts = await invoke<FinancialManagerAlerts>("get_financial_manager_alerts");
    } catch (error) {
      console.error("Failed to load financial alerts", error);
    }
  }

  async function addPurchase() {
    if (!purchaseItem.trim() || purchaseAmount <= 0 || busy) {
      return;
    }

    busy = true;
    try {
      await invoke("add_purchase_record", {
        input: {
          item_name: purchaseItem,
          category: purchaseCategory,
          amount: purchaseAmount,
          payment_method: paymentMethod,
          card_name: paymentMethod === "credit" ? purchaseCard || null : null,
          purchased_at: purchaseDate,
          note: null,
        },
      });
      purchaseItem = "";
      purchaseAmount = 0;
      await refreshOverview();
      await refreshAlerts();
      emitRefresh("finance");
      emitRefresh("alerts");
    } catch (error) {
      console.error("Failed to add purchase", error);
    } finally {
      busy = false;
    }
  }

  async function saveCard() {
    try {
      await invoke("upsert_credit_card_account", {
        input: {
          card_name: cardName,
          statement_balance: cardBalance,
          credit_limit: cardLimit,
          minimum_due: null,
          due_date: cardDue || null,
        },
      });
      await refreshOverview();
      await refreshAlerts();
      emitRefresh("finance");
      emitRefresh("alerts");
    } catch (error) {
      console.error("Failed to save credit card", error);
    }
  }

  async function saveBank() {
    try {
      await invoke("upsert_bank_account_balance", {
        input: {
          account_name: bankName,
          current_balance: bankCurrent,
          available_balance: bankAvailable,
        },
      });
      await refreshOverview();
      await refreshAlerts();
      emitRefresh("finance");
      emitRefresh("alerts");
    } catch (error) {
      console.error("Failed to save bank account", error);
    }
  }

  async function syncAdvisor() {
    try {
      await invoke("send_financial_overview_to_advisor");
      emitRefresh("orchestration");
    } catch (error) {
      console.error("Failed to sync overview to advisor", error);
    }
  }

  async function syncAlertsAdvisor() {
    try {
      await invoke("send_financial_alerts_to_advisor");
      emitRefresh("orchestration");
      emitRefresh("alerts");
    } catch (error) {
      console.error("Failed to sync financial alerts to advisor", error);
    }
  }
</script>

<div class="space-y-3 text-xs text-slate-100">
  {#if overview}
    <div class="grid grid-cols-2 gap-2">
      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <p class="text-[10px] uppercase text-amber-200">Monthly Spend</p>
        <p class="text-sm font-semibold">${overview.monthly_purchase_total.toFixed(2)}</p>
      </div>
      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <p class="text-[10px] uppercase text-cyan-200">Bank Balance</p>
        <p class="text-sm font-semibold">${overview.total_bank_balance.toFixed(2)}</p>
      </div>
      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <p class="text-[10px] uppercase text-rose-200">Card Balance</p>
        <p class="text-sm font-semibold">${overview.total_credit_card_balance.toFixed(2)}</p>
      </div>
      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <p class="text-[10px] uppercase text-violet-200">Utilization</p>
        <p class="text-sm font-semibold">{overview.credit_utilization_percent.toFixed(2)}%</p>
      </div>
    </div>
  {/if}

  <div class="rounded-lg border border-white/10 bg-black/20 p-2">
    <p class="mb-2 text-[10px] uppercase text-amber-200">Add Purchase</p>
    <div class="grid grid-cols-2 gap-2">
      <input bind:value={purchaseItem} class="rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Item" />
      <input bind:value={purchaseCategory} class="rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Category" />
      <input type="number" bind:value={purchaseAmount} class="rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Amount" />
      <select bind:value={paymentMethod} class="rounded border border-white/10 bg-white/5 px-2 py-1">
        <option value="debit">debit</option>
        <option value="credit">credit</option>
      </select>
      <input bind:value={purchaseCard} class="rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Card (if credit)" />
      <input type="date" bind:value={purchaseDate} class="rounded border border-white/10 bg-white/5 px-2 py-1" />
    </div>
    <button type="button" class="mt-2 rounded bg-amber-400/80 px-3 py-1.5 font-semibold text-slate-950 hover:bg-amber-300" on:click={addPurchase}>Save Purchase</button>
  </div>

  <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <p class="mb-2 text-[10px] uppercase text-cyan-200">Credit Card</p>
      <input bind:value={cardName} class="mb-1 w-full rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Card Name" />
      <input type="number" bind:value={cardBalance} class="mb-1 w-full rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Statement Balance" />
      <input type="number" bind:value={cardLimit} class="mb-1 w-full rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Credit Limit" />
      <input type="date" bind:value={cardDue} class="w-full rounded border border-white/10 bg-white/5 px-2 py-1" />
      <button type="button" class="mt-2 rounded border border-cyan-300/30 bg-cyan-500/15 px-2 py-1 text-[10px] text-cyan-100 hover:bg-cyan-500/25" on:click={saveCard}>Save Card</button>
    </div>

    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <p class="mb-2 text-[10px] uppercase text-emerald-200">Bank Account</p>
      <input bind:value={bankName} class="mb-1 w-full rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Account Name" />
      <input type="number" bind:value={bankCurrent} class="mb-1 w-full rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Current Balance" />
      <input type="number" bind:value={bankAvailable} class="w-full rounded border border-white/10 bg-white/5 px-2 py-1" placeholder="Available Balance" />
      <button type="button" class="mt-2 rounded border border-emerald-300/30 bg-emerald-500/15 px-2 py-1 text-[10px] text-emerald-100 hover:bg-emerald-500/25" on:click={saveBank}>Save Bank</button>
    </div>
  </div>

  {#if overview && overview.recent_purchases.length > 0}
    <div class="rounded-lg border border-white/10 bg-white/5 p-2">
      <p class="mb-1 text-[10px] uppercase text-slate-300">Recent Purchases</p>
      <ul class="space-y-1">
        {#each overview.recent_purchases as purchase}
          <li class="flex items-center justify-between gap-2 text-[10px]">
            <span class="truncate">{purchase.purchased_at} {purchase.item_name}</span>
            <span>${purchase.amount.toFixed(2)}</span>
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if alerts}
    <div class="rounded-lg border border-white/10 bg-black/20 p-2">
      <p class="mb-1 text-[10px] uppercase text-violet-200">Alerts Summary</p>
      <p class="text-[11px] text-slate-200">{alerts.alert_summary}</p>
    </div>

    <div class="grid grid-cols-1 gap-2 sm:grid-cols-2">
      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <p class="mb-1 text-[10px] uppercase text-amber-200">Recurring Expenses</p>
        <ul class="space-y-1">
          {#each alerts.recurring_expenses.slice(0, 4) as recurring}
            <li class="text-[10px] text-slate-200">
              {recurring.item_name} (${recurring.average_amount.toFixed(2)}) next ~ {recurring.estimated_next_purchase_date}
            </li>
          {/each}
          {#if alerts.recurring_expenses.length === 0}
            <li class="text-[10px] text-slate-400">No recurring pattern detected yet.</li>
          {/if}
        </ul>
      </div>

      <div class="rounded-lg border border-white/10 bg-white/5 p-2">
        <p class="mb-1 text-[10px] uppercase text-rose-200">Upcoming Card Dues</p>
        <ul class="space-y-1">
          {#each alerts.due_reminders.slice(0, 4) as due}
            <li class="text-[10px] text-slate-200">
              {due.card_name}: {due.due_date} ({due.days_left}d) ${due.statement_balance.toFixed(2)}
            </li>
          {/each}
          {#if alerts.due_reminders.length === 0}
            <li class="text-[10px] text-slate-400">No card due in next 14 days.</li>
          {/if}
        </ul>
      </div>
    </div>
  {/if}

  <button type="button" class="rounded-lg bg-violet-400/80 px-3 py-1.5 text-[11px] font-semibold text-slate-950 hover:bg-violet-300" on:click={syncAdvisor}>
    Send Summary To Advisor
  </button>

  <button type="button" class="rounded-lg bg-rose-400/80 px-3 py-1.5 text-[11px] font-semibold text-slate-950 hover:bg-rose-300" on:click={syncAlertsAdvisor}>
    Send Alerts To Advisor
  </button>
</div>
