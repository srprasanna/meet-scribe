//! Streaming transcription commands
//!
//! Provides real-time transcription capabilities during active meetings.

use crate::adapters::services::asr;
use crate::domain::models::Transcript;
use crate::ports::storage::StoragePort;
use crate::ports::transcription::{
    StreamingSession, StreamingTranscriptionCallback, TranscriptionConfig, TranscriptionSegment,
};
use crate::AppState;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::SystemTime;
use tauri::Emitter;
use tokio::sync::Mutex;

/// State for managing streaming transcription sessions
pub struct StreamingTranscriptionState {
    /// Currently active streaming session
    pub active_session: Arc<Mutex<Option<Box<dyn StreamingSession>>>>,

    /// Meeting ID being transcribed
    pub meeting_id: Arc<Mutex<Option<i64>>>,
}

impl StreamingTranscriptionState {
    pub fn new() -> Self {
        Self {
            active_session: Arc::new(Mutex::new(None)),
            meeting_id: Arc::new(Mutex::new(None)),
        }
    }
}

/// Tauri event callback for streaming transcription
/// This sends transcript segments to the frontend via Tauri events
struct TauriStreamingCallback {
    app_handle: tauri::AppHandle,
    meeting_id: i64,
    storage: Arc<dyn StoragePort>,
}

#[async_trait]
impl StreamingTranscriptionCallback for TauriStreamingCallback {
    async fn on_transcript(&self, segment: TranscriptionSegment) {
        log::info!(
            "Received final transcript: {} chars, speaker: {:?}",
            segment.text.len(),
            segment.speaker_label
        );

        // Store transcript in database
        let transcript = Transcript {
            id: None,
            meeting_id: self.meeting_id,
            participant_id: None, // Will be linked later when user maps speakers
            participant_name: None, // Will be populated when speaker is linked to participant
            speaker_label: segment.speaker_label.clone(),
            timestamp_ms: segment.start_ms,
            text: segment.text.clone(),
            confidence: segment.confidence,
            created_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        match self.storage.create_transcript(&transcript).await {
            Ok(id) => {
                log::debug!("Stored transcript with ID: {}", id);

                // Emit event to frontend with the stored transcript
                let mut stored_transcript = transcript;
                stored_transcript.id = Some(id);

                let _ = self
                    .app_handle
                    .emit_to("main", "streaming-transcript", stored_transcript);
            }
            Err(e) => {
                log::error!("Failed to store transcript: {}", e);
            }
        }
    }

    async fn on_interim_transcript(&self, segment: TranscriptionSegment) {
        log::debug!("Received interim transcript: {} chars", segment.text.len());

        // Emit interim transcripts to frontend (not stored in DB)
        let _ = self
            .app_handle
            .emit_to("main", "streaming-transcript-interim", segment);
    }

    async fn on_error(&self, error: String) {
        log::error!("Streaming transcription error: {}", error);

        let _ = self
            .app_handle
            .emit_to("main", "streaming-transcription-error", error);
    }

    async fn on_close(&self) {
        log::info!("Streaming transcription closed");

        let _ = self
            .app_handle
            .emit_to("main", "streaming-transcription-closed", ());
    }
}

/// Start streaming transcription for an active meeting
#[tauri::command]
pub async fn start_streaming_transcription(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    streaming_state: tauri::State<'_, StreamingTranscriptionState>,
    meeting_id: i64,
    config: Option<TranscriptionConfig>,
) -> Result<(), String> {
    log::info!(
        "Starting streaming transcription for meeting {}",
        meeting_id
    );

    // Check if there's already an active session
    let mut active_session = streaming_state.active_session.lock().await;
    if active_session.is_some() {
        return Err("Streaming transcription already active".to_string());
    }

    // Load transcription config if not provided
    let transcription_config = if let Some(cfg) = config {
        cfg
    } else {
        let mut default_config = TranscriptionConfig::default();

        // Load from active service configuration
        match state.storage.get_active_service_config("asr").await {
            Ok(Some(service_config)) => {
                if let Some(settings_str) = service_config.settings {
                    match serde_json::from_str::<serde_json::Value>(&settings_str) {
                        Ok(settings) => {
                            if let Some(model) = settings.get("model").and_then(|m| m.as_str()) {
                                default_config.model = Some(model.to_string());
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse service config settings: {}", e);
                        }
                    }
                }
            }
            Ok(None) => {
                return Err("No active ASR service configured".to_string());
            }
            Err(e) => {
                return Err(format!("Failed to get ASR service config: {}", e));
            }
        }

        default_config
    };

    // Get the active ASR service
    let asr_service = asr::get_active_asr_service(&state.storage, &state.keychain)
        .await
        .map_err(|e| e.to_string())?;

    // Check if streaming is supported
    if !asr_service.supports_streaming() {
        return Err(format!(
            "{} does not support streaming transcription",
            asr_service.provider_name()
        ));
    }

    // Create callback that emits Tauri events
    let callback = Box::new(TauriStreamingCallback {
        app_handle: app.clone(),
        meeting_id,
        storage: Arc::clone(&state.storage) as Arc<dyn StoragePort>,
    });

    // Start streaming session
    let session = asr_service
        .start_streaming(&transcription_config, callback)
        .await
        .map_err(|e| e.to_string())?;

    // Store the session
    *active_session = Some(session);
    *streaming_state.meeting_id.lock().await = Some(meeting_id);

    log::info!("Streaming transcription started for meeting {}", meeting_id);

    Ok(())
}

/// Stop streaming transcription
#[tauri::command]
pub async fn stop_streaming_transcription(
    streaming_state: tauri::State<'_, StreamingTranscriptionState>,
) -> Result<(), String> {
    log::info!("Stopping streaming transcription");

    let mut active_session = streaming_state.active_session.lock().await;

    if let Some(mut session) = active_session.take() {
        session
            .flush()
            .await
            .map_err(|e| format!("Failed to flush session: {}", e))?;

        session
            .close()
            .await
            .map_err(|e| format!("Failed to close session: {}", e))?;

        *streaming_state.meeting_id.lock().await = None;

        log::info!("Streaming transcription stopped");
        Ok(())
    } else {
        Err("No active streaming transcription session".to_string())
    }
}

/// Send audio chunk to the streaming transcription session
#[tauri::command]
pub async fn send_audio_chunk(
    streaming_state: tauri::State<'_, StreamingTranscriptionState>,
    audio_chunk: Vec<u8>,
) -> Result<(), String> {
    let mut active_session = streaming_state.active_session.lock().await;

    if let Some(session) = active_session.as_mut() {
        session
            .send_audio(&audio_chunk)
            .await
            .map_err(|e| format!("Failed to send audio chunk: {}", e))?;

        Ok(())
    } else {
        Err("No active streaming transcription session".to_string())
    }
}

/// Get streaming transcription status
#[tauri::command]
pub async fn get_streaming_transcription_status(
    streaming_state: tauri::State<'_, StreamingTranscriptionState>,
) -> Result<StreamingTranscriptionStatus, String> {
    let active_session = streaming_state.active_session.lock().await;
    let meeting_id = streaming_state.meeting_id.lock().await;

    Ok(StreamingTranscriptionStatus {
        is_active: active_session.is_some(),
        meeting_id: *meeting_id,
    })
}

/// Response for streaming transcription status
#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingTranscriptionStatus {
    pub is_active: bool,
    pub meeting_id: Option<i64>,
}
