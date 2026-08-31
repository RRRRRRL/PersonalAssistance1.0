pub mod ai;
pub mod advisor;
pub mod calendar;
pub mod clipboard;
pub mod files;
pub mod market_data;
pub mod orchestration;
pub mod sensitive;
pub mod system_monitor;
pub mod voice;
pub mod widgets;
pub mod youtube_music;

pub use ai::{
	begin_ai_stream_session, finalize_ai_stream_session, get_ai_provider_settings,
	save_ai_token_batch, set_ai_provider_settings, stream_ai_response,
};
pub use advisor::{
	acknowledge_alert, add_purchase_record, evaluate_alerts, get_active_alerts,
	get_financial_manager_overview, get_financial_snapshot,
	get_financial_manager_alerts, get_stock_research_digest, get_weekly_planning_assistant,
	send_financial_alerts_to_advisor, send_financial_overview_to_advisor,
	upsert_bank_account_balance, upsert_credit_card_account,
	// Market Data Engine
	get_watchlist, add_watchlist_item, remove_watchlist_item,
	refresh_market_data, get_cached_quotes,
	// Position tracking
	open_position, close_position, get_positions, log_trade,
	// Portfolio + Risk Engine
	get_portfolio_summary, get_risk_rules, update_risk_rule, evaluate_risk_engine,
};
pub use calendar::get_calendar_events;
pub use files::{
	get_desktop_files, get_file_grouping_policy, get_recent_and_frequent_files, log_file_access,
	open_file, preview_file_grouping_batch, run_file_grouping_batch, save_file_grouping_policy,
};
pub use orchestration::{
	add_orchestration_rule, apply_orchestration_action, apply_orchestration_actions,
	complete_orchestration_action, get_orchestration_agent_settings,
	get_orchestration_agent_status, get_orchestration_queue, get_orchestration_rules,
	process_orchestration_signal, set_orchestration_agent_settings, set_orchestration_rule_active,
	start_orchestration_agent, stop_orchestration_agent, suggest_related_files,
	search_vector_memories, get_vector_memory_stats, run_sandbox_code, get_integration_capabilities,
};
pub use sensitive::inspect_sensitive_text;
pub use voice::{
	get_voice_settings, set_voice_settings, synthesize_speech, transcribe_audio,
};
pub use youtube_music::{
	get_youtube_music_playlists, get_youtube_music_status, save_youtube_music_settings,
	search_youtube_music,
};
pub use widgets::{
	check_email_breach, generate_secure_password, get_exchange_rates, get_network_info,
	get_public_holidays, get_weather_summary, lookup_word, scan_url_safety,
	get_self_review_traces, get_self_review_detail, submit_self_review_score,
	render_markdown,
};
pub use system_monitor::get_system_stats;
pub use clipboard::{
	get_clipboard_text, set_clipboard_text, get_clipboard_history, clear_clipboard_history,
};
