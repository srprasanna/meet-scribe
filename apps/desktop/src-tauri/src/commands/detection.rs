//! Tauri commands for participant detection
//!
//! These commands expose the participant detection functionality to the frontend.

use crate::adapters::detection::create_detector;
use crate::domain::models::{Participant, Platform};
use crate::ports::detection::{
    DetectedMeeting, DetectedParticipant, DetectionConfig, DetectionResult, ParticipantDetectorPort,
};
use crate::ports::storage::StoragePort;
use crate::AppState;
use serde::{Deserialize, Serialize};

/// Response for list_active_meetings command
#[derive(Debug, Serialize)]
pub struct ActiveMeetingsResponse {
    pub meetings: Vec<DetectedMeeting>,
    pub detector_available: bool,
    pub detection_method: String,
}

/// Request for detect_participants command
#[derive(Debug, Deserialize)]
pub struct DetectParticipantsRequest {
    pub meeting: DetectedMeeting,
    #[serde(default)]
    pub config: Option<DetectionConfig>,
}

/// Request for auto_detect_participants command
#[derive(Debug, Deserialize)]
pub struct AutoDetectRequest {
    #[serde(default)]
    pub target_platform: Option<String>,
    #[serde(default = "default_include_self")]
    pub include_self: bool,
}

fn default_include_self() -> bool {
    true
}

/// Response for auto_detect_participants command
#[derive(Debug, Serialize)]
pub struct AutoDetectResponse {
    pub result: Option<DetectionResult>,
    pub detector_available: bool,
}

/// Request for import_detected_participants command
#[derive(Debug, Deserialize)]
pub struct ImportParticipantsRequest {
    pub meeting_id: i64,
    pub participants: Vec<DetectedParticipant>,
}

/// Lists all currently active meeting windows
///
/// Scans for running instances of Teams, Zoom, and Google Meet.
#[tauri::command]
pub async fn list_active_meetings() -> Result<ActiveMeetingsResponse, String> {
    let detector = create_detector();

    let meetings = detector
        .list_active_meetings()
        .await
        .map_err(|e| e.to_string())?;

    Ok(ActiveMeetingsResponse {
        meetings,
        detector_available: detector.is_available(),
        detection_method: detector.detection_method().to_string(),
    })
}

/// Detects participants from a specific meeting window
///
/// Uses accessibility APIs to extract participant names from the meeting UI.
#[tauri::command]
pub async fn detect_participants(
    request: DetectParticipantsRequest,
) -> Result<DetectionResult, String> {
    let detector = create_detector();
    let config = request.config.unwrap_or_default();

    detector
        .detect_participants(&request.meeting, &config)
        .await
        .map_err(|e| e.to_string())
}

/// Auto-detects participants from any running meeting
///
/// Finds the first active meeting and detects participants.
/// Optionally filter by platform.
#[tauri::command]
pub async fn auto_detect_participants(
    request: AutoDetectRequest,
) -> Result<AutoDetectResponse, String> {
    let detector = create_detector();

    let target_platform = request.target_platform.and_then(|p| match p.as_str() {
        "teams" => Some(Platform::Teams),
        "zoom" => Some(Platform::Zoom),
        "meet" => Some(Platform::Meet),
        _ => None,
    });

    let config = DetectionConfig {
        target_platform,
        include_self: request.include_self,
        ..Default::default()
    };

    let result = detector
        .auto_detect(&config)
        .await
        .map_err(|e| e.to_string())?;

    Ok(AutoDetectResponse {
        result,
        detector_available: detector.is_available(),
    })
}

/// Checks if participant detection is available on this platform
#[tauri::command]
pub async fn is_detection_available() -> Result<bool, String> {
    let detector = create_detector();
    Ok(detector.is_available())
}

/// Gets information about the detection method being used
#[tauri::command]
pub async fn get_detection_info() -> Result<serde_json::Value, String> {
    let detector = create_detector();

    Ok(serde_json::json!({
        "method": detector.detection_method().to_string(),
        "available": detector.is_available(),
        "platform": std::env::consts::OS,
    }))
}

/// Imports detected participants into a meeting record
///
/// Creates participant records in the database from detected participants.
/// This allows linking detected participants with speaker labels from transcription.
#[tauri::command]
pub async fn import_detected_participants(
    state: tauri::State<'_, AppState>,
    request: ImportParticipantsRequest,
) -> Result<Vec<i64>, String> {
    let storage = &state.storage;
    let mut created_ids = Vec::new();

    for detected in request.participants {
        let participant = Participant {
            id: None,
            meeting_id: request.meeting_id,
            name: detected.name,
            email: None,
            speaker_label: None, // Will be linked later during transcription
        };

        let id = storage
            .create_participant(&participant)
            .await
            .map_err(|e| e.to_string())?;

        created_ids.push(id);
    }

    Ok(created_ids)
}

/// Detects and imports participants in one step
///
/// Convenience command that auto-detects participants and imports them
/// directly into the specified meeting.
#[tauri::command]
pub async fn detect_and_import_participants(
    state: tauri::State<'_, AppState>,
    meeting_id: i64,
    target_platform: Option<String>,
) -> Result<DetectionResult, String> {
    // First, auto-detect participants
    let detector = create_detector();

    let target = target_platform.and_then(|p| match p.as_str() {
        "teams" => Some(Platform::Teams),
        "zoom" => Some(Platform::Zoom),
        "meet" => Some(Platform::Meet),
        _ => None,
    });

    let config = DetectionConfig {
        target_platform: target,
        include_self: true,
        ..Default::default()
    };

    let result = detector
        .auto_detect(&config)
        .await
        .map_err(|e| e.to_string())?;

    let Some(detection_result) = result else {
        return Err("No active meeting found".to_string());
    };

    // Import detected participants
    let storage = &state.storage;
    for detected in &detection_result.participants {
        let participant = Participant {
            id: None,
            meeting_id,
            name: detected.name.clone(),
            email: None,
            speaker_label: None,
        };

        let _ = storage.create_participant(&participant).await;
    }

    Ok(detection_result)
}
