mod commands;

use std::sync::Mutex;

use commands::engine::EngineState;
use rmeter_core::http::client::HttpClient;
use rmeter_core::http::history::RequestHistory;
use rmeter_core::plan::PlanManager;
use rmeter_core::results::ResultStore;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(HttpClient::new())
        .manage(Mutex::new(RequestHistory::new(100)))
        .manage(Mutex::new(PlanManager::new()))
        .manage(Mutex::new(EngineState::new()))
        .manage(Mutex::new(ResultStore::new(20)))
        .invoke_handler(tauri::generate_handler![
            // HTTP request commands
            commands::request::send_request,
            commands::request::get_request_history,
            commands::request::clear_request_history,
            // Plan CRUD
            commands::plan::create_plan,
            commands::plan::get_plan,
            commands::plan::list_plans,
            commands::plan::delete_plan,
            commands::plan::set_active_plan,
            commands::plan::get_active_plan,
            // File I/O
            commands::plan::save_plan,
            commands::plan::load_plan,
            // Thread group operations
            commands::plan::add_thread_group,
            commands::plan::remove_thread_group,
            commands::plan::update_thread_group,
            // Request operations
            commands::plan::add_request,
            commands::plan::remove_request,
            commands::plan::update_request,
            // Utilities
            commands::plan::duplicate_element,
            commands::plan::reorder_thread_groups,
            commands::plan::reorder_requests,
            commands::plan::toggle_element,
            commands::plan::rename_element,
            // Assertion operations
            commands::plan::add_assertion,
            commands::plan::remove_assertion,
            commands::plan::update_assertion,
            // Variable operations
            commands::plan::add_variable,
            commands::plan::remove_variable,
            commands::plan::update_variable,
            // Extractor operations
            commands::plan::add_extractor,
            commands::plan::remove_extractor,
            commands::plan::update_extractor,
            // Templates
            commands::plan::create_from_template,
            // CSV Data Source operations
            commands::plan::add_csv_data_source,
            commands::plan::remove_csv_data_source,
            commands::plan::update_csv_data_source,
            // Engine / Load-test execution
            commands::engine::start_test,
            commands::engine::stop_test,
            commands::engine::force_stop_test,
            commands::engine::get_engine_status,
            commands::engine::get_current_stats,
            commands::engine::get_time_series,
            // Results & Export
            commands::results::list_results,
            commands::results::get_result,
            commands::results::export_results_csv,
            commands::results::export_results_json,
            commands::results::export_results_html,
            commands::results::compare_run_results,
            // Extended protocol support
            commands::protocol::test_websocket,
            commands::protocol::send_graphql,
            commands::protocol::graphql_introspect,
        ])
        .run(tauri::generate_context!())
        .expect("error while running rmeter application");
}
