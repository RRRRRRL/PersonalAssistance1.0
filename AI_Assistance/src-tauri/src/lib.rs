mod commands;
mod db;

use commands::{
    acknowledge_alert,
    add_orchestration_rule, apply_orchestration_action, apply_orchestration_actions,
    add_purchase_record, begin_ai_stream_session, complete_orchestration_action,
    evaluate_alerts,
    finalize_ai_stream_session, get_ai_provider_settings, get_calendar_events, get_desktop_files,
    get_active_alerts,
    get_financial_manager_alerts, get_financial_manager_overview, get_financial_snapshot,
    get_orchestration_agent_settings, get_orchestration_agent_status,
    get_orchestration_queue, get_orchestration_rules, get_recent_and_frequent_files,
    get_file_grouping_policy, get_voice_settings, open_file,
    get_stock_research_digest, get_weekly_planning_assistant, log_file_access,
    preview_file_grouping_batch, run_file_grouping_batch, save_file_grouping_policy,
    process_orchestration_signal, save_ai_token_batch, send_financial_alerts_to_advisor,
    send_financial_overview_to_advisor, set_ai_provider_settings,
    set_orchestration_agent_settings, set_orchestration_rule_active,
    set_voice_settings, start_orchestration_agent, stop_orchestration_agent,
    stream_ai_response, suggest_related_files, synthesize_speech,
    transcribe_audio, upsert_bank_account_balance, upsert_credit_card_account,
    inspect_sensitive_text, get_youtube_music_playlists, get_youtube_music_status,
    save_youtube_music_settings, search_youtube_music,
    check_email_breach, generate_secure_password, get_exchange_rates, get_network_info,
    get_public_holidays, get_weather_summary, lookup_word, scan_url_safety,
    get_self_review_traces, get_self_review_detail, submit_self_review_score,
    search_vector_memories, get_vector_memory_stats, run_sandbox_code, get_integration_capabilities,
    // Lightweight integrations
    render_markdown, get_system_stats,
    get_clipboard_text, set_clipboard_text, get_clipboard_history, clear_clipboard_history,
    // Market Data Engine
    get_watchlist, add_watchlist_item, remove_watchlist_item,
    refresh_market_data, get_cached_quotes,
    // Position tracking
    open_position, close_position, get_positions, log_trade,
    // Portfolio + Risk Engine
    get_portfolio_summary, get_risk_rules, update_risk_rule, evaluate_risk_engine,
};
use db::{init_db, AgentState, AgentStateShared, DbState};
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::default().build())
        .setup(|app| {
            let conn = init_db().expect("Failed to initialize database");
            app.manage(DbState(std::sync::Mutex::new(conn)));

            // Initialize agent state for the LangGraph sidecar
            let agent_state: AgentStateShared = Arc::new(Mutex::new(AgentState::new(8765)));
            app.manage(agent_state);

            if let Some(window) = app.get_webview_window("main") {
                let window_for_shortcut = window.clone();
                tauri_plugin_global_shortcut::register("Alt+Space", move || {
                    if window_for_shortcut.is_visible().unwrap_or(false) {
                        let _ = window_for_shortcut.hide();
                    } else {
                        let _ = window_for_shortcut.show();
                        let _ = window_for_shortcut.set_focus();
                    }
                })
                .expect("Failed to register Alt+Space global shortcut");
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_desktop_files,
            log_file_access,
            open_file,
            get_recent_and_frequent_files,
            get_file_grouping_policy,
            save_file_grouping_policy,
            preview_file_grouping_batch,
            run_file_grouping_batch,
            begin_ai_stream_session,
            save_ai_token_batch,
            finalize_ai_stream_session,
            stream_ai_response,
            get_ai_provider_settings,
            set_ai_provider_settings,
            get_financial_snapshot,
            get_stock_research_digest,
            get_weekly_planning_assistant,
            add_purchase_record,
            upsert_credit_card_account,
            upsert_bank_account_balance,
            get_financial_manager_overview,
            get_financial_manager_alerts,
            evaluate_alerts,
            get_active_alerts,
            acknowledge_alert,
            send_financial_overview_to_advisor,
            send_financial_alerts_to_advisor,
            // Market Data Engine
            get_watchlist,
            add_watchlist_item,
            remove_watchlist_item,
            refresh_market_data,
            get_cached_quotes,
            // Position tracking
            open_position,
            close_position,
            get_positions,
            log_trade,
            // Portfolio + Risk Engine
            get_portfolio_summary,
            get_risk_rules,
            update_risk_rule,
            evaluate_risk_engine,
            get_calendar_events,
            process_orchestration_signal,
            get_orchestration_queue,
            complete_orchestration_action,
            apply_orchestration_action,
            apply_orchestration_actions,
            get_orchestration_rules,
            add_orchestration_rule,
            set_orchestration_rule_active,
            suggest_related_files,
            start_orchestration_agent,
            stop_orchestration_agent,
            get_orchestration_agent_status,
            get_orchestration_agent_settings,
            set_orchestration_agent_settings,
            inspect_sensitive_text,
            get_voice_settings,
            set_voice_settings,
            transcribe_audio,
            synthesize_speech,
            get_youtube_music_status,
            save_youtube_music_settings,
            get_youtube_music_playlists,
            search_youtube_music,
            get_weather_summary,
            get_public_holidays,
            scan_url_safety,
            check_email_breach,
            get_network_info,
            generate_secure_password,
            get_exchange_rates,
            lookup_word,
            get_self_review_traces,
            get_self_review_detail,
            submit_self_review_score,
            search_vector_memories,
            get_vector_memory_stats,
            run_sandbox_code,
            get_integration_capabilities,
            // Lightweight integrations
            render_markdown,
            get_system_stats,
            get_clipboard_text,
            set_clipboard_text,
            get_clipboard_history,
            clear_clipboard_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
