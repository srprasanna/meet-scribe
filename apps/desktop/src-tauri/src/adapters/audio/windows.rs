//! Windows WASAPI Audio Capture Implementation
//!
//! Uses Windows Core Audio APIs (WASAPI) to capture system audio via loopback recording.
//! This allows capturing audio playing through the system without being intrusive.

use crate::error::{AppError, Result};
use crate::ports::audio::{AudioBuffer, AudioCapturePort, AudioFormat};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator,
    MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

/// Windows WASAPI audio capture implementation
pub struct WasapiAudioCapture {
    is_capturing: Arc<Mutex<bool>>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    format: AudioFormat,
    capture_handle: Option<tokio::task::JoinHandle<()>>,
}

impl WasapiAudioCapture {
    /// Creates a new WASAPI audio capture instance
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            format: AudioFormat::default(),
            capture_handle: None,
        }
    }

    /// Initialize COM for the current thread
    fn init_com() -> Result<()> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(|e| AppError::AudioCapture(format!("Failed to initialize COM: {}", e)))?;
        }
        Ok(())
    }

    /// Get the default audio render device (loopback capture)
    fn get_default_device() -> Result<IMMDevice> {
        unsafe {
            let enumerator: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| AppError::AudioCapture(format!("Failed to create device enumerator: {}", e)))?;

            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eConsole)
                .map_err(|e| AppError::AudioCapture(format!("Failed to get default audio endpoint: {}", e)))?;

            Ok(device)
        }
    }

    /// Initialize the audio client with the desired format
    fn initialize_audio_client(audio_client: &IAudioClient) -> Result<(WAVEFORMATEX, u16, u16)> {
        unsafe {
            // Get the device's mix format
            let mix_format_ptr = audio_client
                .GetMixFormat()
                .map_err(|e| AppError::AudioCapture(format!("Failed to get mix format: {}", e)))?;

            if mix_format_ptr.is_null() {
                return Err(AppError::AudioCapture("Mix format pointer is null".to_string()));
            }

            let mix_format = *mix_format_ptr;
            let sample_rate = mix_format.nSamplesPerSec as u16;
            let bits_per_sample = mix_format.wBitsPerSample;

            // Initialize the audio client for loopback capture
            let buffer_duration = 10_000_000; // 1 second in 100-nanosecond units
            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK,
                    buffer_duration,
                    0,
                    mix_format_ptr,
                    None,
                )
                .map_err(|e| AppError::AudioCapture(format!("Failed to initialize audio client: {}", e)))?;

            // Free the mix format
            windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));

            Ok((mix_format, sample_rate, bits_per_sample))
        }
    }

    /// Convert audio samples from bytes to f32 normalized format based on format
    fn convert_samples_to_f32(data: &[u8], format: &WAVEFORMATEX) -> Vec<f32> {
        let mut samples = Vec::new();
        let bits_per_sample = format.wBitsPerSample;

        match bits_per_sample {
            16 => {
                // 16-bit PCM
                for chunk in data.chunks_exact(2) {
                    let sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                    samples.push(sample as f32 / 32768.0);
                }
            }
            32 => {
                // 32-bit float (most common for WASAPI)
                for chunk in data.chunks_exact(4) {
                    let sample = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    samples.push(sample);
                }
            }
            24 => {
                // 24-bit PCM (less common)
                for chunk in data.chunks_exact(3) {
                    let mut bytes = [0u8; 4];
                    bytes[1..4].copy_from_slice(chunk);
                    let sample = i32::from_le_bytes(bytes);
                    samples.push(sample as f32 / 8388608.0);
                }
            }
            _ => {
                log::warn!("Unsupported bit depth: {}", bits_per_sample);
            }
        }

        samples
    }

    /// Perform the actual audio capture loop
    fn capture_loop(
        audio_client: IAudioClient,
        capture_client: IAudioCaptureClient,
        format: WAVEFORMATEX,
        is_capturing: Arc<Mutex<bool>>,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
    ) {
        unsafe {
            // Start the audio client
            if let Err(e) = audio_client.Start() {
                log::error!("Failed to start audio client: {}", e);
                return;
            }

            log::info!("WASAPI capture loop started");

            // Store format values locally to avoid packed field issues
            let frame_size = format.nBlockAlign as usize;
            let bits_per_sample = format.wBitsPerSample;

            // Capture loop
            while *is_capturing.lock().unwrap() {
                // Sleep a bit to avoid busy-waiting
                std::thread::sleep(Duration::from_millis(10));

                // Get the next packet of data
                let packet_length = match capture_client.GetNextPacketSize() {
                    Ok(size) => size,
                    Err(e) => {
                        log::error!("Failed to get packet size: {}", e);
                        break;
                    }
                };

                if packet_length > 0 {
                    let mut data_ptr: *mut u8 = std::ptr::null_mut();
                    let mut num_frames_available: u32 = 0;
                    let mut flags: u32 = 0;

                    // Get the buffer
                    match capture_client.GetBuffer(
                        &mut data_ptr,
                        &mut num_frames_available,
                        &mut flags,
                        None,
                        None,
                    ) {
                        Ok(_) => {
                            // Check if the buffer is silent
                            if (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) == 0 && num_frames_available > 0 {
                                // Calculate the size of the data
                                let data_size = num_frames_available as usize * frame_size;

                                // Copy the data
                                let data_slice = std::slice::from_raw_parts(data_ptr, data_size);

                                // Convert to f32 samples
                                let samples = Self::convert_samples_to_f32(data_slice, &format);

                                // Append to the buffer
                                let mut buffer = audio_buffer.lock().unwrap();
                                buffer.extend(samples);
                            }

                            // Release the buffer
                            if let Err(e) = capture_client.ReleaseBuffer(num_frames_available) {
                                log::error!("Failed to release buffer: {}", e);
                            }
                        }
                        Err(e) => {
                            log::error!("Failed to get buffer: {}", e);
                            break;
                        }
                    }
                }
            }

            // Stop the audio client
            if let Err(e) = audio_client.Stop() {
                log::error!("Failed to stop audio client: {}", e);
            }

            log::info!("WASAPI capture loop stopped");
        }
    }
}

