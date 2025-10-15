//! Linux PulseAudio Capture Implementation
//!
//! Uses PulseAudio monitor sources to capture system audio streams.
//! Monitor sources allow non-intrusive capture of audio playing through the system.

use crate::error::{AppError, Result};
use crate::ports::audio::{AudioBuffer, AudioCapturePort, AudioFormat};
use async_trait::async_trait;
use libpulse_simple_binding::{Simple, Direction, SampleSpec, SampleFormat, ChannelMap};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Linux PulseAudio capture implementation
///
/// Captures system audio output using PulseAudio monitor sources.
/// Uses @DEFAULT_MONITOR@ to capture what's playing through the speakers.
///
/// Audio format: 44100 Hz, 2 channels (stereo), 16-bit signed little-endian
pub struct PulseAudioCapture {
    is_capturing: Arc<Mutex<bool>>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    /// Audio format - placeholder until capture starts, then set to 44.1kHz stereo 16-bit
    format: AudioFormat,
    capture_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PulseAudioCapture {
    /// Creates a new PulseAudio capture instance
    ///
    /// The format field is initialized to a default placeholder.
    /// Actual format (44.1kHz stereo 16-bit) is set when `start_capture()` is called.
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            format: AudioFormat::default(), // Placeholder, updated during start_capture()
            capture_handle: None,
        }
    }

    /// Convert audio samples from i16 to f32 normalized format
    fn convert_samples(samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / 32768.0).collect()
    }
}

impl Default for PulseAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioCapturePort for PulseAudioCapture {
    async fn list_devices(&self) -> Result<Vec<String>> {
        // For Phase 2, we'll just return the default monitor source
        // TODO: Implement full device enumeration using libpulse-binding in future phases
        Ok(vec!["Default Monitor Source".to_string()])
    }

