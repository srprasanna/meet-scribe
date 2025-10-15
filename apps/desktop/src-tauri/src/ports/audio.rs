/// Audio capture port trait
///
/// Defines the interface for capturing system audio streams.
/// Platform-specific implementations in adapters/audio/
use crate::error::Result;
use async_trait::async_trait;

/// Represents audio format specifications
///
/// The actual format varies by platform and system configuration:
/// - **Windows WASAPI**: Auto-detected from system (typically 48000 Hz, stereo, 32-bit float)
/// - **Linux PulseAudio**: Fixed at 44100 Hz, stereo, 16-bit signed
///
/// The default values are placeholder values used before audio capture starts.
#[derive(Debug, Clone)]
pub struct AudioFormat {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u16,
}

impl Default for AudioFormat {
    fn default() -> Self {
        Self {
            sample_rate: 16000, // Placeholder - actual format set during capture
            channels: 1,        // Placeholder - actual format set during capture
            bits_per_sample: 16, // Placeholder - actual format set during capture
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
