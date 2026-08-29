pub mod commands;
pub mod components;
pub mod database;
pub mod domain;
pub mod error;
pub mod frontend;
pub mod hardware;
pub mod providers;
pub mod routing;
pub mod runtime;
pub mod security;
pub mod workers;

use commands::AppState;
use database::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let data_directory = app.path().app_data_dir()?;
            let database = Database::open(data_directory.join("harness.sqlite3"))?;
            app.manage(AppState {
                database: std::sync::Mutex::new(database),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_health,
            commands::bootstrap,
            commands::create_project,
            commands::list_projects,
            commands::create_chat_group,
            commands::list_chat_groups,
            commands::create_conversation,
            commands::list_conversations,
            commands::append_message,
            commands::list_messages,
            commands::search_messages,
            commands::detect_providers,
            commands::route_request,
            commands::save_task_graph,
            commands::record_usage,
            commands::usage_summary,
            commands::analyze_hardware,
            commands::local_ai_recommendation,
            commands::validate_provider_manifest,
            commands::plan_component_update,
            commands::evaluate_permission,
            commands::plan_cli_invocation,
            frontend::get_harness_snapshot,
            frontend::send_message,
            frontend::cancel_agent_run,
            frontend::search_harness,
            frontend::choose_workspace,
        ])
        .run(tauri::generate_context!())
        .expect("Harness Desktop runtime failed");
}