    async fn start_capture(&mut self, device_name: Option<String>) -> Result<()> {
        {
            let mut is_capturing = self.is_capturing.lock().unwrap();
            if *is_capturing {
                return Err(AppError::AudioCapture(
                    "Capture already in progress".to_string(),
                ));
            }

            *is_capturing = true;
        } // Drop is_capturing guard here

        let is_capturing_clone = Arc::clone(&self.is_capturing);
        let audio_buffer_clone = Arc::clone(&self.audio_buffer);

        // Determine which device to use for capture
        // Default to system monitor source if not specified
        let device = device_name.unwrap_or_else(|| "@DEFAULT_MONITOR@".to_string());

        // Store format info to be updated after detection
        let format_info = Arc::new(Mutex::new(AudioFormat::default()));
        let format_info_clone = Arc::clone(&format_info);

        // Spawn background task for audio capture
        let handle = tokio::task::spawn_blocking(move || {
            // Set up PulseAudio sample specification
            let spec = SampleSpec {
                format: SampleFormat::S16LE, // 16-bit signed little-endian
                channels: 2,                  // Stereo
                rate: 44100,                 // 44.1 kHz
            };

            // Store the format
            *format_info_clone.lock().unwrap() = AudioFormat {
                sample_rate: spec.rate,
                channels: spec.channels,
                bits_per_sample: 16, // S16LE is 16-bit
            };

            // Create a simple recording connection
            // Use monitor source to capture system audio output
            let simple = match Simple::new(
                None,                          // Use default server
                "Meet-Scribe",                // Application name
                Direction::Record,            // Recording
                Some(&device),                // Monitor source for system audio
                "Audio Capture",              // Stream description
                &spec,                        // Sample spec
                None,                         // Use default channel map
                None,                         // Use default buffering attributes
            ) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Failed to create PulseAudio simple connection: {}", e);
                    *is_capturing_clone.lock().unwrap() = false;
                    return;
                }
            };

            log::info!("PulseAudio capture initialized successfully");
            log::info!("Device: {}", device);
            log::info!("Format: {} Hz, {} channels, 16-bit", spec.rate, spec.channels);

            // Buffer for reading samples (1024 frames at a time)
            let buffer_size = 1024 * spec.channels as usize * 2; // 2 bytes per sample (16-bit)
            let mut read_buffer = vec![0u8; buffer_size];

            // Capture loop
            while *is_capturing_clone.lock().unwrap() {
                // Read audio data from PulseAudio
                match simple.read(&mut read_buffer) {
                    Ok(_) => {
                        // Convert bytes to i16 samples
                        let mut i16_samples = Vec::with_capacity(buffer_size / 2);
                        for chunk in read_buffer.chunks_exact(2) {
                            let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                            i16_samples.push(sample);
                        }

                        // Convert to f32 normalized format
                        let f32_samples = Self::convert_samples(&i16_samples);

                        // Append to the shared buffer
                        let mut buffer = audio_buffer_clone.lock().unwrap();
                        buffer.extend(f32_samples);
                    }
                    Err(e) => {
                        log::error!("Failed to read from PulseAudio: {}", e);
                        break;
                    }
                }

                // Small sleep to prevent busy-waiting
                std::thread::sleep(Duration::from_millis(1));
            }

            // Drain any remaining buffered data
            if let Err(e) = simple.drain() {
                log::warn!("Failed to drain PulseAudio buffer: {}", e);
            }

            log::info!("PulseAudio capture thread stopped");
        });

        self.capture_handle = Some(handle);

        // Wait for format initialization to complete
        // Format is set to 44100 Hz, stereo, 16-bit in the background thread
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Update our format from the initialized format
        self.format = format_info.lock().unwrap().clone();

        log::info!("Audio capture started with format: {} Hz, {} channels, {} bits",
            self.format.sample_rate, self.format.channels, self.format.bits_per_sample);
        Ok(())
    }

    async fn stop_capture(&mut self) -> Result<()> {
        {
            let mut is_capturing = self.is_capturing.lock().unwrap();
            if !*is_capturing {
                return Ok(());
            }
            *is_capturing = false;
        } // MutexGuard dropped here

        // Wait for capture thread to finish
        if let Some(handle) = self.capture_handle.take() {
            handle
                .await
                .map_err(|e| AppError::AudioCapture(format!("Failed to stop capture thread: {}", e)))?;
        }

        log::info!("Audio capture stopped");
        Ok(())
    }

    async fn get_audio_buffer(&mut self) -> Result<Option<AudioBuffer>> {
        let mut buffer = self.audio_buffer.lock().unwrap();
        if buffer.is_empty() {
            return Ok(None);
        }

        let samples = buffer.drain(..).collect();
        Ok(Some(AudioBuffer {
            samples,
            format: self.format.clone(),
        }))
    }

    fn is_capturing(&self) -> bool {
        *self.is_capturing.lock().unwrap()
    }

    fn get_format(&self) -> AudioFormat {
        self.format.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_pulseaudio_capture() {
        let capture = PulseAudioCapture::new();
        assert!(!capture.is_capturing());
    }

    #[test]
    fn test_default_format() {
        let capture = PulseAudioCapture::new();
        let format = capture.get_format();
        // Before capture starts, format is the default placeholder
        // Actual format is set during start_capture() to: 44100 Hz, stereo, 16-bit
        assert_eq!(format.sample_rate, 16000); // Placeholder before capture
        assert_eq!(format.channels, 1);         // Placeholder before capture
        assert_eq!(format.bits_per_sample, 16); // Placeholder before capture
    }

    #[tokio::test]
    async fn test_list_devices() {
        let capture = PulseAudioCapture::new();
        let devices = capture.list_devices().await.unwrap();
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_convert_samples() {
        let samples = vec![0i16, 16384, -16384, 32767, -32768];
        let converted = PulseAudioCapture::convert_samples(&samples);
        assert_eq!(converted.len(), 5);
        assert!((converted[0] - 0.0).abs() < 0.001);
        assert!((converted[1] - 0.5).abs() < 0.001);
        assert!((converted[2] + 0.5).abs() < 0.001);
    }
}
