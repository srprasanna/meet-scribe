/// Port trait definitions (interfaces)
///
/// These traits define the contracts for adapters to implement.
/// Following the ports-and-adapters (hexagonal) architecture pattern.
pub mod audio;
pub mod llm;
pub mod storage;
pub mod transcription;

#[cfg(test)]
pub mod mocks;

pub use audio::{AudioBuffer, AudioCapturePort, AudioFormat};
pub use llm::{GeneratedInsight, InsightRequest, LlmConfig, LlmServicePort};
pub use storage::StoragePort;
pub use transcription::{
    TranscriptionConfig, TranscriptionResult, TranscriptionSegment, TranscriptionServicePort,
};
