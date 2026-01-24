/// Export commands for exporting meeting data to various formats
use crate::domain::models::{Insight, InsightType, Meeting, Participant, Transcript};
use crate::ports::storage::StoragePort;
use crate::AppState;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

/// Export format
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Markdown,
    Json,
}

/// Export request from frontend
#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub meeting_id: i64,
    pub format: ExportFormat,
    #[serde(default = "default_true")]
    pub include_insights: bool,
    #[serde(default = "default_true")]
    pub include_participants: bool,
}

fn default_true() -> bool {
    true
}

/// Export response to frontend
#[derive(Debug, Serialize)]
pub struct ExportResponse {
    pub file_path: String,
    pub format: String,
    pub size_bytes: u64,
}

/// Export a meeting to Markdown or JSON format
#[tauri::command]
pub async fn export_meeting(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResponse, String> {
    log::info!(
        "Exporting meeting {} to {:?} format",
        request.meeting_id,
        request.format
    );

    // Fetch meeting data
    let meeting = state
        .storage
        .get_meeting(request.meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch meeting: {}", e))?
        .ok_or_else(|| format!("Meeting with id {} not found", request.meeting_id))?;

    // Fetch transcripts
    let transcripts = state
        .storage
        .get_transcripts(request.meeting_id)
        .await
        .map_err(|e| format!("Failed to fetch transcripts: {}", e))?;

    // Fetch participants if requested
    let participants = if request.include_participants {
        state
            .storage
            .get_participants(request.meeting_id)
            .await
            .map_err(|e| format!("Failed to fetch participants: {}", e))?
    } else {
        Vec::new()
    };

    // Fetch insights if requested
    let insights = if request.include_insights {
        state
            .storage
            .get_insights(request.meeting_id)
            .await
            .map_err(|e| format!("Failed to fetch insights: {}", e))?
    } else {
        Vec::new()
    };

    // Format content based on requested format
    let content = match request.format {
        ExportFormat::Markdown => {
            format_meeting_as_markdown(&meeting, &transcripts, &participants, &insights)
        }
        ExportFormat::Json => {
            format_meeting_as_json(&meeting, &transcripts, &participants, &insights)
                .map_err(|e| format!("Failed to serialize JSON: {}", e))?
        }
    };

    // Determine output path
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    let export_dir = app_data_dir.join("exports");
    std::fs::create_dir_all(&export_dir)
        .map_err(|e| format!("Failed to create exports directory: {}", e))?;

    // Create file name
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let extension = match request.format {
        ExportFormat::Markdown => "md",
        ExportFormat::Json => "json",
    };
    let file_name = format!("meeting_{}_{}.{}", request.meeting_id, timestamp, extension);
    let file_path = export_dir.join(&file_name);

    // Write file
    std::fs::write(&file_path, content)
        .map_err(|e| format!("Failed to write export file: {}", e))?;

    // Get file size
    let size = std::fs::metadata(&file_path)
        .map_err(|e| format!("Failed to get file metadata: {}", e))?
        .len();

    log::info!("Successfully exported meeting to: {}", file_path.display());

    Ok(ExportResponse {
        file_path: file_path.to_string_lossy().to_string(),
        format: format!("{:?}", request.format).to_lowercase(),
        size_bytes: size,
    })
}

/// Format meeting data as Markdown
fn format_meeting_as_markdown(
    meeting: &Meeting,
    transcripts: &[Transcript],
    participants: &[Participant],
    insights: &[Insight],
) -> String {
    let mut output = String::new();

    // Header
    let title = meeting.title.as_deref().unwrap_or("Untitled Meeting");
    output.push_str(&format!("# Meeting: {} - {}\n\n", title, meeting.platform));

    // Meeting metadata
    let start_time = DateTime::from_timestamp(meeting.start_time, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    output.push_str(&format!("**Date:** {}\n", start_time));

    // Duration calculation
    if let Some(end_time) = meeting.end_time {
        let duration_secs = end_time - meeting.start_time;
        let hours = duration_secs / 3600;
        let minutes = (duration_secs % 3600) / 60;
        let seconds = duration_secs % 60;

        if hours > 0 {
            output.push_str(&format!(
                "**Duration:** {}h {}m {}s\n",
                hours, minutes, seconds
            ));
        } else if minutes > 0 {
            output.push_str(&format!("**Duration:** {}m {}s\n", minutes, seconds));
        } else {
            output.push_str(&format!("**Duration:** {}s\n", seconds));
        }
    }

    if let Some(count) = meeting.participant_count {
        output.push_str(&format!("**Participants:** {}\n", count));
    }

    output.push_str("\n---\n\n");

    // Participants section
    if !participants.is_empty() {
        output.push_str("## Participants\n\n");
        for participant in participants {
            if let Some(email) = &participant.email {
                output.push_str(&format!("- {} ({})", participant.name, email));
            } else {
                output.push_str(&format!("- {}", participant.name));
            }

            if let Some(speaker_label) = &participant.speaker_label {
                output.push_str(&format!(" - {}", speaker_label));
            }

            output.push('\n');
        }
        output.push_str("\n---\n\n");
    }

    // Transcript section
    if !transcripts.is_empty() {
        output.push_str("## Transcript\n\n");

        for transcript in transcripts {
            // Format timestamp
            let timestamp_secs = transcript.timestamp_ms / 1000;
            let hours = timestamp_secs / 3600;
            let minutes = (timestamp_secs % 3600) / 60;
            let seconds = timestamp_secs % 60;

            let timestamp = if hours > 0 {
                format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
            } else {
                format!("{:02}:{:02}", minutes, seconds)
            };

            // Get speaker name
            let speaker = transcript
                .participant_name
                .as_deref()
                .or(transcript.speaker_label.as_deref())
                .unwrap_or("Unknown Speaker");

            output.push_str(&format!(
                "**[{}] {}:** {}\n\n",
                timestamp, speaker, transcript.text
            ));
        }

        output.push_str("---\n\n");
    }

    // Insights section
    if !insights.is_empty() {
        output.push_str("## Insights\n\n");

        // Group insights by type
        let summaries: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Summary)
            .collect();
        let action_items: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::ActionItem)
            .collect();
        let key_points: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::KeyPoint)
            .collect();
        let decisions: Vec<_> = insights
            .iter()
            .filter(|i| i.insight_type == InsightType::Decision)
            .collect();

        // Summary
        if !summaries.is_empty() {
            output.push_str("### Summary\n\n");
            for insight in summaries {
                output.push_str(&format!("{}\n\n", insight.content));
            }
        }

        // Action Items
        if !action_items.is_empty() {
            output.push_str("### Action Items\n\n");
            for insight in action_items {
                output.push_str(&format!("- {}\n", insight.content));
            }
            output.push('\n');
        }

        // Key Points
        if !key_points.is_empty() {
            output.push_str("### Key Points\n\n");
            for insight in key_points {
                output.push_str(&format!("- {}\n", insight.content));
            }
            output.push('\n');
        }

        // Decisions
        if !decisions.is_empty() {
            output.push_str("### Decisions\n\n");
            for insight in decisions {
                output.push_str(&format!("- {}\n", insight.content));
            }
            output.push('\n');
        }
    }

    // Footer
    output.push_str("---\n\n");
    output.push_str(&format!(
        "*Exported from Meet Scribe on {}*\n",
        Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    ));

    output
}

