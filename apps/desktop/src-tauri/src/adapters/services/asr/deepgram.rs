//! Deepgram transcription service adapter
//!
//! Implements the TranscriptionServicePort for Deepgram's API.
//! Simpler API than AssemblyAI - single request with file streaming.

use crate::error::{AppError, Result};
use crate::ports::transcription::{
    StreamingSession, StreamingTranscriptionCallback, TranscriptionConfig, TranscriptionResult,
    TranscriptionSegment, TranscriptionServicePort,
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

    /// Fetch available models from Deepgram API
    /// Filters to only English-supporting models and excludes outdated models
    pub async fn list_models(&self) -> Result<Vec<DeepgramModel>> {
        log::info!("Fetching Deepgram models from API (English only, exclude outdated)");

        let url = format!("{}/models?include_outdated=false", DEEPGRAM_API_BASE);

        let response = self
            .client
            .get(&url)
            .header("authorization", format!("Token {}", self.api_key))
            .send()
            .await
            .map_err(|e| AppError::Transcription(format!("Failed to fetch models: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(AppError::Transcription(format!(
                "Deepgram API error ({}): {}",
                status, error_text
            )));
        }

        let models_response: DeepgramModelsResponse = response.json().await.map_err(|e| {
            AppError::Transcription(format!("Failed to parse models response: {}", e))
        })?;

        // Return only STT models (Speech-to-Text) that support English
        Ok(models_response
            .stt
            .into_iter()
            .filter(|model| {
                model.languages.iter().any(|lang| {
                    lang.starts_with("en")
                        || lang == "en-US"
                        || lang == "en-GB"
                        || lang == "en-AU"
                        || lang == "en-IN"
                        || lang == "en-NZ"
                })
            })
            .collect())
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

        // Log WAV file details
        if buffer.len() > 44 {
            // WAV header is 44 bytes - check if this looks like a valid WAV
            let is_wav = &buffer[0..4] == b"RIFF" && &buffer[8..12] == b"WAVE";
            println!(
                ">>> WAV file check: is_valid_wav={}, total_bytes={}",
                is_wav,
                buffer.len()
            );

            if is_wav {
                // Parse basic WAV info
                let audio_format = u16::from_le_bytes([buffer[20], buffer[21]]);
                let num_channels = u16::from_le_bytes([buffer[22], buffer[23]]);
                let sample_rate =
                    u32::from_le_bytes([buffer[24], buffer[25], buffer[26], buffer[27]]);
                let bits_per_sample = u16::from_le_bytes([buffer[34], buffer[35]]);

                println!(">>> WAV format: audio_format={}, channels={}, sample_rate={}, bits_per_sample={}",
                    audio_format, num_channels, sample_rate, bits_per_sample);
            } else {
                println!("!!! WARNING: File doesn't have valid WAV header!");
            }
        }

        // Build query parameters
        let mut url = format!("{}/listen", DEEPGRAM_API_BASE);

        // Use model from config, or default to nova-2-meeting
        let model = config.model.as_deref().unwrap_or("nova-2-meeting");

        let mut params = vec![
            ("model", model),
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

        println!(">>> Sending request to Deepgram API: {}", url);
        println!(">>> Audio file size: {} bytes", buffer.len());
        log::info!("Sending request to Deepgram API: {}", url);
        log::info!("Audio file size: {} bytes", buffer.len());

        // Send request
        let response = self
            .client
            .post(&url)
            .header("authorization", format!("Token {}", self.api_key))
            .header("content-type", "audio/wav")
            .body(buffer)
            .send()
            .await
            .map_err(|e| {
                log::error!("Deepgram HTTP request failed: {}", e);
                AppError::Transcription(format!("Deepgram request failed: {}", e))
            })?;

        let status = response.status();
        println!(">>> Deepgram API response status: {}", status);
        log::info!("Deepgram API response status: {}", status);

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            println!("!!! Deepgram API error ({}): {}", status, error_text);
            log::error!("Deepgram API error response: {}", error_text);
            return Err(AppError::Transcription(format!(
                "Deepgram API error ({}): {}",
                status, error_text
            )));
        }

        let deepgram_response: DeepgramResponse = response.json().await.map_err(|e| {
            println!("!!! Failed to parse Deepgram JSON response: {}", e);
            log::error!("Failed to parse Deepgram JSON response: {}", e);
            AppError::Transcription(format!("Failed to parse Deepgram response: {}", e))
        })?;

        println!(">>> Successfully parsed Deepgram JSON response");
        println!(">>> Channels: {}", deepgram_response.results.channels.len());

        let result = self.parse_deepgram_response(deepgram_response)?;
        println!(">>> Parsed into {} segments", result.segments.len());
        println!(">>> Transcript text length: {} chars", result.text.len());

        Ok(result)
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

        println!(">>> Transcript text from Deepgram: {} chars", text.len());
        println!(">>> Has utterances: {}", alternative.utterances.is_some());
        println!(">>> Has words: {}", alternative.words.is_some());

        if let Some(ref utterances) = alternative.utterances {
            println!(">>> Utterances count: {}", utterances.len());
        }
        if let Some(ref words) = alternative.words {
            println!(">>> Words count: {}", words.len());
        }

        // Parse utterances with speaker labels
        let segments = if let Some(utterances) = &alternative.utterances {
            println!(">>> Using utterances for segments");
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
            println!(">>> Using words fallback for segments");
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
            println!(">>> No utterances or words - using fallback single segment");
            // No diarization - single segment
            if text.is_empty() {
                println!("!!! WARNING: Transcript text is empty!");
                vec![]
            } else {
                vec![TranscriptionSegment {
                    text: text.clone(),
                    start_ms: 0,
                    end_ms: (response.metadata.duration * 1000.0) as i64,
                    speaker_label: None,
                    confidence,
                }]
            }
        };

        println!(">>> Final segments count: {}", segments.len());

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

        // Use model from config, or default to nova-2-meeting
        let model = config.model.as_deref().unwrap_or("nova-2-meeting");

        let mut params = vec![
            ("model", model),
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

    async fn start_streaming(
        &self,
        config: &TranscriptionConfig,
        callback: Box<dyn StreamingTranscriptionCallback>,
    ) -> Result<Box<dyn StreamingSession>> {
        log::info!("Starting Deepgram streaming session");

        // Import the streaming module
        use super::deepgram_streaming::DeepgramStreamingSession;

        let session = DeepgramStreamingSession::new(self.api_key.clone(), config, callback).await?;

        Ok(Box::new(session))
    }

    fn provider_name(&self) -> &str {
        "Deepgram"
    }

    fn is_configured(&self) -> bool {
        !self.api_key.is_empty()
    }

    fn supports_streaming(&self) -> bool {
        true // Deepgram supports streaming
    }
}

// ===== API Response Types =====

/// Response from /v1/models endpoint
#[derive(Debug, Deserialize)]
struct DeepgramModelsResponse {
    stt: Vec<DeepgramModel>,
    #[allow(dead_code)]
    tts: Vec<serde_json::Value>, // TTS models - not used for transcription
}

/// Deepgram STT model information
#[derive(Debug, Deserialize, Clone)]
pub struct DeepgramModel {
    pub name: String,
    pub canonical_name: String,
    pub architecture: String,
    pub languages: Vec<String>,
    pub version: String,
    pub uuid: String,
    pub batch: bool,
    pub streaming: bool,
    pub formatted_output: bool,
}

/// Response from /v1/listen endpoint
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
