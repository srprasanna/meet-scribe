//! AssemblyAI transcription service adapter
//!
//! Implements the TranscriptionServicePort for AssemblyAI's API.
//! API flow:
//! 1. Upload audio file to AssemblyAI
//! 2. Submit transcription request with diarization
//! 3. Poll for completion
//! 4. Parse results with speaker labels

use crate::error::{AppError, Result};
use crate::ports::transcription::{
    StreamingSession, StreamingTranscriptionCallback, TranscriptionConfig, TranscriptionResult,
    TranscriptionSegment, TranscriptionServicePort,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const ASSEMBLYAI_API_BASE: &str = "https://api.assemblyai.com/v2";
const POLL_INTERVAL_MS: u64 = 3000; // Poll every 3 seconds
const MAX_POLL_ATTEMPTS: u32 = 200; // Max 10 minutes (200 * 3s)

/// AssemblyAI service implementation
pub struct AssemblyAIService {
    client: Client,
    api_key: String,
}

impl AssemblyAIService {
    /// Create a new AssemblyAI service with the given API key
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    /// Upload audio file to AssemblyAI and get the upload URL
    async fn upload_file(&self, audio_path: &str) -> Result<String> {
        log::info!("Uploading audio file to AssemblyAI: {}", audio_path);

        // Read the audio file
        let mut file = File::open(audio_path)
            .await
            .map_err(|e| AppError::Transcription(format!("Failed to open audio file: {}", e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .map_err(|e| AppError::Transcription(format!("Failed to read audio file: {}", e)))?;

        // Upload to AssemblyAI
        let response = self
            .client
            .post(format!("{}/upload", ASSEMBLYAI_API_BASE))
            .header("authorization", &self.api_key)
            .header("content-type", "application/octet-stream")
            .body(buffer)
            .send()
            .await
            .map_err(|e| AppError::Transcription(format!("Upload request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Transcription(format!(
                "Upload failed: {}",
                error_text
            )));
        }

        let upload_response: UploadResponse = response.json().await.map_err(|e| {
            AppError::Transcription(format!("Failed to parse upload response: {}", e))
        })?;

        log::info!("File uploaded successfully: {}", upload_response.upload_url);
        Ok(upload_response.upload_url)
    }

    /// Submit transcription request with diarization enabled
    async fn submit_transcription(
        &self,
        audio_url: &str,
        config: &TranscriptionConfig,
    ) -> Result<String> {
        log::info!("Submitting transcription request to AssemblyAI");

        let request_body = TranscriptionRequest {
            audio_url: audio_url.to_string(),
            speaker_labels: config.enable_diarization,
            speakers_expected: config.num_speakers,
            language_code: config.language.clone(),
            speech_model: config.model.clone(),
        };

        let response = self
            .client
            .post(format!("{}/transcript", ASSEMBLYAI_API_BASE))
            .header("authorization", &self.api_key)
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await
            .map_err(|e| AppError::Transcription(format!("Submit request failed: {}", e)))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Transcription(format!(
                "Submit failed: {}",
                error_text
            )));
        }

        let submit_response: TranscriptResponse = response.json().await.map_err(|e| {
            AppError::Transcription(format!("Failed to parse submit response: {}", e))
        })?;

        log::info!("Transcription submitted with ID: {}", submit_response.id);
        Ok(submit_response.id)
    }

    /// Poll for transcription completion
    async fn poll_transcription(&self, transcript_id: &str) -> Result<TranscriptionResult> {
        log::info!("Polling for transcription completion: {}", transcript_id);

        for attempt in 1..=MAX_POLL_ATTEMPTS {
            // Wait before polling
            tokio::time::sleep(Duration::from_millis(POLL_INTERVAL_MS)).await;

            let response = self
                .client
                .get(format!(
                    "{}/transcript/{}",
                    ASSEMBLYAI_API_BASE, transcript_id
                ))
                .header("authorization", &self.api_key)
                .send()
                .await
                .map_err(|e| AppError::Transcription(format!("Poll request failed: {}", e)))?;

            if !response.status().is_success() {
                let error_text = response.text().await.unwrap_or_default();
                return Err(AppError::Transcription(format!(
                    "Poll failed: {}",
                    error_text
                )));
            }

            let transcript_response: TranscriptResponse = response.json().await.map_err(|e| {
                AppError::Transcription(format!("Failed to parse poll response: {}", e))
            })?;

            match transcript_response.status.as_str() {
                "completed" => {
                    log::info!("Transcription completed successfully");
                    return self.parse_transcript_response(transcript_response);
                }
                "error" => {
                    return Err(AppError::Transcription(format!(
                        "Transcription failed: {}",
                        transcript_response.error.unwrap_or_default()
                    )));
                }
                "queued" | "processing" => {
                    log::debug!(
                        "Transcription status: {} (attempt {}/{})",
                        transcript_response.status,
                        attempt,
                        MAX_POLL_ATTEMPTS
                    );
                    continue;
                }
                status => {
                    log::warn!("Unknown transcription status: {}", status);
                    continue;
                }
            }
        }

        Err(AppError::Transcription(
            "Transcription timeout: exceeded maximum polling attempts".to_string(),
        ))
    }

    /// Parse AssemblyAI response into our TranscriptionResult format
    fn parse_transcript_response(
        &self,
        response: TranscriptResponse,
    ) -> Result<TranscriptionResult> {
        let text = response.text.unwrap_or_default();
        let confidence = response.confidence;

        // Parse utterances (speaker-labeled segments)
        let segments = if let Some(utterances) = response.utterances {
            utterances
                .into_iter()
                .map(|utt| TranscriptionSegment {
                    text: utt.text,
                    start_ms: utt.start,
                    end_ms: utt.end,
                    speaker_label: Some(format!("Speaker {}", utt.speaker)),
                    confidence: Some(utt.confidence),
                })
                .collect()
        } else {
            // Fallback: no diarization, single segment
            vec![TranscriptionSegment {
                text: text.clone(),
                start_ms: 0,
                end_ms: response.audio_duration.unwrap_or(0),
                speaker_label: None,
                confidence,
            }]
        };

        Ok(TranscriptionResult {
            text,
            segments,
            confidence,
        })
    }
}

#[async_trait]
impl TranscriptionServicePort for AssemblyAIService {
    async fn transcribe_file(
        &self,
        audio_path: &str,
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        log::info!("Starting AssemblyAI transcription for: {}", audio_path);

        // Step 1: Upload file
        let audio_url = self.upload_file(audio_path).await?;

        // Step 2: Submit transcription
        let transcript_id = self.submit_transcription(&audio_url, config).await?;

        // Step 3: Poll for completion
        let result = self.poll_transcription(&transcript_id).await?;

        log::info!(
            "AssemblyAI transcription complete: {} segments, {} chars",
            result.segments.len(),
            result.text.len()
        );

        Ok(result)
    }

    async fn transcribe_bytes(
        &self,
        audio_data: &[u8],
        format: &str,
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        log::info!(
            "Transcribing audio bytes via AssemblyAI (format: {})",
            format
        );

        // AssemblyAI requires file upload, so write bytes to a temporary file
        let temp_dir = std::env::temp_dir();
        let temp_file_path = temp_dir.join(format!(
            "assemblyai_temp_{}.{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            format
        ));

        // Write audio data to temporary file
        tokio::fs::write(&temp_file_path, audio_data)
            .await
            .map_err(|e| {
                AppError::Transcription(format!("Failed to write temporary file: {}", e))
            })?;

        log::debug!("Created temporary file: {}", temp_file_path.display());

        // Transcribe using the temporary file
        let result = self
            .transcribe_file(
                temp_file_path.to_str().ok_or_else(|| {
                    AppError::Transcription("Invalid temporary file path".to_string())
                })?,
                config,
            )
            .await;

        // Clean up temporary file
        if let Err(e) = tokio::fs::remove_file(&temp_file_path).await {
            log::warn!(
                "Failed to remove temporary file {}: {}",
                temp_file_path.display(),
                e
            );
        } else {
            log::debug!("Removed temporary file: {}", temp_file_path.display());
        }

        result
    }

    async fn start_streaming(
        &self,
        _config: &TranscriptionConfig,
        _callback: Box<dyn StreamingTranscriptionCallback>,
    ) -> Result<Box<dyn StreamingSession>> {
        // TODO: Implement AssemblyAI streaming
        // AssemblyAI supports streaming via WebSocket at wss://api.assemblyai.com/v2/realtime/ws
        // For now, return an error indicating streaming is not yet implemented
        Err(AppError::Transcription(
            "AssemblyAI streaming not yet implemented. Use Deepgram for streaming transcription."
                .to_string(),
        ))
    }

    fn provider_name(&self) -> &str {
        "AssemblyAI"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn supports_streaming(&self) -> bool {
        false // TODO: Implement AssemblyAI streaming support
    }
}

// ===== API Request/Response Types =====

#[derive(Debug, Serialize)]
struct TranscriptionRequest {
    audio_url: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    speaker_labels: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    speakers_expected: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    speech_model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadResponse {
    upload_url: String,
}

#[derive(Debug, Deserialize)]
struct TranscriptResponse {
    id: String,
    status: String,
    text: Option<String>,
    confidence: Option<f32>,
    audio_duration: Option<i64>,
    utterances: Option<Vec<Utterance>>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Utterance {
    text: String,
    start: i64,
    end: i64,
    confidence: f32,
    speaker: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assemblyai_service_creation() {
        let service = AssemblyAIService::new("test_api_key".to_string());
        assert_eq!(service.provider_name(), "AssemblyAI");
        assert!(service.is_configured());
    }

    #[test]
    fn test_assemblyai_service_not_configured() {
        let service = AssemblyAIService::new("".to_string());
        assert!(!service.is_configured());
    }
}
