//! Linux PulseAudio Capture Implementation
//!
//! Uses PulseAudio monitor sources to capture system audio streams.
//! Monitor sources allow non-intrusive capture of audio playing through the system.

use crate::error::{AppError, Result};
use crate::ports::audio::{AudioBuffer, AudioCapturePort, AudioFormat};
use async_trait::async_trait;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::sample::{Format, Spec};
use libpulse_binding::stream::Direction;
use libpulse_simple_binding::Simple;
use std::cell::RefCell;
use std::rc::Rc;
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
    /// Current audio level for visual feedback (0.0 to 1.0)
    current_level: Arc<Mutex<f32>>,
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
            current_level: Arc::new(Mutex::new(0.0)),
        }
    }

    /// Convert audio samples from i16 to f32 normalized format
    fn convert_samples(samples: &[i16]) -> Vec<f32> {
        samples.iter().map(|&s| s as f32 / 32768.0).collect()
    }

    /// Get the PulseAudio device name by index
    ///
    /// Parses the device index from the device selection string and returns the appropriate
    /// PulseAudio device name. Index 0 is always the default monitor.
    fn get_device_name_by_index(device_index: usize) -> Result<String> {
        if device_index == 0 {
            // Default monitor source
            return Ok("@DEFAULT_MONITOR@".to_string());
        }

        // For non-default devices, we need to enumerate and find the device by index
        // This is a simplified approach - in production, you might want to cache device names
        // For now, we'll use the device index as a suffix to query specific devices
        // PulseAudio device names are typically like "alsa_output.pci-0000_00_1f.3.analog-stereo"

        // Return a placeholder that will be resolved during enumeration
        // In practice, the device selection should pass the actual device name, not just the index
        Ok(format!("device_{}", device_index))
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
        tokio::task::spawn_blocking(|| {
            // Create a mainloop and context for introspection
            let mut mainloop = Mainloop::new().ok_or_else(|| {
                AppError::AudioCapture("Failed to create PulseAudio mainloop".to_string())
            })?;

            let mut context = Context::new(&mainloop, "Meet-Scribe Device Enumeration")
                .ok_or_else(|| {
                    AppError::AudioCapture("Failed to create PulseAudio context".to_string())
                })?;

            // Connect to PulseAudio server
            context
                .connect(None, ContextFlagSet::NOFLAGS, None)
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to connect to PulseAudio: {}", e))
                })?;

            // Start the mainloop
            mainloop.lock();
            mainloop
                .start()
                .map_err(|e| AppError::AudioCapture(format!("Failed to start mainloop: {}", e)))?;

            // Wait for context to be ready
            loop {
                match context.get_state() {
                    libpulse_binding::context::State::Ready => break,
                    libpulse_binding::context::State::Failed
                    | libpulse_binding::context::State::Terminated => {
                        mainloop.unlock();
                        mainloop.stop();
                        return Err(AppError::AudioCapture(
                            "PulseAudio context failed".to_string(),
                        ));
                    }
                    _ => {
                        mainloop.unlock();
                        std::thread::sleep(Duration::from_millis(10));
                        mainloop.lock();
                    }
                }
            }

            // Device list to be populated
            let devices: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let devices_clone = Rc::clone(&devices);

            // Flag to track completion
            let done = Rc::new(RefCell::new(false));
            let done_clone = Rc::clone(&done);

            // Add default monitor source first
            devices
                .borrow_mut()
                .push("0: Default Monitor (Default Speaker)".to_string());
            let mut device_index = 1;

            // Enumerate sink (speaker/output) devices
            let devices_sinks = Rc::clone(&devices);
            let done_sinks = Rc::clone(&done);
            let mut sink_index = device_index;

            let introspector = context.introspect();
            introspector.get_sink_info_list(move |result| match result {
                libpulse_binding::callbacks::ListResult::Item(sink_info) => {
                    let name = sink_info
                        .description
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| {
                            sink_info
                                .name
                                .as_ref()
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| format!("Speaker {}", sink_index))
                        });
                    devices_sinks
                        .borrow_mut()
                        .push(format!("{}: {} (Speaker)", sink_index, name));
                    sink_index += 1;
                }
                libpulse_binding::callbacks::ListResult::End => {
                    *done_sinks.borrow_mut() = true;
                }
                libpulse_binding::callbacks::ListResult::Error => {
                    log::error!("Error enumerating sinks");
                    *done_sinks.borrow_mut() = true;
                }
            });

            // Wait for sink enumeration to complete
            mainloop.unlock();
            while !*done.borrow() {
                std::thread::sleep(Duration::from_millis(10));
            }
            mainloop.lock();

            device_index = sink_index;
            *done.borrow_mut() = false;

            // Enumerate source (microphone/input) devices
            let devices_sources = Rc::clone(&devices_clone);
            let done_sources = Rc::clone(&done_clone);
            let mut source_index = device_index;

            let introspector = context.introspect();
            introspector.get_source_info_list(move |result| {
                match result {
                    libpulse_binding::callbacks::ListResult::Item(source_info) => {
                        // Skip monitor sources (they're for capturing output, not real microphones)
                        let is_monitor = source_info.monitor_of_sink.is_some();
                        if !is_monitor {
                            let name = source_info
                                .description
                                .as_ref()
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| {
                                    source_info
                                        .name
                                        .as_ref()
                                        .map(|n| n.to_string())
                                        .unwrap_or_else(|| format!("Microphone {}", source_index))
                                });
                            devices_sources
                                .borrow_mut()
                                .push(format!("{}: {} (Microphone)", source_index, name));
                            source_index += 1;
                        }
                    }
                    libpulse_binding::callbacks::ListResult::End => {
                        *done_sources.borrow_mut() = true;
                    }
                    libpulse_binding::callbacks::ListResult::Error => {
                        log::error!("Error enumerating sources");
                        *done_sources.borrow_mut() = true;
                    }
                }
            });

            // Wait for source enumeration to complete
            mainloop.unlock();
            while !*done_clone.borrow() {
                std::thread::sleep(Duration::from_millis(10));
            }
            mainloop.lock();

            // Cleanup
            mainloop.unlock();
            mainloop.stop();
            context.disconnect();

            let result = devices_clone.borrow().clone();
            log::info!("Found {} audio devices on Linux", result.len());
            Ok(result)
        })
        .await
        .map_err(|e| AppError::AudioCapture(format!("Task join error: {}", e)))?
    }

    async fn list_speaker_devices(&self) -> Result<Vec<String>> {
        tokio::task::spawn_blocking(|| {
            let mut mainloop = Mainloop::new().ok_or_else(|| {
                AppError::AudioCapture("Failed to create PulseAudio mainloop".to_string())
            })?;

            let mut context = Context::new(&mainloop, "Meet-Scribe Speaker Enumeration")
                .ok_or_else(|| {
                    AppError::AudioCapture("Failed to create PulseAudio context".to_string())
                })?;

            context
                .connect(None, ContextFlagSet::NOFLAGS, None)
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to connect to PulseAudio: {}", e))
                })?;

            mainloop.lock();
            mainloop
                .start()
                .map_err(|e| AppError::AudioCapture(format!("Failed to start mainloop: {}", e)))?;

            // Wait for context to be ready
            loop {
                match context.get_state() {
                    libpulse_binding::context::State::Ready => break,
                    libpulse_binding::context::State::Failed
                    | libpulse_binding::context::State::Terminated => {
                        mainloop.unlock();
                        mainloop.stop();
                        return Err(AppError::AudioCapture(
                            "PulseAudio context failed".to_string(),
                        ));
                    }
                    _ => {
                        mainloop.unlock();
                        std::thread::sleep(Duration::from_millis(10));
                        mainloop.lock();
                    }
                }
            }

            let devices: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let devices_clone = Rc::clone(&devices);
            let done = Rc::new(RefCell::new(false));
            let done_clone = Rc::clone(&done);

            // Add default monitor source first
            devices
                .borrow_mut()
                .push("0: Default Monitor (Default)".to_string());

            // Enumerate sink (speaker/output) devices
            let mut sink_index = 1;
            let introspector = context.introspect();
            introspector.get_sink_info_list(move |result| match result {
                libpulse_binding::callbacks::ListResult::Item(sink_info) => {
                    let name = sink_info
                        .description
                        .as_ref()
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| {
                            sink_info
                                .name
                                .as_ref()
                                .map(|n| n.to_string())
                                .unwrap_or_else(|| format!("Speaker {}", sink_index))
                        });
                    devices_clone
                        .borrow_mut()
                        .push(format!("{}: {}", sink_index, name));
                    sink_index += 1;
                }
                libpulse_binding::callbacks::ListResult::End => {
                    *done_clone.borrow_mut() = true;
                }
                libpulse_binding::callbacks::ListResult::Error => {
                    log::error!("Error enumerating sinks");
                    *done_clone.borrow_mut() = true;
                }
            });

            mainloop.unlock();
            while !*done.borrow() {
                std::thread::sleep(Duration::from_millis(10));
            }
            mainloop.lock();

            mainloop.unlock();
            mainloop.stop();
            context.disconnect();

            let result = devices.borrow().clone();
            log::info!("Found {} speaker devices on Linux", result.len());
            Ok(result)
        })
        .await
        .map_err(|e| AppError::AudioCapture(format!("Task join error: {}", e)))?
    }

    async fn list_microphone_devices(&self) -> Result<Vec<String>> {
        tokio::task::spawn_blocking(|| {
            let mut mainloop = Mainloop::new().ok_or_else(|| {
                AppError::AudioCapture("Failed to create PulseAudio mainloop".to_string())
            })?;

            let mut context = Context::new(&mainloop, "Meet-Scribe Microphone Enumeration")
                .ok_or_else(|| {
                    AppError::AudioCapture("Failed to create PulseAudio context".to_string())
                })?;

            context
                .connect(None, ContextFlagSet::NOFLAGS, None)
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to connect to PulseAudio: {}", e))
                })?;

            mainloop.lock();
            mainloop
                .start()
                .map_err(|e| AppError::AudioCapture(format!("Failed to start mainloop: {}", e)))?;

            // Wait for context to be ready
            loop {
                match context.get_state() {
                    libpulse_binding::context::State::Ready => break,
                    libpulse_binding::context::State::Failed
                    | libpulse_binding::context::State::Terminated => {
                        mainloop.unlock();
                        mainloop.stop();
                        return Err(AppError::AudioCapture(
                            "PulseAudio context failed".to_string(),
                        ));
                    }
                    _ => {
                        mainloop.unlock();
                        std::thread::sleep(Duration::from_millis(10));
                        mainloop.lock();
                    }
                }
            }

            let devices: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
            let devices_clone = Rc::clone(&devices);
            let done = Rc::new(RefCell::new(false));
            let done_clone = Rc::clone(&done);

            // Enumerate source (microphone/input) devices
            let mut source_index = 0;
            let introspector = context.introspect();
            introspector.get_source_info_list(move |result| {
                match result {
                    libpulse_binding::callbacks::ListResult::Item(source_info) => {
                        // Skip monitor sources (they're for capturing output, not real microphones)
                        let is_monitor = source_info.monitor_of_sink.is_some();
                        if !is_monitor {
                            let name = source_info
                                .description
                                .as_ref()
                                .map(|d| d.to_string())
                                .unwrap_or_else(|| {
                                    source_info
                                        .name
                                        .as_ref()
                                        .map(|n| n.to_string())
                                        .unwrap_or_else(|| format!("Microphone {}", source_index))
                                });
                            devices_clone
                                .borrow_mut()
                                .push(format!("{}: {}", source_index, name));
                            source_index += 1;
                        }
                    }
                    libpulse_binding::callbacks::ListResult::End => {
                        *done_clone.borrow_mut() = true;
                    }
                    libpulse_binding::callbacks::ListResult::Error => {
                        log::error!("Error enumerating sources");
                        *done_clone.borrow_mut() = true;
                    }
                }
            });

            mainloop.unlock();
            while !*done.borrow() {
                std::thread::sleep(Duration::from_millis(10));
            }
            mainloop.lock();

            mainloop.unlock();
            mainloop.stop();
            context.disconnect();

            let result = devices.borrow().clone();
            log::info!("Found {} microphone devices on Linux", result.len());
            Ok(result)
        })
        .await
        .map_err(|e| AppError::AudioCapture(format!("Task join error: {}", e)))?
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

        // Parse device index from device name
        // Device name format: "0: Device Name (Type)" or "1: Device Name (Type)"
        let device_index = device_name
            .as_ref()
            .and_then(|name| {
                // Extract index from "N: Device Name" format
                name.split(':').next()?.trim().parse::<usize>().ok()
            })
            .unwrap_or(0); // Default to index 0 if parsing fails

        log::info!("Using audio device index: {}", device_index);

        // Determine which device to use for capture
        // Default to system monitor source if not specified
        let device = Self::get_device_name_by_index(device_index)?;

        // Store format info to be updated after detection
        let format_info = Arc::new(Mutex::new(AudioFormat::default()));
        let format_info_clone = Arc::clone(&format_info);

        // Spawn background task for audio capture
        let handle = tokio::task::spawn_blocking(move || {
            // Set up PulseAudio sample specification
            let spec = Spec {
                format: Format::S16le, // 16-bit signed little-endian
                channels: 2,           // Stereo
                rate: 44100,           // 44.1 kHz
            };

            // Store the format
            *format_info_clone.lock().unwrap() = AudioFormat {
                sample_rate: spec.rate,
                channels: spec.channels as u16,
                bits_per_sample: 16, // S16LE is 16-bit
            };

            // Create a simple recording connection
            // Use monitor source to capture system audio output
            let simple = match Simple::new(
                None,              // Use default server
                "Meet-Scribe",     // Application name
                Direction::Record, // Recording
                Some(&device),     // Monitor source for system audio
                "Audio Capture",   // Stream description
                &spec,             // Sample spec
                None,              // Use default channel map
                None,              // Use default buffering attributes
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
            log::info!(
                "Format: {} Hz, {} channels, 16-bit",
                spec.rate,
                spec.channels
            );

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

        log::info!(
            "Audio capture started with format: {} Hz, {} channels, {} bits",
            self.format.sample_rate,
            self.format.channels,
            self.format.bits_per_sample
        );
        Ok(())
    }

    async fn start_dual_capture(
        &mut self,
        speaker_device: Option<String>,
        microphone_device: Option<String>,
    ) -> Result<()> {
        {
            let mut is_capturing = self.is_capturing.lock().unwrap();
            if *is_capturing {
                return Err(AppError::AudioCapture(
                    "Capture already in progress".to_string(),
                ));
            }
            *is_capturing = true;
        }

        let is_capturing_clone = Arc::clone(&self.is_capturing);
        let audio_buffer_clone = Arc::clone(&self.audio_buffer);
        let current_level_clone = Arc::clone(&self.current_level);

        // Speaker: index 0 → @DEFAULT_MONITOR@ (loopback of default speaker output)
        let speaker_index = speaker_device
            .as_ref()
            .and_then(|name| name.split(':').next()?.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let speaker_pa_device = Self::get_device_name_by_index(speaker_index)?;

        // Mic: use @DEFAULT_SOURCE@ (PulseAudio default input / microphone).
        // Both streams request stereo 44100 S16le — PulseAudio converts internally,
        // so both buffers always have the same channel layout and can be mixed directly.
        // TODO: resolve the actual PulseAudio source name when a specific mic index is provided.
        let mic_pa_device = "@DEFAULT_SOURCE@".to_string();

        log::info!(
            "Linux dual capture: speaker={}, mic={}",
            speaker_pa_device,
            mic_pa_device
        );

        let format_info = Arc::new(Mutex::new(AudioFormat::default()));
        let format_info_clone = Arc::clone(&format_info);
        *format_info_clone.lock().unwrap() = AudioFormat {
            sample_rate: 44100,
            channels: 2,
            bits_per_sample: 16,
        };

        let handle = tokio::task::spawn_blocking(move || {
            let speaker_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
            let mic_buf: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));

            let is_capturing_spk = Arc::clone(&is_capturing_clone);
            let is_capturing_mic = Arc::clone(&is_capturing_clone);
            let speaker_buf_writer = Arc::clone(&speaker_buf);
            let mic_buf_writer = Arc::clone(&mic_buf);

            // Speaker capture thread — creates its own Simple connection inside
            let spk_thread = std::thread::spawn(move || {
                let spec = Spec {
                    format: Format::S16le,
                    channels: 2,
                    rate: 44100,
                };
                let simple = match Simple::new(
                    None,
                    "Meet-Scribe",
                    Direction::Record,
                    Some(&speaker_pa_device),
                    "Speaker Capture",
                    &spec,
                    None,
                    None,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to create speaker PulseAudio stream: {}", e);
                        return;
                    }
                };

                log::info!(
                    "Speaker capture started: {} Hz, {} ch, 16-bit S16le",
                    spec.rate,
                    spec.channels
                );

                let buffer_size = 1024 * spec.channels as usize * 2; // frames × channels × bytes
                let mut read_buffer = vec![0u8; buffer_size];

                while *is_capturing_spk.lock().unwrap() {
                    match simple.read(&mut read_buffer) {
                        Ok(_) => {
                            let samples: Vec<f32> = read_buffer
                                .chunks_exact(2)
                                .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                                .collect();
                            speaker_buf_writer.lock().unwrap().extend(samples);
                        }
                        Err(e) => {
                            log::error!("Speaker read error: {}", e);
                            break;
                        }
                    }
                }

                log::info!("Speaker capture thread stopped");
            });

            // Mic capture thread — creates its own Simple connection inside.
            // Requests stereo so PulseAudio upmixes mono hardware internally;
            // both buffers are always stereo, eliminating any channel mismatch in the mixer.
            let mic_thread = std::thread::spawn(move || {
                let spec = Spec {
                    format: Format::S16le,
                    channels: 2,
                    rate: 44100,
                };
                let simple = match Simple::new(
                    None,
                    "Meet-Scribe",
                    Direction::Record,
                    Some(&mic_pa_device),
                    "Mic Capture",
                    &spec,
                    None,
                    None,
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Failed to create mic PulseAudio stream: {}", e);
                        return;
                    }
                };

                log::info!(
                    "Mic capture started: {} Hz, {} ch, 16-bit S16le",
                    spec.rate,
                    spec.channels
                );

                let buffer_size = 1024 * spec.channels as usize * 2;
                let mut read_buffer = vec![0u8; buffer_size];

                while *is_capturing_mic.lock().unwrap() {
                    match simple.read(&mut read_buffer) {
                        Ok(_) => {
                            let samples: Vec<f32> = read_buffer
                                .chunks_exact(2)
                                .map(|c| i16::from_le_bytes([c[0], c[1]]) as f32 / 32768.0)
                                .collect();
                            mic_buf_writer.lock().unwrap().extend(samples);
                        }
                        Err(e) => {
                            log::error!("Mic read error: {}", e);
                            break;
                        }
                    }
                }

                log::info!("Mic capture thread stopped");
            });

            // Mixer loop: drain both buffers every 50 ms and mix.
            // Both streams are stereo so samples interleave identically — no channel mismatch.
            while *is_capturing_clone.lock().unwrap() {
                std::thread::sleep(Duration::from_millis(50));

                let speaker_samples: Vec<f32> = {
                    let mut buf = speaker_buf.lock().unwrap();
                    buf.drain(..).collect()
                };
                let mic_samples: Vec<f32> = {
                    let mut buf = mic_buf.lock().unwrap();
                    buf.drain(..).collect()
                };

                if speaker_samples.is_empty() && mic_samples.is_empty() {
                    continue;
                }

                let max_len = speaker_samples.len().max(mic_samples.len());
                let mut mixed = Vec::with_capacity(max_len);
                for i in 0..max_len {
                    let s = speaker_samples.get(i).copied().unwrap_or(0.0);
                    let m = mic_samples.get(i).copied().unwrap_or(0.0);
                    mixed.push((s + m) / 2.0);
                }

                if !mixed.is_empty() {
                    let level = mixed.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                    *current_level_clone.lock().unwrap() = level;
                }

                audio_buffer_clone.lock().unwrap().extend(mixed);
            }

            let _ = spk_thread.join();
            let _ = mic_thread.join();

            log::info!("PulseAudio dual capture stopped");
        });

        self.capture_handle = Some(handle);

        // Brief wait for both PulseAudio streams to open
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        self.format = format_info.lock().unwrap().clone();

        log::info!(
            "Dual audio capture started: {} Hz, {} channels, {} bits",
            self.format.sample_rate,
            self.format.channels,
            self.format.bits_per_sample
        );
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
            handle.await.map_err(|e| {
                AppError::AudioCapture(format!("Failed to stop capture thread: {}", e))
            })?;
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

    fn get_current_level(&self) -> f32 {
        *self.current_level.lock().unwrap()
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
        assert_eq!(format.channels, 1); // Placeholder before capture
        assert_eq!(format.bits_per_sample, 16); // Placeholder before capture
    }

    #[tokio::test]
    async fn test_list_devices() {
        let capture = PulseAudioCapture::new();

        // In CI environments, PulseAudio may not be available
        // Skip the test gracefully if we can't connect
        match capture.list_devices().await {
            Ok(devices) => {
                // If PulseAudio is available, ensure we get at least the default device
                assert!(!devices.is_empty(), "Should have at least one audio device");
            }
            Err(e) => {
                // Skip test if PulseAudio is not available (common in CI)
                println!("Skipping test - PulseAudio not available: {}", e);
                // Don't fail the test, just skip it
            }
        }
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
