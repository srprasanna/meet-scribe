/// Participant management commands
use crate::domain::models::Participant;
use crate::ports::storage::StoragePort;
use crate::AppState;
use serde::{Deserialize, Serialize};
use tauri::State;

/// Speaker summary with sample transcripts
#[derive(Debug, Serialize, Deserialize)]
pub struct SpeakerSummary {
    pub speaker_label: String,
    pub transcript_count: usize,
    pub sample_transcripts: Vec<String>, // 2-3 sample lines
    pub participant: Option<ParticipantInfo>,
}

/// Participant information
#[derive(Debug, Serialize, Deserialize)]
pub struct ParticipantInfo {
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
}

/// Request to link a speaker to a participant
#[derive(Debug, Deserialize)]
pub struct LinkSpeakerRequest {
    pub meeting_id: i64,
    pub speaker_label: String,
    pub participant_name: String,
    pub participant_email: Option<String>,
}

/// Get summary of all speakers in a meeting with sample transcripts
#[tauri::command]
pub async fn get_speaker_summary(
    meeting_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<SpeakerSummary>, String> {
    log::info!("Getting speaker summary for meeting {}", meeting_id);

    // Get all transcripts for the meeting
    let transcripts = state
        .storage
        .get_transcripts(meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcripts: {}", e))?;

    // Get all participants for the meeting
    let participants = state
        .storage
        .get_participants(meeting_id)
        .await
        .map_err(|e| format!("Failed to get participants: {}", e))?;

    // Group transcripts by speaker_label
    let mut speaker_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for transcript in transcripts {
        if let Some(speaker_label) = &transcript.speaker_label {
            speaker_map
                .entry(speaker_label.clone())
                .or_insert_with(Vec::new)
                .push(transcript.text.clone());
        }
    }

    // Create speaker summaries
    let mut summaries = Vec::new();
    for (speaker_label, texts) in speaker_map {
        // Get up to 3 sample transcripts
        let sample_transcripts: Vec<String> = texts.iter().take(3).cloned().collect();

        // Find if this speaker is already linked to a participant
        let participant = participants
            .iter()
            .find(|p| p.speaker_label.as_ref() == Some(&speaker_label))
            .map(|p| ParticipantInfo {
                id: p.id.unwrap_or(0),
                name: p.name.clone(),
                email: p.email.clone(),
            });

        summaries.push(SpeakerSummary {
            speaker_label: speaker_label.clone(),
            transcript_count: texts.len(),
            sample_transcripts,
            participant,
        });
    }

    // Sort by speaker label for consistent ordering
    summaries.sort_by(|a, b| a.speaker_label.cmp(&b.speaker_label));

    log::info!(
        "Found {} unique speakers for meeting {}",
        summaries.len(),
        meeting_id
    );
    Ok(summaries)
}

/// Link a speaker label to a participant (create or update)
#[tauri::command]
pub async fn link_speaker_to_participant(
    request: LinkSpeakerRequest,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    log::info!(
        "Linking speaker '{}' to participant '{}' for meeting {}",
        request.speaker_label,
        request.participant_name,
        request.meeting_id
    );

    // Check if participant already exists with this speaker_label
    let participants = state
        .storage
        .get_participants(request.meeting_id)
        .await
        .map_err(|e| format!("Failed to get participants: {}", e))?;

    let existing_participant = participants
        .iter()
        .find(|p| p.speaker_label.as_ref() == Some(&request.speaker_label));

    let participant_id = if let Some(existing) = existing_participant {
        // Update existing participant
        let mut updated = existing.clone();
        updated.name = request.participant_name.clone();
        updated.email = request.participant_email.clone();

        state
            .storage
            .update_participant(&updated)
            .await
            .map_err(|e| format!("Failed to update participant: {}", e))?;

        existing.id.unwrap_or(0)
    } else {
        // Create new participant
        let participant = Participant {
            id: None,
            meeting_id: request.meeting_id,
            name: request.participant_name.clone(),
            email: request.participant_email.clone(),
            speaker_label: Some(request.speaker_label.clone()),
        };

        state
            .storage
            .create_participant(&participant)
            .await
            .map_err(|e| format!("Failed to create participant: {}", e))?
    };

    // Batch update all transcripts with this speaker_label to link to the participant
    // This is much more efficient than updating one by one, especially for large meetings
    let updated_count = state
        .storage
        .update_transcripts_by_speaker_label(
            request.meeting_id,
            &request.speaker_label,
            participant_id,
        )
        .await
        .map_err(|e| format!("Failed to update transcripts: {}", e))?;

    log::info!(
        "Successfully linked {} transcripts to participant {} (ID: {}) for meeting {}",
        updated_count,
        request.participant_name,
        participant_id,
        request.meeting_id
    );

    Ok(participant_id)
}

/// Unlink a speaker from a participant (remove mapping)
#[tauri::command]
pub async fn unlink_speaker(
    meeting_id: i64,
    speaker_label: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!(
        "Unlinking speaker '{}' for meeting {}",
        speaker_label,
        meeting_id
    );

    // Get all transcripts and clear participant_id for this speaker
    let transcripts = state
        .storage
        .get_transcripts(meeting_id)
        .await
        .map_err(|e| format!("Failed to get transcripts: {}", e))?;

    let mut updated_count = 0;
    for mut transcript in transcripts {
        if transcript.speaker_label.as_ref() == Some(&speaker_label) {
            transcript.participant_id = None;
            state
                .storage
                .update_transcript(&transcript)
                .await
                .map_err(|e| format!("Failed to update transcript: {}", e))?;
            updated_count += 1;
        }
    }

    // Find and delete the participant with this speaker_label
    let participants = state
        .storage
        .get_participants(meeting_id)
        .await
        .map_err(|e| format!("Failed to get participants: {}", e))?;

    if let Some(participant) = participants
        .iter()
        .find(|p| p.speaker_label.as_ref() == Some(&speaker_label))
    {
        if let Some(id) = participant.id {
            state
                .storage
                .delete_participant(id)
                .await
                .map_err(|e| format!("Failed to delete participant: {}", e))?;
        }
    }

    log::info!(
        "Successfully unlinked {} transcripts from speaker '{}'",
        updated_count,
        speaker_label
    );

    Ok(())
}

/// Delete all participants for a meeting
/// This is useful when regenerating transcripts to start fresh
#[tauri::command]
pub async fn delete_meeting_participants(
    meeting_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    log::info!("Deleting all participants for meeting {}", meeting_id);

    // Get all participants for this meeting
    let participants = state
        .storage
        .get_participants(meeting_id)
        .await
        .map_err(|e| format!("Failed to get participants: {}", e))?;

    let participant_count = participants.len();

    // Delete each participant
    for participant in participants {
        if let Some(id) = participant.id {
            state
                .storage
                .delete_participant(id)
                .await
                .map_err(|e| format!("Failed to delete participant {}: {}", id, e))?;
        }
    }

    log::info!(
        "Successfully deleted {} participants for meeting {}",
        participant_count,
        meeting_id
    );

    Ok(())
}
