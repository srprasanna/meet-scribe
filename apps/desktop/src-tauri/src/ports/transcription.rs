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

    /// Model to use for transcription (provider-specific)
    pub model: Option<String>,

    /// Provider-specific settings as JSON
    pub additional_settings: Option<serde_json::Value>,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            enable_diarization: true,
            num_speakers: None,
            language: Some("en".to_string()),
            model: None,
            additional_settings: None,
        }
    }
}

/// Callback trait for streaming transcription events
#[async_trait]
pub trait StreamingTranscriptionCallback: Send + Sync {
    /// Called when a new transcript segment is received
    async fn on_transcript(&self, segment: TranscriptionSegment);

    /// Called when an interim (partial) transcript is received
    /// Interim transcripts are not final and may change
    async fn on_interim_transcript(&self, segment: TranscriptionSegment);

    /// Called when the stream encounters an error
    async fn on_error(&self, error: String);

    /// Called when the stream is closed
    async fn on_close(&self);
}

/// Port trait for transcription services (ASR)
#[async_trait]
pub trait TranscriptionServicePort: Send + Sync {
    /// Transcribe audio from a file path (batch mode)
    async fn transcribe_file(
        &self,
        audio_path: &str,
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult>;

    /// Transcribe audio from raw bytes (batch mode)
    async fn transcribe_bytes(
        &self,
        audio_data: &[u8],
        format: &str, // "wav", "mp3", etc.
        config: &TranscriptionConfig,
    ) -> Result<TranscriptionResult>;

    /// Start a streaming transcription session (real-time mode)
    /// Returns a session handle that can be used to send audio chunks
    async fn start_streaming(
        &self,
        config: &TranscriptionConfig,
        callback: Box<dyn StreamingTranscriptionCallback>,
    ) -> Result<Box<dyn StreamingSession>>;

    /// Get the provider name
    fn provider_name(&self) -> &str;

    /// Check if the service is configured (has API key)
    fn is_configured(&self) -> bool;

    /// Check if streaming is supported by this provider
    fn supports_streaming(&self) -> bool {
        false // Default: not supported (backward compatibility)
    }
}

/// Handle for an active streaming transcription session
#[async_trait]
pub trait StreamingSession: Send + Sync {
    /// Send an audio chunk to the streaming session
    /// Audio should be raw PCM data matching the session's format
    async fn send_audio(&mut self, audio_chunk: &[u8]) -> Result<()>;

    /// Flush any buffered audio and finalize remaining transcripts
    async fn flush(&mut self) -> Result<()>;

    /// Close the streaming session
    async fn close(&mut self) -> Result<()>;

    /// Check if the session is still active
    fn is_active(&self) -> bool;
}
