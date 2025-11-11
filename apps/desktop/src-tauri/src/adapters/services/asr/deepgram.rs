//! Deepgram transcription service adapter
//!
//! Implements the TranscriptionServicePort for Deepgram's API.
//! Simpler API than AssemblyAI - single request with file streaming.

use crate::error::{AppError, Result};
use crate::ports::transcription::{
    TranscriptionConfig, TranscriptionResult, TranscriptionSegment, TranscriptionServicePort,
};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

const DEEPGRAM_API_BASE: &str = "https://api.deepgram.com/v1";

/// Deepgram service implementation
pub struct DeepgramService {
    client: Client,
    api_key: String,
}

impl DeepgramService {
    /// Create a new Deepgram service with the given API key
    pub fn new(api_key: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300)) // Longer timeout for large files
            .build()
            .expect("Failed to create HTTP client");

        Self { client, api_key }
    }

    /// Transcribe audio file with diarization
    async fn transcribe_with_diarization(
        &self,
        audio_path: &str,
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        log::info!("Transcribing with Deepgram: {}", audio_path);

        // Read the audio file
        let mut file = File::open(audio_path)
            .await
            .map_err(|e| AppError::Transcription(format!("Failed to open audio file: {}", e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .await
            .map_err(|e| AppError::Transcription(format!("Failed to read audio file: {}", e)))?;

        // Build query parameters
        let mut url = format!("{}/listen", DEEPGRAM_API_BASE);
        let mut params = vec![
            ("punctuate", "true"),
            (
                "diarize",
                if config.enable_diarization {
                    "true"
                } else {
                    "false"
                },
            ),
            ("utterances", "true"),
        ];

        if let Some(lang) = &config.language {
            params.push(("language", lang));
        }

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        url = format!("{}?{}", url, query_string);

        // Send request
        let response = self
            .client
            .post(&url)
            .header("authorization", format!("Token {}", self.api_key))
            .header("content-type", "audio/wav")
            .body(buffer)
            .send()
            .await
            .map_err(|e| AppError::Transcription(format!("Deepgram request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Transcription(format!(
                "Deepgram API error ({}): {}",
                status, error_text
            )));
        }

        let deepgram_response: DeepgramResponse = response.json().await.map_err(|e| {
            AppError::Transcription(format!("Failed to parse Deepgram response: {}", e))
        })?;

        self.parse_deepgram_response(deepgram_response)
    }

    /// Parse Deepgram response into our TranscriptionResult format
    fn parse_deepgram_response(&self, response: DeepgramResponse) -> Result<TranscriptionResult> {
        let channel = response.results.channels.get(0).ok_or_else(|| {
            AppError::Transcription("No channels in Deepgram response".to_string())
        })?;

        let alternative = channel.alternatives.get(0).ok_or_else(|| {
            AppError::Transcription("No alternatives in Deepgram response".to_string())
        })?;

        let text = alternative.transcript.clone();
        let confidence = Some(alternative.confidence);

        // Parse utterances with speaker labels
        let segments = if let Some(utterances) = &alternative.utterances {
            utterances
                .iter()
                .map(|utt| TranscriptionSegment {
                    text: utt.transcript.clone(),
                    start_ms: (utt.start * 1000.0) as i64,
                    end_ms: (utt.end * 1000.0) as i64,
                    speaker_label: Some(format!("Speaker {}", utt.speaker)),
                    confidence: Some(utt.confidence),
                })
                .collect()
        } else if let Some(words) = &alternative.words {
            // Fallback: group words by speaker if utterances not available
            let mut segments = Vec::new();
            let mut current_speaker = None;
            let mut current_text = String::new();
            let mut current_start = 0i64;
            let mut current_end = 0i64;
            let mut word_count = 0;
            let mut confidence_sum = 0.0;

            for word in words {
                let word_speaker = word.speaker.unwrap_or(0);

                if current_speaker != Some(word_speaker) {
                    // New speaker - save previous segment
                    if !current_text.is_empty() {
                        segments.push(TranscriptionSegment {
                            text: current_text.trim().to_string(),
                            start_ms: current_start,
                            end_ms: current_end,
                            speaker_label: current_speaker.map(|s| format!("Speaker {}", s)),
                            confidence: if word_count > 0 {
                                Some(confidence_sum / word_count as f32)
                            } else {
                                None
                            },
                        });
                    }

                    // Start new segment
                    current_speaker = Some(word_speaker);
                    current_text = word.word.clone();
                    current_start = (word.start * 1000.0) as i64;
                    current_end = (word.end * 1000.0) as i64;
                    word_count = 1;
                    confidence_sum = word.confidence;
                } else {
                    // Same speaker - append word
                    current_text.push(' ');
                    current_text.push_str(&word.word);
                    current_end = (word.end * 1000.0) as i64;
                    word_count += 1;
                    confidence_sum += word.confidence;
                }
            }

            // Save last segment
            if !current_text.is_empty() {
                segments.push(TranscriptionSegment {
                    text: current_text.trim().to_string(),
                    start_ms: current_start,
                    end_ms: current_end,
                    speaker_label: current_speaker.map(|s| format!("Speaker {}", s)),
                    confidence: if word_count > 0 {
                        Some(confidence_sum / word_count as f32)
                    } else {
                        None
                    },
                });
            }

            segments
        } else {
            // No diarization - single segment
            vec![TranscriptionSegment {
                text: text.clone(),
                start_ms: 0,
                end_ms: (response.metadata.duration * 1000.0) as i64,
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
impl TranscriptionServicePort for DeepgramService {
    async fn transcribe_file(
        &self,
        audio_path: &str,
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult> {
        log::info!("Starting Deepgram transcription for: {}", audio_path);

        let result = self.transcribe_with_diarization(audio_path, config).await?;

        log::info!(
            "Deepgram transcription complete: {} segments, {} chars",
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
            "Transcribing {} bytes with Deepgram (format: {})",
            audio_data.len(),
            format
        );

        // Build query parameters
        let mut url = format!("{}/listen", DEEPGRAM_API_BASE);
        let mut params = vec![
            ("punctuate", "true"),
            (
                "diarize",
                if config.enable_diarization {
                    "true"
                } else {
                    "false"
                },
            ),
            ("utterances", "true"),
        ];

        if let Some(lang) = &config.language {
            params.push(("language", lang));
        }

        let query_string = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        url = format!("{}?{}", url, query_string);

        // Determine content type
        let content_type = match format {
            "wav" => "audio/wav",
            "mp3" => "audio/mpeg",
            "flac" => "audio/flac",
            _ => "audio/wav", // Default
        };

        // Send request
        let response = self
            .client
            .post(&url)
            .header("authorization", format!("Token {}", self.api_key))
            .header("content-type", content_type)
            .body(audio_data.to_vec())
            .send()
            .await
            .map_err(|e| AppError::Transcription(format!("Deepgram request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Transcription(format!(
                "Deepgram API error ({}): {}",
                status, error_text
            )));
        }

        let deepgram_response: DeepgramResponse = response.json().await.map_err(|e| {
            AppError::Transcription(format!("Failed to parse Deepgram response: {}", e))
        })?;

        self.parse_deepgram_response(deepgram_response)
    }

    fn provider_name(&self) -> &str {
        "Deepgram"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }
}

// ===== API Response Types =====

#[derive(Debug, Deserialize)]
struct DeepgramResponse {
    metadata: Metadata,
    results: Results,
}

#[derive(Debug, Deserialize)]
struct Metadata {
    duration: f64,
}

#[derive(Debug, Deserialize)]
struct Results {
    channels: Vec<Channel>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    alternatives: Vec<Alternative>,
}

#[derive(Debug, Deserialize)]
struct Alternative {
    transcript: String,
    confidence: f32,
    words: Option<Vec<Word>>,
    utterances: Option<Vec<Utterance>>,
}

#[derive(Debug, Deserialize)]
struct Word {
    word: String,
    start: f64,
    end: f64,
    confidence: f32,
    speaker: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Utterance {
    transcript: String,
    start: f64,
    end: f64,
    confidence: f32,
    speaker: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deepgram_service_creation() {
        let service = DeepgramService::new("test_api_key".to_string());
        assert_eq!(service.provider_name(), "Deepgram");
        assert!(service.is_configured());
    }

    #[test]
    fn test_deepgram_service_not_configured() {
        let service = DeepgramService::new("".to_string());
        assert!(!service.is_configured());
    }
}