/// Format meeting data as JSON
fn format_meeting_as_json(
    meeting: &Meeting,
    transcripts: &[Transcript],
    participants: &[Participant],
    insights: &[Insight],
) -> Result<String, serde_json::Error> {
    // Create export structure
    let export_data = serde_json::json!({
        "meeting": {
            "id": meeting.id,
            "platform": meeting.platform,
            "title": meeting.title,
            "start_time": meeting.start_time,
            "end_time": meeting.end_time,
            "duration_seconds": meeting.end_time.map(|end| end - meeting.start_time),
            "participant_count": meeting.participant_count,
            "audio_file_path": meeting.audio_file_path,
            "created_at": meeting.created_at,
        },
        "participants": participants,
        "transcript": transcripts,
        "insights": {
            "summary": insights.iter().filter(|i| i.insight_type == InsightType::Summary).collect::<Vec<_>>(),
            "action_items": insights.iter().filter(|i| i.insight_type == InsightType::ActionItem).collect::<Vec<_>>(),
            "key_points": insights.iter().filter(|i| i.insight_type == InsightType::KeyPoint).collect::<Vec<_>>(),
            "decisions": insights.iter().filter(|i| i.insight_type == InsightType::Decision).collect::<Vec<_>>(),
        },
        "metadata": {
            "exported_at": Utc::now().timestamp(),
            "exporter": "Meet Scribe v0.0.3",
        }
    });

    serde_json::to_string_pretty(&export_data)
}