impl Default for WasapiAudioCapture {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AudioCapturePort for WasapiAudioCapture {
    async fn list_devices(&self) -> Result<Vec<String>> {
        // For Phase 2, we'll just return the default device
        // TODO: Implement full device enumeration in future phases
        Ok(vec!["Default Audio Output (Loopback)".to_string()])
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
            // Initialize COM for this thread
            if let Err(e) = Self::init_com() {
                log::error!("Failed to initialize COM: {}", e);
                *is_capturing_clone.lock().unwrap() = false;
                return;
            }

            // Get the default audio device
            let device = match Self::get_default_device() {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to get default device: {}", e);
                    *is_capturing_clone.lock().unwrap() = false;
                    unsafe { CoUninitialize(); }
                    return;
                }
            };

            // Activate the audio client
            let audio_client: IAudioClient = match unsafe {
                device.Activate::<IAudioClient>(CLSCTX_ALL, None)
            } {
                Ok(client) => client,
                Err(e) => {
                    log::error!("Failed to activate audio client: {}", e);
                    *is_capturing_clone.lock().unwrap() = false;
                    unsafe { CoUninitialize(); }
                    return;
                }
            };

            // Initialize the audio client
            let (format, sample_rate, bits_per_sample) = match Self::initialize_audio_client(&audio_client) {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to initialize audio client: {}", e);
                    *is_capturing_clone.lock().unwrap() = false;
                    unsafe { CoUninitialize(); }
                    return;
                }
            };

            // Get the capture client
            let capture_client: IAudioCaptureClient = match unsafe {
                audio_client.GetService::<IAudioCaptureClient>()
            } {
                Ok(client) => client,
                Err(e) => {
                    log::error!("Failed to get capture client: {}", e);
                    *is_capturing_clone.lock().unwrap() = false;
                    unsafe { CoUninitialize(); }
                    return;
                }
            };

            log::info!("WASAPI audio capture initialized successfully");
            log::info!("Format: {} Hz, {} bits", sample_rate, bits_per_sample);

            // Run the capture loop
            Self::capture_loop(
                audio_client,
                capture_client,
                format,
                is_capturing_clone,
                audio_buffer_clone,
            );

            unsafe {
                CoUninitialize();
            }
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
    fn test_new_wasapi_capture() {
        let capture = WasapiAudioCapture::new();
        assert!(!capture.is_capturing());
    }

    #[test]
    fn test_default_format() {
        let capture = WasapiAudioCapture::new();
        let format = capture.get_format();
        assert_eq!(format.sample_rate, 16000);
        assert_eq!(format.channels, 1);
        assert_eq!(format.bits_per_sample, 16);
    }

    #[tokio::test]
    async fn test_list_devices() {
        let capture = WasapiAudioCapture::new();
        let devices = capture.list_devices().await.unwrap();
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_convert_samples_16bit() {
        let data: Vec<u8> = vec![0x00, 0x00, 0x00, 0x40, 0x00, 0xC0];
        let mut format = WAVEFORMATEX::default();
        format.wBitsPerSample = 16;
        format.nBlockAlign = 2;

        let samples = WasapiAudioCapture::convert_samples_to_f32(&data, &format);
        assert_eq!(samples.len(), 3);
        assert!((samples[0] - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_convert_samples_32bit_float() {
        let data: Vec<u8> = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x3F];
        let mut format = WAVEFORMATEX::default();
        format.wBitsPerSample = 32;
        format.nBlockAlign = 4;

        let samples = WasapiAudioCapture::convert_samples_to_f32(&data, &format);
        assert_eq!(samples.len(), 2);
        assert!((samples[0] - 0.0).abs() < 0.001);
        assert!((samples[1] - 1.0).abs() < 0.001);
    }
}
