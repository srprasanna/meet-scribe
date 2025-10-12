/// Audio capture port trait
///
/// Defines the interface for capturing system audio streams.
/// Platform-specific implementations in adapters/audio/
use crate::error::Result;
use async_trait::async_trait;

/// Represents audio format specifications
#[derive(Debug, Clone)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 16000, // 16kHz is standard for speech recognition
            channels: 1,        // Mono
            bits_per_sample: 16,
        }
    }
}

/// Audio buffer containing captured audio samples
#[derive(Debug)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub format: AudioFormat,
}

/// Port trait for audio capture functionality
#[async_trait]
pub trait AudioCapturePort: Send + Sync {
    /// Lists available audio devices for capture
    async fn list_devices(&self) -> Result<Vec<String>>;

    /// Starts capturing audio from the specified device
    /// Returns immediately, audio is captured in background
    async fn start_capture(&mut self, device_name: Option<String>) -> Result<()>;

    /// Stops audio capture
    async fn stop_capture(&mut self) -> Result<()>;

    /// Retrieves captured audio buffer
    /// Returns None if no audio has been captured yet
    async fn get_audio_buffer(&mut self) -> Result<Option<AudioBuffer>>;

    /// Checks if currently capturing
    fn is_capturing(&self) -> bool;

    /// Gets the audio format being used
    fn get_format(&self) -> AudioFormat;
}
