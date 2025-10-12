/// Transcription service port trait
///
/// Defines the interface for ASR (Automatic Speech Recognition) services.
/// Implementations: AssemblyAI, Deepgram
use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Represents a transcription result with diarization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Full transcript text
    pub text: String,

    /// Individual words or segments with timestamps and speaker labels
    pub segments: Vec<TranscriptionSegment>,

    /// Overall confidence score (0.0 to 1.0)
    pub confidence: Option<f32>,
}

/// Represents a segment of transcription with timing and speaker info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// The transcribed text for this segment
    pub text: String,

    /// Start time in milliseconds
    pub start_ms: i64,

    /// End time in milliseconds
    pub end_ms: i64,

    /// Speaker label (e.g., "Speaker 1", "Speaker 2")
    pub speaker_label: Option<String>,

    /// Confidence score for this segment (0.0 to 1.0)
    pub confidence: Option<f32>,
}

/// Configuration for transcription request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Enable speaker diarization
    pub enable_diarization: bool,

    /// Number of speakers (if known, helps diarization accuracy)
    pub num_speakers: Option<u32>,

    /// Language code (e.g., "en", "es", "fr")
    pub language: Option<String>,

    /// Provider-specific settings as JSON
    pub additional_settings: Option<serde_json::Value>,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enable_diarization: true,
            num_speakers: None,
            language: Some("en".to_string()),
            additional_settings: None,
        }
    }
}

/// Port trait for transcription services (ASR)
#[async_trait]
pub trait TranscriptionServicePort: Send + Sync {
    /// Transcribe audio from a file path
    async fn transcribe_file(
        &self,
        audio_path: &str,
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult>;

    /// Transcribe audio from raw bytes
    async fn transcribe_bytes(
        &self,
        audio_data: &[u8],
        format: &str, // "wav", "mp3", etc.
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult>;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Check if the service is configured (has API key)
    fn is_configured(&self) -> bool;
}
