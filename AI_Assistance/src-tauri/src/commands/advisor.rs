use crate::db::DbState;
use crate::commands::market_data;
use chrono::{Duration, Local, NaiveDate};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tauri::State;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize)]
pub struct MarketMover {
    pub ticker: String,
    pub change_percent: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinancialSnapshot {
    pub estimated_portfolio_value: f64,
    pub day_change_percent: f64,
    pub watchlist_overview: String,
    pub risk_note: String,
    pub top_movers: Vec<MarketMover>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StockDigestItem {
    pub ticker: String,
    pub headline: String,
    pub sentiment: String,
    pub action_hint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyCalendarItem {
    pub title: String,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyActionItem {
    pub title: String,
    pub due_date: Option<String>,
    pub action_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeeklyPlanningAssistant {
    pub week_label: String,
    pub priorities: Vec<String>,
    pub calendar_items: Vec<WeeklyCalendarItem>,
    pub pending_actions: Vec<WeeklyActionItem>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PurchaseInput {
    pub item_name: String,
    pub category: String,
    pub amount: f64,
    pub payment_method: String,
    pub card_name: Option<String>,
    pub purchased_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreditCardInput {
    pub card_name: String,
    pub statement_balance: f64,
    pub credit_limit: f64,
    pub minimum_due: Option<f64>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BankAccountInput {
    pub account_name: String,
    pub current_balance: f64,
    pub available_balance: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PurchaseRecord {
    pub id: i64,
    pub item_name: String,
    pub category: String,
    pub amount: f64,
    pub payment_method: String,
    pub card_name: Option<String>,
    pub purchased_at: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CreditCardAccount {
    pub card_name: String,
    pub statement_balance: f64,
    pub credit_limit: f64,
    pub minimum_due: Option<f64>,
    pub due_date: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BankAccountBalance {
    pub account_name: String,
    pub current_balance: f64,
    pub available_balance: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinancialManagerOverview {
    pub monthly_purchase_total: f64,
    pub total_credit_card_balance: f64,
    pub total_bank_balance: f64,
    pub credit_utilization_percent: f64,
    pub recent_purchases: Vec<PurchaseRecord>,
    pub credit_cards: Vec<CreditCardAccount>,
    pub bank_accounts: Vec<BankAccountBalance>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RecurringExpenseInsight {
    pub item_name: String,
    pub category: String,
    pub average_amount: f64,
    pub occurrences: i64,
    pub last_purchase_date: String,
    pub estimated_next_purchase_date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CardDueReminder {
    pub card_name: String,
    pub due_date: String,
    pub days_left: i64,
    pub statement_balance: f64,
    pub minimum_due: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinancialManagerAlerts {
    pub recurring_expenses: Vec<RecurringExpenseInsight>,
    pub due_reminders: Vec<CardDueReminder>,
    pub alert_summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemAlert {
    pub id: i64,
    pub title: String,
    pub detail: String,
    pub severity: String,
    pub source: String,
    pub status: String,
    pub occurrences: i64,
    pub last_triggered_at: i64,
}

fn now_ts() -> Result<i64, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs() as i64)
}

#[tauri::command]
pub fn add_purchase_record(input: PurchaseInput, state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT INTO purchase_history
         (item_name, category, amount, payment_method, card_name, purchased_at, note, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.item_name,
            input.category,
            input.amount,
            input.payment_method,
            input.card_name,
            input.purchased_at,
            input.note,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub fn upsert_credit_card_account(input: CreditCardInput, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT INTO credit_card_accounts
         (card_name, statement_balance, credit_limit, minimum_due, due_date, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(card_name) DO UPDATE SET
           statement_balance = excluded.statement_balance,
           credit_limit = excluded.credit_limit,
           minimum_due = excluded.minimum_due,
           due_date = excluded.due_date,
           updated_at = excluded.updated_at",
        params![
            input.card_name,
            input.statement_balance,
            input.credit_limit,
            input.minimum_due,
            input.due_date,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn upsert_bank_account_balance(input: BankAccountInput, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT INTO bank_accounts
         (account_name, current_balance, available_balance, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(account_name) DO UPDATE SET
           current_balance = excluded.current_balance,
           available_balance = excluded.available_balance,
           updated_at = excluded.updated_at",
        params![
            input.account_name,
            input.current_balance,
            input.available_balance,
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_financial_manager_overview(state: State<'_, DbState>) -> Result<FinancialManagerOverview, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let current_month = Local::now().format("%Y-%m").to_string();

    let monthly_purchase_total: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(amount), 0)
             FROM purchase_history
             WHERE substr(purchased_at, 1, 7) = ?1",
            params![current_month],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let mut purchase_stmt = conn
        .prepare(
            "SELECT id, item_name, category, amount, payment_method, card_name, purchased_at, note
             FROM purchase_history
             ORDER BY purchased_at DESC, id DESC
             LIMIT 8",
        )
        .map_err(|e| e.to_string())?;

    let recent_purchases = purchase_stmt
        .query_map([], |row| {
            Ok(PurchaseRecord {
                id: row.get(0)?,
                item_name: row.get(1)?,
                category: row.get(2)?,
                amount: row.get(3)?,
                payment_method: row.get(4)?,
                card_name: row.get(5)?,
                purchased_at: row.get(6)?,
                note: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut card_stmt = conn
        .prepare(
            "SELECT card_name, statement_balance, credit_limit, minimum_due, due_date
             FROM credit_card_accounts
             ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let credit_cards = card_stmt
        .query_map([], |row| {
            Ok(CreditCardAccount {
                card_name: row.get(0)?,
                statement_balance: row.get(1)?,
                credit_limit: row.get(2)?,
                minimum_due: row.get(3)?,
                due_date: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut bank_stmt = conn
        .prepare(
            "SELECT account_name, current_balance, available_balance
             FROM bank_accounts
             ORDER BY updated_at DESC",
        )
        .map_err(|e| e.to_string())?;

    let bank_accounts = bank_stmt
        .query_map([], |row| {
            Ok(BankAccountBalance {
                account_name: row.get(0)?,
                current_balance: row.get(1)?,
                available_balance: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let total_credit_card_balance = credit_cards.iter().map(|c| c.statement_balance).sum::<f64>();
    let total_credit_limit = credit_cards.iter().map(|c| c.credit_limit).sum::<f64>();
    let total_bank_balance = bank_accounts.iter().map(|b| b.current_balance).sum::<f64>();
    let credit_utilization_percent = if total_credit_limit > 0.0 {
        (total_credit_card_balance / total_credit_limit) * 100.0
    } else {
        0.0
    };

    Ok(FinancialManagerOverview {
        monthly_purchase_total,
        total_credit_card_balance,
        total_bank_balance,
        credit_utilization_percent,
        recent_purchases,
        credit_cards,
        bank_accounts,
    })
}

#[tauri::command]
pub fn send_financial_overview_to_advisor(state: State<'_, DbState>) -> Result<String, String> {
    let overview = get_financial_manager_overview(state.clone())?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;

    let summary = format!(
        "Monthly spend ${:.2}, credit utilization {:.2}%, bank balance ${:.2}",
        overview.monthly_purchase_total,
        overview.credit_utilization_percent,
        overview.total_bank_balance
    );

    conn.execute(
        "INSERT INTO orchestration_signals (source, content, metadata, occurred_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["financial_manager", summary, "advisor_sync", now, now],
    )
    .map_err(|e| e.to_string())?;

    let signal_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO orchestration_actions
         (signal_id, action_type, title, due_date, status, note, created_at)
         VALUES (?1, 'finance_review', ?2, ?3, 'pending', ?4, ?5)",
        params![
            signal_id,
            "Review budget, utilization, and upcoming dues",
            Local::now().format("%Y-%m-%d").to_string(),
            "Generated from financial manager overview",
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok("Financial overview sent to advisor queue".to_string())
}

#[tauri::command]
pub fn get_financial_manager_alerts(state: State<'_, DbState>) -> Result<FinancialManagerAlerts, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let today = Local::now().date_naive();

    let mut recurring_stmt = conn
        .prepare(
            "SELECT item_name, category, AVG(amount), COUNT(*), MAX(purchased_at)
             FROM purchase_history
             GROUP BY item_name, category
             HAVING COUNT(*) >= 2
             ORDER BY COUNT(*) DESC, AVG(amount) DESC
             LIMIT 6",
        )
        .map_err(|e| e.to_string())?;

    let recurring_expenses = recurring_stmt
        .query_map([], |row| {
            let last_date: String = row.get(4)?;
            let parsed = NaiveDate::parse_from_str(&last_date, "%Y-%m-%d").ok();
            let estimated_next = parsed
                .map(|d| (d + Duration::days(30)).format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "unknown".to_string());

            Ok(RecurringExpenseInsight {
                item_name: row.get(0)?,
                category: row.get(1)?,
                average_amount: row.get(2)?,
                occurrences: row.get(3)?,
                last_purchase_date: last_date,
                estimated_next_purchase_date: estimated_next,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut card_stmt = conn
        .prepare(
            "SELECT card_name, due_date, statement_balance, minimum_due
             FROM credit_card_accounts
             WHERE due_date IS NOT NULL",
        )
        .map_err(|e| e.to_string())?;

    let due_reminders = card_stmt
        .query_map([], |row| {
            let due_date: String = row.get(1)?;
            let parsed_due = NaiveDate::parse_from_str(&due_date, "%Y-%m-%d").ok();
            let days_left = parsed_due
                .map(|d| d.signed_duration_since(today).num_days())
                .unwrap_or(999);

            Ok(CardDueReminder {
                card_name: row.get(0)?,
                due_date,
                days_left,
                statement_balance: row.get(2)?,
                minimum_due: row.get(3)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .filter(|item| item.days_left <= 14)
        .collect::<Vec<_>>();

    let alert_summary = format!(
        "{} recurring expenses tracked, {} card dues within 14 days",
        recurring_expenses.len(),
        due_reminders.len()
    );

    Ok(FinancialManagerAlerts {
        recurring_expenses,
        due_reminders,
        alert_summary,
    })
}

#[tauri::command]
pub fn send_financial_alerts_to_advisor(state: State<'_, DbState>) -> Result<String, String> {
    let alerts = get_financial_manager_alerts(state.clone())?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;

    let summary = format!(
        "Financial alerts: {}. Recurring items: {}. Upcoming dues: {}",
        alerts.alert_summary,
        alerts.recurring_expenses.len(),
        alerts.due_reminders.len()
    );

    conn.execute(
        "INSERT INTO orchestration_signals (source, content, metadata, occurred_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params!["financial_alerts", summary, "advisor_sync", now, now],
    )
    .map_err(|e| e.to_string())?;

    let signal_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO orchestration_actions
         (signal_id, action_type, title, due_date, status, note, created_at)
         VALUES (?1, 'finance_due_alert', ?2, ?3, 'pending', ?4, ?5)",
        params![
            signal_id,
            "Review recurring spend and upcoming card dues",
            Local::now().format("%Y-%m-%d").to_string(),
            "Generated from financial alert analytics",
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok("Financial alerts sent to advisor queue".to_string())
}

fn upsert_system_alert(
    conn: &rusqlite::Connection,
    alert_key: &str,
    title: &str,
    detail: &str,
    severity: &str,
    source: &str,
    now: i64,
) -> Result<(), String> {
    conn.execute(
        "INSERT INTO system_alerts
         (alert_key, title, detail, severity, source, status, first_triggered_at, last_triggered_at, occurrences)
         VALUES (?1, ?2, ?3, ?4, ?5, 'active', ?6, ?6, 1)
         ON CONFLICT(alert_key) DO UPDATE SET
            title = excluded.title,
            detail = excluded.detail,
            severity = excluded.severity,
            source = excluded.source,
            status = 'active',
            last_triggered_at = excluded.last_triggered_at,
            occurrences = system_alerts.occurrences + 1,
            acknowledged_at = NULL",
        params![alert_key, title, detail, severity, source, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn evaluate_alerts(state: State<'_, DbState>) -> Result<Vec<SystemAlert>, String> {
    let now = now_ts()?;
    let overview = get_financial_manager_overview(state.clone())?;
    let alerts = get_financial_manager_alerts(state.clone())?;
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;

    if overview.credit_utilization_percent >= 50.0 {
        upsert_system_alert(
            &conn,
            "finance_utilization_critical",
            "Credit utilization is high",
            &format!(
                "Current utilization is {:.2}%. Consider payment or limit management.",
                overview.credit_utilization_percent
            ),
            "critical",
            "financial_manager",
            now,
        )?;
    } else if overview.credit_utilization_percent >= 30.0 {
        upsert_system_alert(
            &conn,
            "finance_utilization_warning",
            "Credit utilization is elevated",
            &format!(
                "Current utilization is {:.2}%. Monitor spending this cycle.",
                overview.credit_utilization_percent
            ),
            "warning",
            "financial_manager",
            now,
        )?;
    }

    if overview.monthly_purchase_total >= 2500.0 {
        upsert_system_alert(
            &conn,
            "monthly_spend_warning",
            "Monthly spending crossed threshold",
            &format!(
                "Current month spend is ${:.2}. Review category breakdown and upcoming purchases.",
                overview.monthly_purchase_total
            ),
            "warning",
            "financial_manager",
            now,
        )?;
    }

    for due in &alerts.due_reminders {
        let severity = if due.days_left <= 3 { "critical" } else if due.days_left <= 7 { "warning" } else { "info" };
        upsert_system_alert(
            &conn,
            &format!("card_due_{}", due.card_name.to_lowercase().replace(' ', "_")),
            &format!("Card due soon: {}", due.card_name),
            &format!(
                "Due {} in {} day(s), statement ${:.2}",
                due.due_date, due.days_left, due.statement_balance
            ),
            severity,
            "financial_manager",
            now,
        )?;
    }

    get_active_alerts(state)
}

#[tauri::command]
pub fn get_active_alerts(state: State<'_, DbState>) -> Result<Vec<SystemAlert>, String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let mut stmt = conn
        .prepare(
            "SELECT id, title, detail, severity, source, status, occurrences, last_triggered_at
             FROM system_alerts
             WHERE status = 'active'
             ORDER BY
               CASE severity
                 WHEN 'critical' THEN 1
                 WHEN 'warning' THEN 2
                 ELSE 3
               END,
               last_triggered_at DESC
             LIMIT 40",
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map([], |row| {
            Ok(SystemAlert {
                id: row.get(0)?,
                title: row.get(1)?,
                detail: row.get(2)?,
                severity: row.get(3)?,
                source: row.get(4)?,
                status: row.get(5)?,
                occurrences: row.get(6)?,
                last_triggered_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    Ok(rows)
}

#[tauri::command]
pub fn acknowledge_alert(alert_id: i64, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "UPDATE system_alerts
         SET status = 'acknowledged', acknowledged_at = ?1
         WHERE id = ?2",
        params![now, alert_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn get_financial_snapshot(state: State<'_, DbState>) -> FinancialSnapshot {
    // Data-driven snapshot (NautilusTrader pattern: Portfolio + DataEngine)
    let portfolio = get_portfolio_summary(state.clone()).await;

    match portfolio {
        Ok(p) => FinancialSnapshot {
            estimated_portfolio_value: p.total_market_value,
            day_change_percent: p.unrealized_pnl_pct,
            watchlist_overview: p.watchlist_overview,
            risk_note: p.risk_note,
            top_movers: p.top_movers,
        },
        Err(_) => {
            // Graceful degradation: return minimal snapshot
            FinancialSnapshot {
                estimated_portfolio_value: 0.0,
                day_change_percent: 0.0,
                watchlist_overview: "Market data unavailable. Refresh to load live quotes.".to_string(),
                risk_note: "No positions tracked yet.".to_string(),
                top_movers: vec![],
            }
        }
    }
}

#[tauri::command]
pub async fn get_stock_research_digest(tickers: Option<Vec<String>>, state: State<'_, DbState>) -> Vec<StockDigestItem> {
    // First try to use cached watchlist data, then fall back to requested tickers
    let default_tickers = vec!["NVDA".to_string(), "MSFT".to_string(), "AAPL".to_string()];
    let symbols = tickers.unwrap_or(default_tickers);

    // Try to get cached prices for richer digest
    let cached = get_cached_quotes(state).await.unwrap_or_default();

    symbols
        .into_iter()
        .take(6)
        .map(|ticker| {
            let upper = ticker.to_uppercase();
            // Check if we have cached data for this symbol
            let cached_quote = cached.iter().find(|q| q.symbol == upper);

            let (headline, sentiment, action_hint) = if let Some(q) = cached_quote {
                let change_str = q.change_percent
                    .map(|c| format!("{:+.2}%", c))
                    .unwrap_or_else(|| "N/A".to_string());
                let sentiment_label = match q.change_percent {
                    Some(c) if c > 2.0 => "Bullish",
                    Some(c) if c > 0.0 => "Constructive",
                    Some(c) if c > -2.0 => "Neutral",
                    _ => "Bearish",
                };
                (
                    format!("${:.2} ({})", q.price, change_str),
                    sentiment_label.to_string(),
                    "Review position sizing and earnings calendar".to_string(),
                )
            } else {
                match upper.as_str() {
                    "NVDA" => (
                        "Datacenter expansion narrative remains intact".to_string(),
                        "Bullish".to_string(),
                        "Review earnings date and protect gains plan".to_string(),
                    ),
                    "MSFT" => (
                        "Enterprise demand stable with AI product attach growth".to_string(),
                        "Constructive".to_string(),
                        "Track cloud guidance revision next quarter".to_string(),
                    ),
                    "AAPL" => (
                        "Hardware cycle mixed while services momentum continues".to_string(),
                        "Neutral".to_string(),
                        "Watch gross margin trend and product event timing".to_string(),
                    ),
                    _ => (
                        "Add to watchlist for live data tracking".to_string(),
                        "Watch".to_string(),
                        "Add alert rule and attach notes before decision".to_string(),
                    ),
                }
            };

            StockDigestItem {
                ticker: upper,
                headline,
                sentiment,
                action_hint,
            }
        })
        .collect::<Vec<_>>()
}

#[tauri::command]
pub fn get_weekly_planning_assistant(
    state: State<'_, DbState>,
) -> Result<WeeklyPlanningAssistant, String> {
    let today = Local::now().date_naive();
    let end_of_week = today + Duration::days(7);
    let week_label = format!(
        "{} to {}",
        today.format("%Y-%m-%d"),
        end_of_week.format("%Y-%m-%d")
    );

    let conn = state.0.lock().map_err(|_| "Failed to acquire DB lock")?;

    let mut calendar_stmt = conn
        .prepare(
            "SELECT title, start_time, end_time
             FROM calendar_events
             WHERE substr(start_time, 1, 10) >= ?1 AND substr(start_time, 1, 10) <= ?2
             ORDER BY start_time ASC
             LIMIT 10",
        )
        .map_err(|e| e.to_string())?;

    let calendar_items = calendar_stmt
        .query_map(
            params![
                today.format("%Y-%m-%d").to_string(),
                end_of_week.format("%Y-%m-%d").to_string()
            ],
            |row| {
                Ok(WeeklyCalendarItem {
                    title: row.get(0)?,
                    start_time: row.get(1)?,
                    end_time: row.get(2)?,
                })
            },
        )
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let mut action_stmt = conn
        .prepare(
            "SELECT title, due_date, action_type
             FROM orchestration_actions
             WHERE status = 'pending'
             ORDER BY due_date ASC, created_at DESC
             LIMIT 12",
        )
        .map_err(|e| e.to_string())?;

    let pending_actions = action_stmt
        .query_map([], |row| {
            Ok(WeeklyActionItem {
                title: row.get(0)?,
                due_date: row.get(1)?,
                action_type: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>();

    let priorities = vec![
        "Protect deep-work blocks before adding new meetings".to_string(),
        "Apply high-impact pending actions with near-term due dates".to_string(),
        "Convert important comms into calendar events + prep notes".to_string(),
    ];

    let recommendation = if pending_actions.is_empty() {
        "Queue is clear. Reserve one proactive planning block for upcoming priorities.".to_string()
    } else {
        "You have pending orchestrated tasks. Apply urgent actions first, then lock calendar slots.".to_string()
    };

    Ok(WeeklyPlanningAssistant {
        week_label,
        priorities,
        calendar_items,
        pending_actions,
        recommendation,
    })
}

// ═══════════════════════════════════════════════════════════════════════════════
// Market Data Engine — NautilusTrader DataEngine + Adapter pattern
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchlistItem {
    pub id: i64,
    pub symbol: String,
    pub name: String,
    pub asset_class: String,
    pub venue: String,
    pub notes: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketQuoteView {
    pub symbol: String,
    pub price: f64,
    pub change_percent: Option<f64>,
    pub volume: Option<f64>,
    pub market_cap: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub cached_at: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WatchlistInput {
    pub symbol: String,
    pub name: Option<String>,
    pub asset_class: Option<String>,
    pub venue: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub id: i64,
    pub symbol: String,
    pub quantity: f64,
    pub avg_cost: f64,
    pub side: String,
    pub status: String,
    pub current_price: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub unrealized_pnl_pct: Option<f64>,
    pub market_value: Option<f64>,
    pub realized_pnl: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PositionInput {
    pub symbol: String,
    pub quantity: f64,
    pub avg_cost: f64,
    pub side: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TradeInput {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub commission: Option<f64>,
    pub note: Option<String>,
    pub position_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortfolioSummary {
    pub total_market_value: f64,
    pub total_cost_basis: f64,
    pub unrealized_pnl: f64,
    pub unrealized_pnl_pct: f64,
    pub realized_pnl: f64,
    pub positions: Vec<Position>,
    pub open_count: usize,
    pub top_movers: Vec<MarketMover>,
    pub risk_note: String,
    pub watchlist_overview: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RiskRule {
    pub id: i64,
    pub rule_key: String,
    pub name: String,
    pub threshold: f64,
    pub severity: String,
    pub is_active: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RiskRuleUpdate {
    pub rule_key: String,
    pub threshold: f64,
    pub is_active: Option<bool>,
}

// ─── Watchlist CRUD ─────────────────────────────────────────────────────────

#[tauri::command]
pub async fn get_watchlist(state: State<'_, DbState>) -> Result<Vec<WatchlistItem>, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let mut stmt = conn.prepare(
        "SELECT id, symbol, name, asset_class, venue, notes, is_active FROM watchlist ORDER BY symbol"
    ).map_err(|e| e.to_string())?;

    let items = stmt.query_map([], |row| {
        Ok(WatchlistItem {
            id: row.get(0)?,
            symbol: row.get(1)?,
            name: row.get(2)?,
            asset_class: row.get(3)?,
            venue: row.get(4)?,
            notes: row.get(5)?,
            is_active: row.get::<_, i64>(6)? != 0,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    Ok(items)
}

#[tauri::command]
pub async fn add_watchlist_item(input: WatchlistInput, state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT OR REPLACE INTO watchlist (symbol, name, asset_class, venue, notes, is_active, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![
            input.symbol.to_uppercase(),
            input.name.unwrap_or_default(),
            input.asset_class.unwrap_or_else(|| "equity".to_string()),
            input.venue.unwrap_or_else(|| "yahoo".to_string()),
            input.notes,
            now
        ],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub async fn remove_watchlist_item(symbol: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    conn.execute("UPDATE watchlist SET is_active = 0 WHERE symbol = ?1", params![symbol.to_uppercase()])
        .map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Market Data Fetch + Cache ──────────────────────────────────────────────

#[tauri::command]
pub async fn refresh_market_data(state: State<'_, DbState>) -> Result<Vec<MarketQuoteView>, String> {
    let watchlist = {
        let conn = state.0.lock().map_err(|_| "DB lock")?;
        let mut stmt = conn.prepare(
            "SELECT symbol, asset_class, venue FROM watchlist WHERE is_active = 1"
        ).map_err(|e| e.to_string())?;
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        }).map_err(|e| e.to_string())?
        .filter_map(Result::ok)
        .collect::<Vec<_>>()
    };

    if watchlist.is_empty() {
        return Ok(vec![]);
    }

    // Partition symbols by venue
    let equity_symbols: Vec<String> = watchlist.iter()
        .filter(|(_, ac, _)| ac == "equity" || ac == "etf")
        .map(|(s, _, _)| s.clone())
        .collect();
    let crypto_symbols: Vec<String> = watchlist.iter()
        .filter(|(_, ac, _)| ac == "crypto")
        .map(|(s, _, _)| s.to_lowercase())
        .collect();

    let mut all_quotes: Vec<MarketQuote> = Vec::new();

    // Fetch from adapters (NautilusTrader adapter pattern)
    if !equity_symbols.is_empty() {
        all_quotes.extend(market_data::fetch_yahoo_quotes(&equity_symbols).await);
    }
    if !crypto_symbols.is_empty() {
        all_quotes.extend(market_data::fetch_coingecko_quotes(&crypto_symbols).await);
    }

    // Upsert into cache
    let now = now_ts()?;
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    for q in &all_quotes {
        conn.execute(
            "INSERT INTO price_cache (symbol, price, change_percent, volume, market_cap, high, low, fetched_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(symbol) DO UPDATE SET
               price = excluded.price, change_percent = excluded.change_percent,
               volume = excluded.volume, market_cap = excluded.market_cap,
               high = excluded.high, low = excluded.low, fetched_at = excluded.fetched_at",
            params![q.symbol, q.price, q.change_percent, q.volume, q.market_cap, q.high, q.low, now],
        ).map_err(|e| e.to_string())?;
    }

    // Return cached view
    let views: Vec<MarketQuoteView> = all_quotes.iter().map(|q| MarketQuoteView {
        symbol: q.symbol.clone(),
        price: q.price,
        change_percent: q.change_percent,
        volume: q.volume,
        market_cap: q.market_cap,
        high: q.high,
        low: q.low,
        cached_at: now,
    }).collect();

    Ok(views)
}

#[tauri::command]
pub async fn get_cached_quotes(state: State<'_, DbState>) -> Result<Vec<MarketQuoteView>, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let mut stmt = conn.prepare(
        "SELECT symbol, price, change_percent, volume, market_cap, high, low, fetched_at
         FROM price_cache ORDER BY symbol"
    ).map_err(|e| e.to_string())?;

    let items = stmt.query_map([], |row| {
        Ok(MarketQuoteView {
            symbol: row.get(0)?,
            price: row.get(1)?,
            change_percent: row.get(2)?,
            volume: row.get(3)?,
            market_cap: row.get(4)?,
            high: row.get(5)?,
            low: row.get(6)?,
            cached_at: row.get(7)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    Ok(items)
}

// ─── Position Tracking (NautilusTrader Portfolio pattern) ───────────────────

#[tauri::command]
pub async fn open_position(input: PositionInput, state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT INTO positions (symbol, quantity, avg_cost, side, status, opened_at)
         VALUES (?1, ?2, ?3, ?4, 'open', ?5)",
        params![
            input.symbol.to_uppercase(),
            input.quantity,
            input.avg_cost,
            input.side.unwrap_or_else(|| "long".to_string()),
            now
        ],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
pub async fn close_position(position_id: i64, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "UPDATE positions SET status = 'closed', closed_at = ?1 WHERE id = ?2",
        params![now, position_id],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_positions(state: State<'_, DbState>) -> Result<Vec<Position>, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let mut stmt = conn.prepare(
        "SELECT p.id, p.symbol, p.quantity, p.avg_cost, p.side, p.status, p.realized_pnl,
                pc.price
         FROM positions p
         LEFT JOIN price_cache pc ON p.symbol = pc.symbol
         WHERE p.status = 'open'
         ORDER BY p.opened_at DESC"
    ).map_err(|e| e.to_string())?;

    let positions = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        let symbol: String = row.get(1)?;
        let quantity: f64 = row.get(2)?;
        let avg_cost: f64 = row.get(3)?;
        let side: String = row.get(4)?;
        let status: String = row.get(5)?;
        let realized_pnl: f64 = row.get(6)?;
        let current_price: Option<f64> = row.get(7)?;

        let (unrealized_pnl, unrealized_pnl_pct, market_value) = if let Some(price) = current_price {
            let direction: f64 = if side == "short" { -1.0 } else { 1.0 };
            let mv = quantity * price;
            let pnl = (price - avg_cost) * quantity * direction;
            let pnl_pct = if avg_cost > 0.0 { ((price - avg_cost) / avg_cost) * 100.0 * direction } else { 0.0 };
            (Some(pnl), Some(pnl_pct), Some(mv))
        } else {
            (None, None, None)
        };

        Ok(Position {
            id, symbol, quantity, avg_cost, side, status,
            current_price, unrealized_pnl, unrealized_pnl_pct, market_value, realized_pnl,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    Ok(positions)
}

#[tauri::command]
pub async fn log_trade(input: TradeInput, state: State<'_, DbState>) -> Result<i64, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let now = now_ts()?;
    conn.execute(
        "INSERT INTO trade_journal (symbol, side, quantity, price, commission, note, position_id, traded_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            input.symbol.to_uppercase(),
            input.side,
            input.quantity,
            input.price,
            input.commission.unwrap_or(0.0),
            input.note,
            input.position_id,
            now
        ],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

// ─── Portfolio Engine (NautilusTrader Portfolio component) ──────────────────

#[tauri::command]
pub async fn get_portfolio_summary(state: State<'_, DbState>) -> Result<PortfolioSummary, String> {
    let positions = get_positions(state.clone()).await?;

    let total_market_value: f64 = positions.iter().filter_map(|p| p.market_value).sum();
    let total_cost_basis: f64 = positions.iter().map(|p| p.quantity * p.avg_cost).sum();
    let unrealized_pnl: f64 = positions.iter().filter_map(|p| p.unrealized_pnl).sum();
    let realized_pnl: f64 = positions.iter().map(|p| p.realized_pnl).sum();
    let unrealized_pnl_pct = if total_cost_basis > 0.0 {
        (unrealized_pnl / total_cost_basis) * 100.0
    } else {
        0.0
    };

    // Top movers from cached prices
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let mut mover_stmt = conn.prepare(
        "SELECT symbol, change_percent FROM price_cache
         WHERE change_percent IS NOT NULL
         ORDER BY ABS(change_percent) DESC LIMIT 5"
    ).map_err(|e| e.to_string())?;

    let top_movers: Vec<MarketMover> = mover_stmt.query_map([], |row| {
        let symbol: String = row.get(0)?;
        let change: f64 = row.get(1)?;
        Ok(MarketMover {
            ticker: symbol,
            change_percent: change,
            reason: "Live market data".to_string(),
        })
    }).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    // Risk assessment
    let risk_note = evaluate_portfolio_risk_inline(&positions, total_market_value, total_cost_basis);

    let watchlist_overview = if positions.is_empty() {
        "No open positions. Add instruments to your watchlist and open positions to track.".to_string()
    } else {
        format!("{} open position(s), ${:.0} total market value", positions.len(), total_market_value)
    };

    Ok(PortfolioSummary {
        total_market_value,
        total_cost_basis,
        unrealized_pnl,
        unrealized_pnl_pct,
        realized_pnl,
        open_count: positions.len(),
        positions,
        top_movers,
        risk_note,
        watchlist_overview,
    })
}

/// Inline risk evaluation — mirrors NautilusTrader RiskEngine check pattern.
fn evaluate_portfolio_risk_inline(positions: &[Position], total_mv: f64, total_cost: f64) -> String {
    if positions.is_empty() {
        return "No risk exposure — portfolio is flat.".to_string();
    }

    let mut notes = Vec::new();

    // Concentration risk: any single position > 30% of portfolio
    if total_mv > 0.0 {
        for p in positions {
            if let Some(mv) = p.market_value {
                let pct = (mv / total_mv) * 100.0;
                if pct > 30.0 {
                    notes.push(format!("Concentration risk: {} is {:.0}% of portfolio", p.symbol, pct));
                }
            }
        }
    }

    // Drawdown risk: unrealized loss > 15% of cost basis
    if total_cost > 0.0 {
        let drawdown_pct = ((total_mv - total_cost) / total_cost) * 100.0;
        if drawdown_pct < -15.0 {
            notes.push(format!("Drawdown alert: portfolio is down {:.1}% from cost basis", drawdown_pct.abs()));
        }
    }

    if notes.is_empty() {
        "Risk within acceptable bounds. No concentration or drawdown alerts.".to_string()
    } else {
        notes.join("; ")
    }
}

// ─── Risk Engine (NautilusTrader RiskEngine pattern) ────────────────────────

#[tauri::command]
pub async fn get_risk_rules(state: State<'_, DbState>) -> Result<Vec<RiskRule>, String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let mut stmt = conn.prepare(
        "SELECT id, rule_key, name, threshold, severity, is_active, description
         FROM risk_rules ORDER BY severity, rule_key"
    ).map_err(|e| e.to_string())?;

    let rules = stmt.query_map([], |row| {
        Ok(RiskRule {
            id: row.get(0)?,
            rule_key: row.get(1)?,
            name: row.get(2)?,
            threshold: row.get(3)?,
            severity: row.get(4)?,
            is_active: row.get::<_, i64>(5)? != 0,
            description: row.get(6)?,
        })
    }).map_err(|e| e.to_string())?
    .filter_map(Result::ok)
    .collect();

    Ok(rules)
}

#[tauri::command]
pub async fn update_risk_rule(input: RiskRuleUpdate, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|_| "DB lock")?;
    let active = input.is_active.map(|a| if a { 1 } else { 0 });

    if let Some(act) = active {
        conn.execute(
            "UPDATE risk_rules SET threshold = ?1, is_active = ?2 WHERE rule_key = ?3",
            params![input.threshold, act, input.rule_key],
        ).map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "UPDATE risk_rules SET threshold = ?1 WHERE rule_key = ?2",
            params![input.threshold, input.rule_key],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// NautilusTrader RiskEngine pattern: evaluate all risk rules against current state.
#[tauri::command]
pub async fn evaluate_risk_engine(state: State<'_, DbState>) -> Result<Vec<SystemAlert>, String> {
    let now = now_ts()?;
    let positions = get_positions(state.clone()).await.unwrap_or_default();
    let overview = get_financial_manager_overview(state.clone())?;

    let total_mv: f64 = positions.iter().filter_map(|p| p.market_value).sum();
    let total_cost: f64 = positions.iter().map(|p| p.quantity * p.avg_cost).sum();

    let rules = get_risk_rules(state.clone()).await.unwrap_or_default();
    let conn = state.0.lock().map_err(|_| "DB lock")?;

    for rule in &rules {
        if !rule.is_active { continue; }

        let triggered = match rule.rule_key.as_str() {
            "concentration_pct" => {
                // Check if any single position exceeds threshold
                total_mv > 0.0 && positions.iter().any(|p| {
                    p.market_value.map(|mv| (mv / total_mv) * 100.0 > rule.threshold).unwrap_or(false)
                })
            }
            "drawdown_pct" => {
                // Check if portfolio drawdown exceeds threshold
                total_cost > 0.0 && ((total_mv - total_cost) / total_cost) * 100.0 < -rule.threshold
            }
            "utilization_pct" => {
                overview.credit_utilization_percent > rule.threshold
            }
            "monthly_spend" => {
                overview.monthly_purchase_total > rule.threshold
            }
            _ => false,
        };

        if triggered {
            let detail = match rule.rule_key.as_str() {
                "concentration_pct" => {
                    let worst = positions.iter()
                        .filter_map(|p| p.market_value.map(|mv| (p.symbol.clone(), (mv / total_mv) * 100.0)))
                        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                    match worst {
                        Some((sym, pct)) => format!("{} is {:.1}% of portfolio (threshold: {:.0}%)", sym, pct, rule.threshold),
                        None => format!("Concentration exceeds {:.0}% threshold", rule.threshold),
                    }
                }
                "drawdown_pct" => {
                    let dd = if total_cost > 0.0 { ((total_mv - total_cost) / total_cost) * 100.0 } else { 0.0 };
                    format!("Portfolio drawdown is {:.1}% (threshold: {:.0}%)", dd.abs(), rule.threshold)
                }
                "utilization_pct" => {
                    format!("Credit utilization is {:.1}% (threshold: {:.0}%)", overview.credit_utilization_percent, rule.threshold)
                }
                "monthly_spend" => {
                    format!("Monthly spend is ${:.0} (threshold: ${:.0})", overview.monthly_purchase_total, rule.threshold)
                }
                _ => format!("Risk rule '{}' triggered", rule.rule_key),
            };

            let _ = upsert_system_alert(
                &conn,
                &format!("risk_engine_{}", rule.rule_key),
                &format!("Risk: {}", rule.name),
                &detail,
                &rule.severity,
                "risk_engine",
                now,
            );
        }
    }

    drop(conn);
    get_active_alerts(state)
}
