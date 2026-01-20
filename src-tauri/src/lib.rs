pub mod commands;
pub mod error;
pub mod models;
pub mod services;
pub mod state;

pub use error::{TuskError, TuskResult};

use commands::{health_check, get_app_info, get_log_directory};
use commands::storage::{
    list_connections, get_connection, save_connection, delete_connection,
    get_preference, set_preference, check_database_health,
    check_keychain_available, has_stored_password,
};
use commands::connection::{
    test_connection, connect, disconnect, get_active_connections,
};
use commands::query::{
    execute_query, cancel_query, get_running_queries,
};
use services::{init_logging, AppDirs};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(debug_assertions)]
    let is_debug = true;
    #[cfg(not(debug_assertions))]
    let is_debug = false;

    #[cfg(debug_assertions)]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_mcp_bridge::init());

    #[cfg(not(debug_assertions))]
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init());

    builder
        .invoke_handler(tauri::generate_handler![
            health_check,
            get_app_info,
            get_log_directory,
            list_connections,
            get_connection,
            save_connection,
            delete_connection,
            get_preference,
            set_preference,
            check_database_health,
            check_keychain_available,
            has_stored_password,
            test_connection,
            connect,
            disconnect,
            get_active_connections,
            execute_query,
            cancel_query,
            get_running_queries,
        ])
        .setup(move |app| {
            // Initialize application directories first
            let app_dirs = AppDirs::new(app)?;

            // Initialize logging to the log directory
            // The LogGuard is stored in a Box::leak to keep it alive for the app lifetime
            let log_guard = init_logging(&app_dirs.log_dir, is_debug);
            Box::leak(Box::new(log_guard));

            let info = app.package_info();
            tracing::info!(
                "Starting {} v{} on {}",
                info.name,
                info.version,
                std::env::consts::OS
            );
            tracing::info!("Data directory: {:?}", app_dirs.data_dir);
            tracing::info!("Log directory: {:?}", app_dirs.log_dir);

            // Create application state
            let state = AppState::new(&app_dirs)?;
            app.manage(state);

            #[cfg(debug_assertions)]
            {
                use tauri::Manager;
                if let Some(window) = app.get_webview_window("main") {
                    window.open_devtools();
                }
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Handle graceful shutdown
            if let tauri::WindowEvent::CloseRequested { .. } = event {
                tracing::info!(
                    "Window close requested for: {}",
                    window.label()
                );
            }
            if let tauri::WindowEvent::Destroyed = event {
                tracing::info!("Window destroyed: {}", window.label());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
