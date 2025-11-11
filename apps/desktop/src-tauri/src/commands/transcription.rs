/// Transcription-related Tauri commands
///
/// Provides IPC commands for triggering and managing transcription operations.
use crate::adapters::services::asr::get_active_asr_service;
use crate::adapters::storage::SqliteStorage;
use crate::domain::models::Transcript;
use crate::ports::storage::StoragePort;
use crate::ports::transcription::TranscriptionConfig;
use crate::utils::keychain::KeychainManager;
use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

/// Application state for transcription operations
pub struct TranscriptionState {
    pub storage: Arc<SqliteStorage>,
    pub keychain: Arc<KeychainManager>,
    /// Current transcription status: None, Some(meeting_id) if in progress
    pub current_transcription: Arc<Mutex<Option<i64>>>,
}

/// Start transcription for a completed meeting
///
/// This command triggers the transcription process for a meeting's audio file.
/// It runs asynchronously and updates the database with transcript segments as they arrive.
///
/// # Arguments
/// * `meeting_id` - The ID of the meeting to transcribe
/// * `config` - Optional transcription configuration (uses defaults if None)
///
/// # Returns
/// * `Ok(())` if transcription started successfully
/// * `Err(String)` if there's an error
#[tauri::command]
pub async fn start_transcription(
    meeting_id: i64,
    config: Option<TranscriptionConfig>,
    state: State<'_, TranscriptionState>,
) -> Result<(), String> {
    log::info!("Starting transcription for meeting {}", meeting_id);

    // Check if a transcription is already in progress
    let mut current = state.current_transcription.lock().await;
    if current.is_some() {
        return Err("A transcription is already in progress".to_string());
    }

    // Mark this meeting as being transcribed
    *current = Some(meeting_id);
    drop(current); // Release lock

    // Get the meeting details
    let meeting = state
        .storage
        .get_meeting(meeting_id)
        .await
        .map_err(|e| format!("Failed to get meeting: {}", e))?
        .ok_or_else(|| format!("Meeting {} not found", meeting_id))?;

    // Check if audio file exists
    let audio_file_path = meeting
        .audio_file_path
        .ok_or_else(|| "Meeting has no audio file".to_string())?;

    // Get the active ASR service
    let asr_service = get_active_asr_service(&state.storage, &state.keychain)
        .await
        .map_err(|e| format!("Failed to get ASR service: {}", e))?;

    // Use provided config or defaults
    let transcription_config = config.unwrap_or_default();

    // Clone state for the background task
    let storage = Arc::clone(&state.storage);
    let current_transcription = Arc::clone(&state.current_transcription);

    // Spawn transcription task in background
    tokio::spawn(async move {
        log::info!("Transcribing audio file: {}", audio_file_path);

        // Perform transcription
        let result = match asr_service
            .transcribe_file(&audio_file_path, &transcription_config)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                log::error!("Transcription failed: {}", e);
                *current_transcription.lock().await = None;
                return;
            }
        };

        // Convert TranscriptionSegments to Transcript domain models
        let now = chrono::Utc::now().timestamp();
        let transcripts: Vec<Transcript> = result
            .segments
            .into_iter()
            .map(|segment| Transcript {
                id: None,
                meeting_id,
                participant_id: None, // TODO: Link to participant in Phase 5
                timestamp_ms: segment.start_ms,
                text: segment.text,
                confidence: segment.confidence,
                created_at: now,
            })
            .collect();

        log::info!(
            "Transcription complete: {} segments for meeting {}",
            transcripts.len(),
            meeting_id
        );

        // Store transcripts in batch
        if let Err(e) = storage.create_transcripts_batch(&transcripts).await {
            log::error!("Failed to store transcripts: {}", e);
        } else {
            log::info!("Transcripts stored successfully");
        }

        // Clear current transcription
        *current_transcription.lock().await = None;
    });

    Ok(())
}

/// Get transcription status
///
/// Returns the current transcription status and progress.
///
/// # Returns
/// * `Some(meeting_id)` if a transcription is in progress
/// * `None` if no transcription is running
#[tauri::command]
pub async fn get_transcription_status(
    state: State<'_, TranscriptionState>,
) -> Result<Option<i64>, String> {
    let current = state.current_transcription.lock().await;
    Ok(*current)
}

/// Get transcripts for a meeting
///
/// Retrieves all transcript segments for a given meeting.
///
/// # Arguments
/// * `meeting_id` - The ID of the meeting
///
/// # Returns
/// * `Ok(Vec<Transcript>)` - List of transcript segments ordered by timestamp
/// * `Err(String)` if there's an error
#[tauri::command]
pub async fn get_transcripts(
    meeting_id: i64,
    state: State<'_, TranscriptionState>,
) -> Result<Vec<Transcript>, String> {
    state
        .storage
        .get_transcripts(meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcripts: {}", e))
}

/// Check if transcription is available
///
/// Checks if an ASR service is configured and ready to use.
///
/// # Returns
/// * `Ok(true)` if an ASR service is configured
/// * `Ok(false)` if no ASR service is configured
/// * `Err(String)` if there's an error checking configuration
#[tauri::command]
pub async fn is_transcription_available(
    state: State<'_, TranscriptionState>,
) -> Result<bool, String> {
    match get_active_asr_service(&state.storage, &state.keychain).await {
        Ok(service) => Ok(service.is_configured()),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcription_config_default() {
        let config = TranscriptionConfig::default();
        assert!(config.enable_diarization);
        assert_eq!(config.language, Some("en".to_string()));
        assert!(config.num_speakers.is_none());
    }
}
