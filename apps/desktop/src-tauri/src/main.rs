// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adapters;
mod commands;
mod domain;
mod error;
mod ports;
mod utils;

use adapters::storage::SqliteStorage;
use error::Result;
use ports::storage::StoragePort;
use std::sync::Arc;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconEvent},
    Manager, Runtime,
};
use tokio::sync::Mutex;
use utils::keychain::KeychainManager;

#[cfg(target_os = "linux")]
use adapters::audio::PulseAudioCapture;
#[cfg(target_os = "windows")]
use adapters::audio::WasapiAudioCapture;

#[cfg(target_os = "windows")]
type AudioCapture = WasapiAudioCapture;
#[cfg(target_os = "linux")]
type AudioCapture = PulseAudioCapture;

/// Application state shared across Tauri commands
pub struct AppState {
    pub storage: Arc<SqliteStorage>,
    pub keychain: Arc<KeychainManager>,
    pub audio_capture: Arc<Mutex<AudioCapture>>,
    pub current_meeting_id: Arc<Mutex<Option<i64>>>,
}

/// Initialize the application
///
/// Sets up database connection and runs migrations.
fn initialize_app(
    app: &tauri::AppHandle,
) -> Result<(
    AppState,
    commands::transcription::TranscriptionState,
    commands::streaming::StreamingTranscriptionState,
)> {
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

    let storage_arc = Arc::new(storage);
    let keychain_arc = Arc::new(KeychainManager::new());

    let app_state = AppState {
        storage: Arc::clone(&storage_arc),
        keychain: Arc::clone(&keychain_arc),
        audio_capture: Arc::new(Mutex::new(AudioCapture::new())),
        current_meeting_id: Arc::new(Mutex::new(None)),
    };

    let transcription_state = commands::transcription::TranscriptionState {
        storage: Arc::clone(&storage_arc),
        keychain: Arc::clone(&keychain_arc),
        current_transcription: Arc::new(Mutex::new(None)),
    };

    let streaming_state = commands::streaming::StreamingTranscriptionState::new();

    Ok((app_state, transcription_state, streaming_state))
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

/// Update the tray icon tooltip with recording status
#[tauri::command]
async fn update_tray_status(
    app: tauri::AppHandle,
    is_recording: bool,
) -> std::result::Result<(), String> {
    if let Some(tray) = app.tray_by_id("main") {
        let tooltip = if is_recording {
            "Meet Scribe - Recording..."
        } else {
            "Meet Scribe - Idle"
        };
        tray.set_tooltip(Some(tooltip))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Setup system tray menu
fn setup_tray_menu<R: Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show Window", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "hide", "Hide Window", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    let tray = app.tray_by_id("main").expect("Failed to get tray");
    tray.set_menu(Some(menu))?;

    tray.on_tray_icon_event(|tray, event| {
        if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            ..
        } = event
        {
            let app = tray.app_handle();
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    });

    tray.on_menu_event(move |app, event| match event.id().as_ref() {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "hide" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }
        }
        "quit" => {
            app.exit(0);
        }
        _ => {}
    });

    Ok(())
}

fn main() {
    // Initialize logger
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            // Initialize app state
            let (app_state, transcription_state, streaming_state) = initialize_app(app.handle())?;
            app.manage(app_state);
            app.manage(transcription_state);
            app.manage(streaming_state);

            // Setup system tray
            setup_tray_menu(app.handle())?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // Prevent window from closing, hide it instead
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_version,
            check_db_health,
            update_tray_status,
            // Config commands
            commands::config::save_api_key,
            commands::config::get_api_key_status,
            commands::config::delete_api_key,
            commands::config::save_service_config,
            commands::config::get_service_config,
            commands::config::get_active_service_config,
            commands::config::list_service_configs,
            commands::config::activate_service,
            // Meeting commands
            commands::meeting::start_meeting,
            commands::meeting::stop_meeting,
            commands::meeting::get_meeting_status,
            commands::meeting::get_audio_capture_status,
            commands::meeting::list_audio_devices,
            commands::meeting::list_speaker_devices,
            commands::meeting::list_microphone_devices,
            commands::meeting::get_meeting_history,
            commands::meeting::get_meeting,
            commands::meeting::delete_meeting,
            commands::meeting::test_speaker_capture,
            commands::meeting::test_microphone_capture,
            commands::meeting::stop_audio_test,
            // Transcription commands (batch)
            commands::transcription::start_transcription,
            commands::transcription::get_transcription_status,
            commands::transcription::get_transcripts,
            commands::transcription::is_transcription_available,
            commands::transcription::delete_transcripts,
            commands::transcription::fetch_asr_models,
            // Streaming transcription commands (real-time)
            commands::streaming::start_streaming_transcription,
            commands::streaming::stop_streaming_transcription,
            commands::streaming::send_audio_chunk,
            commands::streaming::get_streaming_transcription_status,
            // LLM commands
            commands::llm::fetch_llm_models,
            commands::llm::save_llm_api_key,
            commands::llm::check_llm_api_key,
            commands::llm::delete_llm_api_key,
            commands::llm::generate_insights,
            commands::llm::get_default_prompts,
            commands::llm::list_llm_providers,
            commands::llm::generate_meeting_insights,
            commands::llm::get_meeting_insights,
            commands::llm::update_insight,
            commands::llm::delete_meeting_insights,
            // Participant commands
            commands::participant::get_speaker_summary,
            commands::participant::link_speaker_to_participant,
            commands::participant::unlink_speaker,
            commands::participant::delete_meeting_participants,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
