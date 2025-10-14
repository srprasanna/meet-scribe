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
pub struct PulseAudioCapture {
    is_capturing: Arc<Mutex<bool>>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    format: AudioFormat,
    capture_handle: Option<tokio::task::JoinHandle<()>>,
}

impl PulseAudioCapture {
    /// Creates a new PulseAudio capture instance
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            format: AudioFormat::default(),
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

    async fn start_capture(&mut self, _device_name: Option<String>) -> Result<()> {
        let mut is_capturing = self.is_capturing.lock().unwrap();
        if *is_capturing {
            return Err(AppError::AudioCapture(
                "Capture already in progress".to_string(),
            ));
        }

        *is_capturing = true;
        drop(is_capturing);

        let is_capturing_clone = Arc::clone(&self.is_capturing);
        let audio_buffer_clone = Arc::clone(&self.audio_buffer);

        // Spawn background task for audio capture
        let handle = tokio::task::spawn_blocking(move || {
            // Set up PulseAudio sample specification
            let spec = SampleSpec {
                format: SampleFormat::S16LE, // 16-bit signed little-endian
                channels: 2,                  // Stereo
                rate: 44100,                 // 44.1 kHz
            };

            // Create a simple recording connection
            // Using None for device name uses the default monitor source
            let simple = match Simple::new(
                None,                          // Use default server
                "Meet-Scribe",                // Application name
                Direction::Record,            // Recording
                None,                         // Use default monitor device
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
        log::info!("Audio capture started");
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
        assert_eq!(format.sample_rate, 16000);
        assert_eq!(format.channels, 1);
        assert_eq!(format.bits_per_sample, 16);
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
