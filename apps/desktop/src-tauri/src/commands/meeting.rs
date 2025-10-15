//! Meeting and audio capture commands

use crate::domain::models::{Meeting, Platform};
use crate::ports::audio::AudioCapturePort;
use crate::ports::storage::StoragePort;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;

/// Request to start a new meeting
#[derive(Debug, Deserialize)]
pub struct StartMeetingRequest {
    pub platform: String, // "teams", "zoom", "meet"
    pub title: Option<String>,
}

/// Meeting status response
#[derive(Debug, Serialize)]
pub struct MeetingStatus {
    pub meeting_id: Option<i64>,
    pub is_recording: bool,
    pub platform: Option<String>,
    pub title: Option<String>,
    pub start_time: Option<i64>,
    pub duration_seconds: Option<i64>,
}

/// Audio capture status
#[derive(Debug, Serialize)]
pub struct AudioCaptureStatus {
    pub is_capturing: bool,
    pub device: Option<String>,
    pub format: AudioFormatInfo,
}

/// Audio format information
#[derive(Debug, Serialize)]
pub struct AudioFormatInfo {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

/// Start a new meeting and begin audio capture
#[tauri::command]
pub async fn start_meeting(
    state: tauri::State<'_, AppState>,
    request: StartMeetingRequest,
) -> Result<i64, String> {
    log::info!("Starting meeting for platform: {}", request.platform);

    // Parse platform
    let platform = match request.platform.as_str() {
        "teams" => Platform::Teams,
        "zoom" => Platform::Zoom,
        "meet" => Platform::Meet,
        _ => return Err(format!("Invalid platform: {}", request.platform)),
    };

    // Create meeting record
    let meeting = Meeting::new(platform, request.title.clone());
    let meeting_id = state
        .storage
        .create_meeting(&meeting)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Created meeting with ID: {}", meeting_id);

    // Start audio capture and wait for confirmation
    // This ensures we only store the meeting ID if audio capture actually started
    let mut audio_capture = state.audio_capture.lock().await;
    match audio_capture.start_capture(None).await {
        Ok(_) => {
            log::info!(
                "Audio capture started successfully for meeting {}",
                meeting_id
            );

            // Store current meeting ID only after successful audio capture
            *state.current_meeting_id.lock().await = Some(meeting_id);

            Ok(meeting_id)
        }
        Err(e) => {
            log::error!("Failed to start audio capture: {}", e);

            // Audio capture failed - delete the meeting record to maintain consistency
            if let Err(delete_err) = state.storage.delete_meeting(meeting_id).await {
                log::error!(
                    "Failed to cleanup meeting record after audio capture failure: {}",
                    delete_err
                );
            }

            Err(format!("Failed to start audio capture: {}", e))
        }
    }
}

/// Stop the current meeting and audio capture
#[tauri::command]
pub async fn stop_meeting(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    meeting_id: i64,
) -> Result<(), String> {
    log::info!("Stopping meeting ID: {}", meeting_id);

    // Stop audio capture and save audio file in background
    let audio_capture_arc = Arc::clone(&state.audio_capture);
    let storage_arc = Arc::clone(&state.storage);

    tokio::spawn(async move {
        // Get the audio buffer BEFORE releasing the mutex
        // This ensures we extract the data while holding the lock, then release it
        // before doing slow file I/O operations
        let buffer_result = {
            let mut audio_capture = audio_capture_arc.lock().await;

            // Stop capture
            if let Err(e) = audio_capture.stop_capture().await {
                log::error!("Failed to stop audio capture: {}", e);
                return;
            }

            // Get audio buffer - this is quick, just moving data
            audio_capture.get_audio_buffer().await
        }; // Mutex is released here, before slow file operations

        // Now perform slow file I/O operations without holding the mutex
        match buffer_result {
            Ok(Some(buffer)) => {
                // Get app data directory for secure storage
                let app_data_dir = match app.path().app_data_dir() {
                    Ok(dir) => dir,
                    Err(e) => {
                        log::error!("Failed to get app data directory: {}", e);
                        return;
                    }
                };

                // Create audio recordings subdirectory with restricted permissions
                let audio_dir = app_data_dir.join("recordings");
                if let Err(e) = std::fs::create_dir_all(&audio_dir) {
                    log::error!("Failed to create recordings directory: {}", e);
                    return;
                }

                // Save audio file with meeting ID for uniqueness
                let audio_file = audio_dir.join(format!("meeting_{}.wav", meeting_id));

                // File I/O happens here - potentially slow, but mutex is NOT held
                match crate::utils::audio_file::save_wav_file(&buffer, &audio_file) {
                    Ok(samples_written) => {
                        log::info!(
                            "Saved {} samples to secure location: {}",
                            samples_written,
                            audio_file.display()
                        );

                        // Store audio file path in database
                        let file_path_str = audio_file.to_string_lossy().to_string();
                        match storage_arc.get_meeting(meeting_id).await {
                            Ok(Some(mut meeting)) => {
                                meeting.audio_file_path = Some(file_path_str);
                                if let Err(e) = storage_arc.update_meeting(&meeting).await {
                                    log::error!(
                                        "Failed to update meeting with audio file path: {}",
                                        e
                                    );
                                }
                            }
                            Ok(None) => {
                                log::error!("Meeting {} not found", meeting_id);
                            }
                            Err(e) => {
                                log::error!("Failed to get meeting: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to save audio file: {}", e);
                    }
                }
            }
            Ok(None) => {
                log::warn!("No audio buffer to save");
            }
            Err(e) => {
                log::error!("Failed to get audio buffer: {}", e);
            }
        }
    });

    // Clear current meeting ID
    *state.current_meeting_id.lock().await = None;

    // Update meeting end time
    let mut meeting = state
        .storage
        .get_meeting(meeting_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))?;

    meeting.end();

    state
        .storage
        .update_meeting(&meeting)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Meeting {} stopped", meeting_id);
    Ok(())
}

/// Get current meeting status
#[tauri::command]
pub async fn get_meeting_status(
    state: tauri::State<'_, AppState>,
) -> Result<MeetingStatus, String> {
    let current_meeting_id = *state.current_meeting_id.lock().await;

    if let Some(meeting_id) = current_meeting_id {
        // Get meeting from database
        let meeting = state
            .storage
            .get_meeting(meeting_id)
            .await
            .map_err(|e| e.to_string())?;

        if let Some(meeting) = meeting {
            // Calculate duration
            let duration_seconds = if let Some(end_time) = meeting.end_time {
                Some(end_time - meeting.start_time)
            } else {
                // Meeting is ongoing, calculate from current time
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64;
                Some(now - meeting.start_time)
            };

            // Check if audio capture is active
            let is_recording = state.audio_capture.lock().await.is_capturing();

            return Ok(MeetingStatus {
                meeting_id: Some(meeting_id),
                is_recording,
                platform: Some(meeting.platform.to_string()),
                title: meeting.title,
                start_time: Some(meeting.start_time),
                duration_seconds,
            });
        }
    }

    // No active meeting
    Ok(MeetingStatus {
        meeting_id: None,
        is_recording: false,
        platform: None,
        title: None,
        start_time: None,
        duration_seconds: None,
    })
}

/// Get audio capture status
#[tauri::command]
pub async fn get_audio_capture_status(
    state: tauri::State<'_, AppState>,
) -> Result<AudioCaptureStatus, String> {
    let audio_capture = state.audio_capture.lock().await;
    let is_capturing = audio_capture.is_capturing();
    let format = audio_capture.get_format();

    Ok(AudioCaptureStatus {
        is_capturing,
        device: if is_capturing {
            Some("System Audio".to_string())
        } else {
            None
        },
        format: AudioFormatInfo {
            sample_rate: format.sample_rate as u32,
            channels: format.channels,
            bits_per_sample: format.bits_per_sample,
        },
    })
}

/// List available audio devices
#[tauri::command]
pub async fn list_audio_devices(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let audio_capture = state.audio_capture.lock().await;
    audio_capture
        .list_devices()
        .await
        .map_err(|e| e.to_string())
}

/// Get meeting history
#[tauri::command]
pub async fn get_meeting_history(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<Meeting>, String> {
    let meetings = state
        .storage
        .list_meetings(Some(limit.unwrap_or(50) as i32), Some(0))
        .await
        .map_err(|e| e.to_string())?;

    Ok(meetings)
}

/// Get a specific meeting by ID
#[tauri::command]
pub async fn get_meeting(
    state: tauri::State<'_, AppState>,
    meeting_id: i64,
) -> Result<Meeting, String> {
    state
        .storage
        .get_meeting(meeting_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Meeting not found: {}", meeting_id))
}

/// Delete a meeting
#[tauri::command]
pub async fn delete_meeting(
    state: tauri::State<'_, AppState>,
    meeting_id: i64,
) -> Result<(), String> {
    state
        .storage
        .delete_meeting(meeting_id)
        .await
        .map_err(|e| e.to_string())?;

    log::info!("Deleted meeting: {}", meeting_id);
    Ok(())
}
