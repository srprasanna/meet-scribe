// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adapters;
mod domain;
mod error;
mod ports;

use adapters::storage::SqliteStorage;
use error::Result;
use ports::storage::StoragePort;
use std::sync::Arc;
use tauri::Manager;

/// Application state shared across Tauri commands
pub struct AppState {
    pub storage: Arc<SqliteStorage>,
}

/// Initialize the application
///
/// Sets up database connection and runs migrations.
fn initialize_app(app: &tauri::AppHandle) -> Result<AppState> {
    // Get application data directory
    let app_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| error::AppError::Config(e.to_string()))?;

    // Create directory if it doesn't exist
    std::fs::create_dir_all(&app_dir)?;

    // Initialize SQLite database
    let db_path = app_dir.join("meet-scribe.db");
    let storage = SqliteStorage::new(db_path)?;

    // Run migrations
    storage.run_migrations()?;

    Ok(AppState {
        storage: Arc::new(storage),
    })
}

/// Example Tauri command - gets application version
#[tauri::command]
fn get_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Example Tauri command - checks database health
#[tauri::command]
async fn check_db_health(state: tauri::State<'_, AppState>) -> std::result::Result<String, String> {
    // Simple health check - try to list meetings
    match state.storage.list_meetings(Some(1), Some(0)).await {
        Ok(_) => Ok("Database is healthy".to_string()),
        Err(e) => Err(e.to_string()),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize app state
            let state = initialize_app(app.handle())?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_version, check_db_health])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
