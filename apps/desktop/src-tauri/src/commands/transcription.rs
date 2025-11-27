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
    println!(
        "\n>>> START_TRANSCRIPTION COMMAND CALLED for meeting {}",
        meeting_id
    );
    use std::io::Write;
    let _ = std::io::stdout().flush();

    log::info!("Starting transcription for meeting {}", meeting_id);

    // Check if a transcription is already in progress
    let mut current = state.current_transcription.lock().await;
    if current.is_some() {
        println!(
            "!!! Transcription already in progress for meeting {:?}",
            *current
        );
        log::warn!(
            "Transcription already in progress for meeting {:?}",
            *current
        );
        return Err("A transcription is already in progress".to_string());
    }

    // Mark this meeting as being transcribed
    *current = Some(meeting_id);
    drop(current); // Release lock
    log::info!("Marked meeting {} as transcribing", meeting_id);

    // Get the meeting details
    log::info!("Fetching meeting {} from database", meeting_id);
    let meeting = state
        .storage
        .get_meeting(meeting_id)
        .await
        .map_err(|e| {
            log::error!("Failed to get meeting {}: {}", meeting_id, e);
            format!("Failed to get meeting: {}", e)
        })?
        .ok_or_else(|| {
            log::error!("Meeting {} not found in database", meeting_id);
            format!("Meeting {} not found", meeting_id)
        })?;

    log::info!(
        "Meeting {} found: platform={}, audio_file_path={:?}",
        meeting_id,
        meeting.platform,
        meeting.audio_file_path
    );

    // Check if audio file exists
    let audio_file_path = meeting
        .audio_file_path
        .ok_or_else(|| {
            log::error!("Meeting {} has no audio file path", meeting_id);
            log::error!("This usually means the audio file hasn't been saved yet, or audio recording failed");
            log::error!("Meeting details: platform={}, start_time={}, end_time={:?}",
                meeting.platform, meeting.start_time, meeting.end_time);
            "Meeting has no audio file. The audio may still be processing, or recording may have failed. Please wait a moment and try again.".to_string()
        })?;

    log::info!(
        "Audio file path for meeting {}: {}",
        meeting_id,
        audio_file_path
    );

    // Get the active ASR service
    println!(">>> Getting active ASR service");
    log::info!("Getting active ASR service");
    let asr_service = get_active_asr_service(&state.storage, &state.keychain)
        .await
        .map_err(|e| {
            println!("!!! Failed to get ASR service: {}", e);
            log::error!("Failed to get ASR service: {}", e);
            format!("Failed to get ASR service: {}", e)
        })?;

    println!(">>> Active ASR service: {}", asr_service.provider_name());
    log::info!("Active ASR service: {}", asr_service.provider_name());

    // Use provided config or defaults
    let transcription_config = config.unwrap_or_default();

    // Clone state for the background task
    let storage = Arc::clone(&state.storage);
    let current_transcription = Arc::clone(&state.current_transcription);

    println!(">>> About to spawn background transcription task");
    let _ = std::io::stdout().flush();

    // Spawn transcription task in background
    tokio::spawn(async move {
        // Print to both logger and stdout to ensure visibility
        println!("=== TRANSCRIPTION BACKGROUND TASK STARTED ===");
        println!("Transcribing audio file: {}", audio_file_path);
        log::info!("=== TRANSCRIPTION BACKGROUND TASK STARTED ===");
        log::info!("Transcribing audio file: {}", audio_file_path);
        log::info!(
            "Transcription config: diarization={}, language={:?}",
            transcription_config.enable_diarization,
            transcription_config.language
        );

        // Force flush logs to console
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();

        // Check if audio file exists
        if !std::path::Path::new(&audio_file_path).exists() {
            log::error!("Audio file not found: {}", audio_file_path);
            *current_transcription.lock().await = None;
            return;
        }

        // Perform transcription
        let result = match asr_service
            .transcribe_file(&audio_file_path, &transcription_config)
            .await
        {
            Ok(result) => {
                log::info!("Transcription API call successful");
                result
            }
            Err(e) => {
                println!("!!! TRANSCRIPTION FAILED: {}", e);
                println!("!!! Error details: {:?}", e);
                log::error!("Transcription failed: {}", e);
                log::error!("Error details: {:?}", e);
                let _ = std::io::stdout().flush();
                let _ = std::io::stderr().flush();
                *current_transcription.lock().await = None;
                return;
            }
        };

        // Convert TranscriptionSegments to Transcript domain models
        println!(
            ">>> Converting {} segments to Transcript models",
            result.segments.len()
        );
        let now = chrono::Utc::now().timestamp();
        let transcripts: Vec<Transcript> = result
            .segments
            .into_iter()
            .map(|segment| Transcript {
                id: None,
                meeting_id,
                participant_id: None, // TODO: Link to participant in Phase 5
                speaker_label: segment.speaker_label, // Diarization speaker label
                timestamp_ms: segment.start_ms,
                text: segment.text,
                confidence: segment.confidence,
                created_at: now,
            })
            .collect();

        println!(">>> Converted {} transcript segments", transcripts.len());
        log::info!(
            "Transcription complete: {} segments for meeting {}",
            transcripts.len(),
            meeting_id
        );

        // Store transcripts in batch
        println!(">>> Storing {} transcripts in database", transcripts.len());
        if let Err(e) = storage.create_transcripts_batch(&transcripts).await {
            println!("!!! Failed to store transcripts: {}", e);
            log::error!("Failed to store transcripts: {}", e);
        } else {
            println!(">>> Transcripts stored successfully!");
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

/// Delete all transcripts for a meeting
///
/// This allows regenerating transcripts by first deleting existing ones.
#[tauri::command]
pub async fn delete_transcripts(
    meeting_id: i64,
    state: State<'_, TranscriptionState>,
) -> Result<(), String> {
    use crate::ports::storage::StoragePort;

    log::info!("Deleting transcripts for meeting {}", meeting_id);
    state
        .storage
        .delete_transcripts(meeting_id)
        .await
        .map_err(|e| format!("Failed to delete transcripts: {}", e))
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
