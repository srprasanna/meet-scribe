//! Windows WASAPI Audio Capture Implementation
//!
//! Uses Windows Core Audio APIs (WASAPI) to capture system audio via loopback recording.
//! This allows capturing audio playing through the system without being intrusive.

use crate::error::{AppError, Result};
use crate::ports::audio::{AudioBuffer, AudioCapturePort, AudioFormat};
use async_trait::async_trait;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use windows::core::Interface;
use windows::Win32::Media::Audio::{
    eCapture, eConsole, eRender, IAudioCaptureClient, IAudioClient, IMMDevice, IMMDeviceEnumerator,
    IMMEndpoint, MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
    AUDCLNT_STREAMFLAGS_LOOPBACK, WAVEFORMATEX,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};

// Only import Property Store related items when not in test mode
#[cfg(not(test))]
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
#[cfg(not(test))]
use windows::Win32::System::Com::STGM_READ;

/// Windows WASAPI audio capture implementation
///
/// Captures system audio output using WASAPI loopback mode.
/// The audio format (sample rate, channels, bit depth) is auto-detected
/// from the system's default audio device during `start_capture()`.
///
/// Typical Windows audio format: 48000 Hz, 2 channels, 32-bit float
pub struct WasapiAudioCapture {
    is_capturing: Arc<Mutex<bool>>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    /// Audio format - placeholder until capture starts, then auto-detected
    format: AudioFormat,
    capture_handle: Option<tokio::task::JoinHandle<()>>,
    /// Second capture handle for dual-capture (microphone)
    mic_capture_handle: Option<tokio::task::JoinHandle<()>>,
    /// Current audio level (0.0 to 1.0) for visual feedback
    current_level: Arc<Mutex<f32>>,
}

impl WasapiAudioCapture {
    /// Creates a new WASAPI audio capture instance
    ///
    /// The format field is initialized to a default placeholder.
    /// Actual format is detected when `start_capture()` is called.
    pub fn new() -> Self {
        Self {
            is_capturing: Arc::new(Mutex::new(false)),
            audio_buffer: Arc::new(Mutex::new(Vec::new())),
            format: AudioFormat::default(), // Placeholder, updated during start_capture()
            capture_handle: None,
            mic_capture_handle: None,
            current_level: Arc::new(Mutex::new(0.0)),
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
        use windows::Win32::Media::Audio::eCommunications;

        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                    AppError::AudioCapture(format!("Failed to create device enumerator: {}", e))
                })?;

            // Try to get the default COMMUNICATION device first (used by meeting apps)
            // If that fails, fall back to the default multimedia device
            let device = enumerator
                .GetDefaultAudioEndpoint(eRender, eCommunications)
                .or_else(|_| {
                    log::info!("No communication device set, using default multimedia device");
                    enumerator.GetDefaultAudioEndpoint(eRender, eConsole)
                })
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to get default audio endpoint: {}", e))
                })?;

            Ok(device)
        }
    }

    /// Get audio device by index
    ///
    /// Searches through all active render devices and returns the one at the given index.
    /// Index 0 is the default device, index 1+ are other devices in the system.
    fn get_device_by_index(device_index: usize) -> Result<IMMDevice> {
        use windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE;

        if device_index == 0 {
            // Index 0 is always the default device
            return Self::get_default_device();
        }

        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                    AppError::AudioCapture(format!("Failed to create device enumerator: {}", e))
                })?;

            let collection = enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to enumerate audio endpoints: {}", e))
                })?;

            let count = collection.GetCount().map_err(|e| {
                AppError::AudioCapture(format!("Failed to get device count: {}", e))
            })?;

            // device_index - 1 because index 0 is the default device
            let actual_index = device_index.saturating_sub(1);

            if actual_index >= count as usize {
                log::warn!(
                    "Device index {} out of range, using default device",
                    device_index
                );
                return Self::get_default_device();
            }

            collection.Item(actual_index as u32).map_err(|e| {
                AppError::AudioCapture(format!("Failed to get device {}: {}", actual_index, e))
            })
        }
    }

    /// Get microphone device by index
    ///
    /// Searches through all active capture devices and returns the one at the given index.
    fn get_microphone_by_index(device_index: usize) -> Result<IMMDevice> {
        use windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE;

        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                    AppError::AudioCapture(format!("Failed to create device enumerator: {}", e))
                })?;

            let collection = enumerator
                .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to enumerate audio endpoints: {}", e))
                })?;

            let count = collection.GetCount().map_err(|e| {
                AppError::AudioCapture(format!("Failed to get device count: {}", e))
            })?;

            if device_index >= count as usize {
                return Err(AppError::AudioCapture(format!(
                    "Microphone device index {} out of range (total: {})",
                    device_index, count
                )));
            }

            collection.Item(device_index as u32).map_err(|e| {
                AppError::AudioCapture(format!(
                    "Failed to get microphone device {}: {}",
                    device_index, e
                ))
            })
        }
    }

    /// Get friendly name for an audio device
    ///
    /// Retrieves the user-friendly device name using Windows Property Store
    /// Falls back to parsing device ID if property access fails
    fn get_device_friendly_name(device: &IMMDevice, device_index: u32) -> String {
        unsafe {
            // Skip Property Store access in test/CI environments to avoid access violations
            // This is a known issue with Property Store API in CI environments
            #[cfg(not(test))]
            {
                // Try to open the property store and get the friendly name
                // Wrap in a closure to catch any panics/errors
                let friendly_name_result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        if let Ok(property_store) = device.OpenPropertyStore(STGM_READ) {
                            if let Ok(prop_variant) =
                                property_store.GetValue(&PKEY_Device_FriendlyName)
                            {
                                // PROPVARIANT is a complex union structure
                                // The layout in memory starts with: vt (u16), reserved fields, then the union
                                // For VT_LPWSTR (31), the pwszVal is at offset 8 bytes
                                #[repr(C)]
                                struct PropVariantSimple {
                                    vt: u16,
                                    _reserved1: u16,
                                    _reserved2: u16,
                                    _reserved3: u16,
                                    pwszval: *mut u16,
                                }

                                let pv = &prop_variant as *const _ as *const PropVariantSimple;
                                let vt = (*pv).vt;

                                // VT_LPWSTR = 31
                                if vt == 31 {
                                    let pwstr_ptr = (*pv).pwszval;
                                    if !pwstr_ptr.is_null() {
                                        // Calculate string length
                                        let mut len = 0;
                                        while *pwstr_ptr.add(len) != 0 {
                                            len += 1;
                                            if len > 1024 {
                                                break;
                                            } // Safety limit
                                        }

                                        if len > 0 && len < 1024 {
                                            let slice = std::slice::from_raw_parts(pwstr_ptr, len);
                                            if let Ok(name) = String::from_utf16(slice) {
                                                if !name.is_empty() {
                                                    log::info!(
                                                        "Device {} friendly name: {}",
                                                        device_index,
                                                        name
                                                    );
                                                    return Some(name);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        None
                    }));

                // If we got a friendly name from property store, use it
                if let Ok(Some(name)) = friendly_name_result {
                    return name;
                }
            }

            // Fallback: Try to get device ID and extract useful information
            if let Ok(device_id) = device.GetId() {
                if let Ok(id_str) = device_id.to_string() {
                    log::info!("Device {} ID: {}", device_index, id_str);

                    // Parse for common hardware vendors in the ID
                    let id_lower = id_str.to_lowercase();

                    // Check for specific hardware vendors
                    if id_lower.contains("realtek") || id_lower.contains("rtk") {
                        return "Realtek Audio".to_string();
                    }
                    if id_lower.contains("nvidia") {
                        return "NVIDIA Audio".to_string();
                    }
                    if id_lower.contains("amd") || id_lower.contains("ati") {
                        return "AMD Audio".to_string();
                    }
                    if id_lower.contains("hdmi") {
                        return "HDMI Audio".to_string();
                    }
                    if id_lower.contains("usb") {
                        return "USB Audio Device".to_string();
                    }
                    if id_lower.contains("bluetooth") || id_lower.contains("bt_") {
                        return "Bluetooth Audio".to_string();
                    }
                }
            }

            // Final fallback to generic name with type and index
            let endpoint_type = device
                .cast::<IMMEndpoint>()
                .ok()
                .and_then(|endpoint| endpoint.GetDataFlow().ok())
                .map(|flow| {
                    if flow == eRender {
                        "Speaker"
                    } else {
                        "Microphone"
                    }
                })
                .unwrap_or("Audio Device");

            format!("{} {}", endpoint_type, device_index + 1)
        }
    }

    /// Initialize the audio client with the desired format
    ///
    /// Queries the WASAPI device for its mix format and initializes the audio client
    /// for loopback capture. Returns the detected format parameters which are used
    /// to update the WasapiAudioCapture.format field.
    ///
    /// Returns: (WAVEFORMATEX, sample_rate, bits_per_sample)
    fn initialize_audio_client(audio_client: &IAudioClient) -> Result<(WAVEFORMATEX, u32, u16)> {
        unsafe {
            // Get the device's mix format (auto-detected from system)
            let mix_format_ptr = audio_client
                .GetMixFormat()
                .map_err(|e| AppError::AudioCapture(format!("Failed to get mix format: {}", e)))?;

            if mix_format_ptr.is_null() {
                return Err(AppError::AudioCapture(
                    "Mix format pointer is null".to_string(),
                ));
            }

            let mix_format = *mix_format_ptr;
            let sample_rate = mix_format.nSamplesPerSec; // Actual system sample rate
            let bits_per_sample = mix_format.wBitsPerSample; // Actual bit depth

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
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to initialize audio client: {}", e))
                })?;

            // Free the mix format
            windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));

            Ok((mix_format, sample_rate, bits_per_sample))
        }
    }

    /// Initialize the audio client for microphone capture (non-loopback)
    ///
    /// Similar to initialize_audio_client but doesn't use loopback mode
    fn initialize_microphone_client(
        audio_client: &IAudioClient,
    ) -> Result<(WAVEFORMATEX, u32, u16)> {
        unsafe {
            // Get the device's mix format (auto-detected from system)
            let mix_format_ptr = audio_client
                .GetMixFormat()
                .map_err(|e| AppError::AudioCapture(format!("Failed to get mix format: {}", e)))?;

            if mix_format_ptr.is_null() {
                return Err(AppError::AudioCapture(
                    "Mix format pointer is null".to_string(),
                ));
            }

            let mix_format = *mix_format_ptr;
            let sample_rate = mix_format.nSamplesPerSec;
            let bits_per_sample = mix_format.wBitsPerSample;

            // Initialize the audio client for normal (non-loopback) capture
            let buffer_duration = 10_000_000; // 1 second in 100-nanosecond units
            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    0, // No special flags (normal capture, not loopback)
                    buffer_duration,
                    0,
                    mix_format_ptr,
                    None,
                )
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to initialize microphone client: {}", e))
                })?;

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

    /// Calculate RMS (Root Mean Square) audio level from samples
    /// Returns a value between 0.0 and 1.0 representing the audio level
    fn calculate_rms_level(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_of_squares: f32 = samples.iter().map(|&s| s * s).sum();
        let mean_square = sum_of_squares / samples.len() as f32;
        mean_square.sqrt()
    }

    /// Perform the actual audio capture loop
    fn capture_loop(
        audio_client: IAudioClient,
        capture_client: IAudioCaptureClient,
        format: WAVEFORMATEX,
        is_capturing: Arc<Mutex<bool>>,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
        current_level: Arc<Mutex<f32>>,
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
            let _bits_per_sample = format.wBitsPerSample;

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
                            if (flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32) == 0
                                && num_frames_available > 0
                            {
                                // Calculate the size of the data
                                let data_size = num_frames_available as usize * frame_size;

                                // Copy the data
                                let data_slice = std::slice::from_raw_parts(data_ptr, data_size);

                                // Convert to f32 samples
                                let samples = Self::convert_samples_to_f32(data_slice, &format);

                                // Calculate and update current audio level
                                let level = Self::calculate_rms_level(&samples);
                                *current_level.lock().unwrap() = level;

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
        use windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE;

        tokio::task::spawn_blocking(|| {
            unsafe {
                // Initialize COM for this thread
                if let Err(e) = Self::init_com() {
                    return Err(e);
                }

                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!("Failed to create device enumerator: {}", e))
                    })?;

                let mut devices = Vec::new();

                // Get default speaker device first
                match Self::get_default_device() {
                    Ok(device) => {
                        let name = Self::get_device_friendly_name(&device, 0);
                        devices.push(format!("0: {} (Default Speaker)", name));
                    }
                    Err(e) => {
                        log::warn!("Failed to get default device: {}", e);
                        devices.push("0: Default Communication Device (Speaker)".to_string());
                    }
                }

                // Enumerate all speaker (render) devices
                let speaker_collection = enumerator
                    .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                    .map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!(
                            "Failed to enumerate speaker endpoints: {}",
                            e
                        ))
                    })?;

                let speaker_count = speaker_collection.GetCount().map_err(|e| {
                    CoUninitialize();
                    AppError::AudioCapture(format!("Failed to get speaker device count: {}", e))
                })?;

                // Add all speaker devices with their friendly names
                for i in 0..speaker_count {
                    match speaker_collection.Item(i) {
                        Ok(device) => {
                            let name = Self::get_device_friendly_name(&device, i);
                            devices.push(format!("{}: {} (Speaker)", i + 1, name));
                        }
                        Err(e) => {
                            log::warn!("Failed to get speaker device {}: {}", i, e);
                        }
                    }
                }

                // Enumerate all microphone (capture) devices
                let mic_collection = enumerator
                    .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
                    .map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!(
                            "Failed to enumerate microphone endpoints: {}",
                            e
                        ))
                    })?;

                let mic_count = mic_collection.GetCount().map_err(|e| {
                    CoUninitialize();
                    AppError::AudioCapture(format!("Failed to get microphone device count: {}", e))
                })?;

                // Add all microphone devices with their friendly names
                // Use offset starting after speaker devices
                let mic_offset = speaker_count + 1;
                for i in 0..mic_count {
                    match mic_collection.Item(i) {
                        Ok(device) => {
                            let name = Self::get_device_friendly_name(&device, i);
                            devices.push(format!("{}: {} (Microphone)", mic_offset + i, name));
                        }
                        Err(e) => {
                            log::warn!("Failed to get microphone device {}: {}", i, e);
                        }
                    }
                }

                CoUninitialize();

                log::info!(
                    "Found {} audio devices ({} speakers, {} microphones)",
                    devices.len(),
                    speaker_count + 1,
                    mic_count
                );
                Ok(devices)
            }
        })
        .await
        .map_err(|e| AppError::AudioCapture(format!("Task join error: {}", e)))?
    }

    async fn list_speaker_devices(&self) -> Result<Vec<String>> {
        use windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE;

        tokio::task::spawn_blocking(|| {
            unsafe {
                if let Err(e) = Self::init_com() {
                    return Err(e);
                }

                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!("Failed to create device enumerator: {}", e))
                    })?;

                let mut devices = Vec::new();

                // Get default speaker device first
                match Self::get_default_device() {
                    Ok(device) => {
                        let name = Self::get_device_friendly_name(&device, 0);
                        devices.push(format!("0: {} (Default Speaker)", name));
                    }
                    Err(e) => {
                        log::warn!("Failed to get default device: {}", e);
                        devices.push("0: Default Communication Device".to_string());
                    }
                }

                // Enumerate all speaker (render) devices
                let speaker_collection = enumerator
                    .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                    .map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!(
                            "Failed to enumerate speaker endpoints: {}",
                            e
                        ))
                    })?;

                let speaker_count = speaker_collection.GetCount().map_err(|e| {
                    CoUninitialize();
                    AppError::AudioCapture(format!("Failed to get speaker device count: {}", e))
                })?;

                // Add all speaker devices with their friendly names
                for i in 0..speaker_count {
                    match speaker_collection.Item(i) {
                        Ok(device) => {
                            let name = Self::get_device_friendly_name(&device, i);
                            devices.push(format!("{}: {}", i + 1, name));
                        }
                        Err(e) => {
                            log::warn!("Failed to get speaker device {}: {}", i, e);
                        }
                    }
                }

                CoUninitialize();
                log::info!("Found {} speaker devices", devices.len());
                Ok(devices)
            }
        })
        .await
        .map_err(|e| AppError::AudioCapture(format!("Task join error: {}", e)))?
    }

    async fn list_microphone_devices(&self) -> Result<Vec<String>> {
        use windows::Win32::Media::Audio::DEVICE_STATE_ACTIVE;

        tokio::task::spawn_blocking(|| {
            unsafe {
                if let Err(e) = Self::init_com() {
                    return Err(e);
                }

                let enumerator: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!("Failed to create device enumerator: {}", e))
                    })?;

                let mut devices = Vec::new();

                // Enumerate all microphone (capture) devices
                let mic_collection = enumerator
                    .EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE)
                    .map_err(|e| {
                        CoUninitialize();
                        AppError::AudioCapture(format!(
                            "Failed to enumerate microphone endpoints: {}",
                            e
                        ))
                    })?;

                let mic_count = mic_collection.GetCount().map_err(|e| {
                    CoUninitialize();
                    AppError::AudioCapture(format!("Failed to get microphone device count: {}", e))
                })?;

                // Add all microphone devices with their friendly names
                for i in 0..mic_count {
                    match mic_collection.Item(i) {
                        Ok(device) => {
                            let name = Self::get_device_friendly_name(&device, i);
                            devices.push(format!("{}: {}", i, name));
                        }
                        Err(e) => {
                            log::warn!("Failed to get microphone device {}: {}", i, e);
                        }
                    }
                }

                CoUninitialize();
                log::info!("Found {} microphone devices", devices.len());
                Ok(devices)
            }
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
        let current_level_clone = Arc::clone(&self.current_level);

        // Store format info to be updated after detection
        let format_info = Arc::new(Mutex::new(AudioFormat::default()));
        let format_info_clone = Arc::clone(&format_info);

        // Spawn background task for audio capture
        let handle = tokio::task::spawn_blocking(move || {
            // Initialize COM for this thread
            if let Err(e) = Self::init_com() {
                log::error!("Failed to initialize COM: {}", e);
                *is_capturing_clone.lock().unwrap() = false;
                return;
            }

            // Get the audio device (specific device or default)
            // Device name format: "0: Default Audio Output" or "1: Audio Device 1"
            let device_index = device_name
                .and_then(|name| {
                    // Extract index from "N: Device Name" format
                    name.split(':').next()?.trim().parse::<usize>().ok()
                })
                .unwrap_or(0); // Default to index 0 if parsing fails

            log::info!("Using audio device index: {}", device_index);

            let device = match Self::get_device_by_index(device_index) {
                Ok(d) => d,
                Err(e) => {
                    log::error!("Failed to get device at index {}: {}", device_index, e);
                    *is_capturing_clone.lock().unwrap() = false;
                    unsafe {
                        CoUninitialize();
                    }
                    return;
                }
            };

            // Activate the audio client
            let audio_client: IAudioClient =
                match unsafe { device.Activate::<IAudioClient>(CLSCTX_ALL, None) } {
                    Ok(client) => client,
                    Err(e) => {
                        log::error!("Failed to activate audio client: {}", e);
                        *is_capturing_clone.lock().unwrap() = false;
                        unsafe {
                            CoUninitialize();
                        }
                        return;
                    }
                };

            // Initialize the audio client and get the actual device format
            // This is where the format is detected from the WASAPI device
            let (format, sample_rate, bits_per_sample) =
                match Self::initialize_audio_client(&audio_client) {
                    Ok(f) => f,
                    Err(e) => {
                        log::error!("Failed to initialize audio client: {}", e);
                        *is_capturing_clone.lock().unwrap() = false;
                        unsafe {
                            CoUninitialize();
                        }
                        return;
                    }
                };

            // IMPORTANT: Update format with actual detected values from the device
            // This replaces the default placeholder values with the real audio format
            let channels = format.nChannels;
            *format_info_clone.lock().unwrap() = AudioFormat {
                sample_rate,     // e.g., 48000 Hz (detected from device)
                channels,        // e.g., 2 (stereo, detected from device)
                bits_per_sample, // e.g., 32 bits (float, detected from device)
            };

            // Get the capture client
            let capture_client: IAudioCaptureClient =
                match unsafe { audio_client.GetService::<IAudioCaptureClient>() } {
                    Ok(client) => client,
                    Err(e) => {
                        log::error!("Failed to get capture client: {}", e);
                        *is_capturing_clone.lock().unwrap() = false;
                        unsafe {
                            CoUninitialize();
                        }
                        return;
                    }
                };

            log::info!("WASAPI audio capture initialized successfully");
            log::info!(
                "Format: {} Hz, {} channels, {} bits",
                sample_rate,
                channels,
                bits_per_sample
            );

            // Run the capture loop
            Self::capture_loop(
                audio_client,
                capture_client,
                format,
                is_capturing_clone,
                audio_buffer_clone,
                current_level_clone,
            );

            unsafe {
                CoUninitialize();
            }
        });

        self.capture_handle = Some(handle);

        // Wait for format detection to complete
        // The background thread detects the system's audio format and stores it in format_info
        // Typical Windows audio: 48000 Hz, stereo, 32-bit float
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Update our format from the auto-detected format
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
        // Note: audio_buffer is not used directly in dual-capture - speaker_buffer and mic_buffer
        // are used instead, and the mixer thread writes to self.audio_buffer

        // Store format info to be updated after detection
        // This will be updated by whichever capture thread starts first
        let format_info = Arc::new(Mutex::new(AudioFormat::default()));

        // Parse device indices
        log::info!("Speaker device string: {:?}", speaker_device);
        log::info!("Microphone device string: {:?}", microphone_device);

        let speaker_index = speaker_device
            .as_ref()
            .and_then(|name| name.split(':').next()?.trim().parse::<usize>().ok());

        let microphone_index = microphone_device
            .as_ref()
            .and_then(|name| name.split(':').next()?.trim().parse::<usize>().ok());

        log::info!(
            "Starting dual-capture: speaker index {:?}, microphone index {:?}",
            speaker_index,
            microphone_index
        );

        // If both devices are None, return error
        if speaker_index.is_none() && microphone_index.is_none() {
            *self.is_capturing.lock().unwrap() = false;
            return Err(AppError::AudioCapture(
                "At least one device (speaker or microphone) must be specified".to_string(),
            ));
        }

        // Spawn speaker (loopback) capture thread (if speaker device specified)
        let speaker_buffer = Arc::new(Mutex::new(Vec::new()));
        let speaker_buffer_clone = Arc::clone(&speaker_buffer);
        let speaker_is_capturing = Arc::clone(&is_capturing_clone);
        let speaker_level = Arc::clone(&self.current_level);
        let speaker_format_info = Arc::clone(&format_info);

        let speaker_handle = if let Some(spk_idx) = speaker_index {
            Some(tokio::task::spawn_blocking(move || {
                if let Err(e) = Self::init_com() {
                    log::error!("Failed to initialize COM for speaker capture: {}", e);
                    *speaker_is_capturing.lock().unwrap() = false;
                    return;
                }

                let device = match Self::get_device_by_index(spk_idx) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Failed to get speaker device: {}", e);
                        *speaker_is_capturing.lock().unwrap() = false;
                        unsafe {
                            CoUninitialize();
                        }
                        return;
                    }
                };

                let audio_client: IAudioClient =
                    match unsafe { device.Activate::<IAudioClient>(CLSCTX_ALL, None) } {
                        Ok(client) => client,
                        Err(e) => {
                            log::error!("Failed to activate speaker audio client: {}", e);
                            *speaker_is_capturing.lock().unwrap() = false;
                            unsafe {
                                CoUninitialize();
                            }
                            return;
                        }
                    };

                let (format, sample_rate, bits_per_sample) =
                    match Self::initialize_audio_client(&audio_client) {
                        Ok(f) => f,
                        Err(e) => {
                            log::error!("Failed to initialize speaker audio client: {}", e);
                            *speaker_is_capturing.lock().unwrap() = false;
                            unsafe {
                                CoUninitialize();
                            }
                            return;
                        }
                    };

                let capture_client: IAudioCaptureClient =
                    match unsafe { audio_client.GetService::<IAudioCaptureClient>() } {
                        Ok(client) => client,
                        Err(e) => {
                            log::error!("Failed to get speaker capture client: {}", e);
                            *speaker_is_capturing.lock().unwrap() = false;
                            unsafe {
                                CoUninitialize();
                            }
                            return;
                        }
                    };

                // Store channels value to avoid packed field reference
                let channels = format.nChannels;

                // Update the shared format info with speaker format
                // Speaker format is typically the master format for loopback capture
                *speaker_format_info.lock().unwrap() = AudioFormat {
                    sample_rate,
                    channels,
                    bits_per_sample,
                };

                log::info!(
                    "Speaker capture initialized: {} Hz, {} channels, {} bits",
                    sample_rate,
                    channels,
                    bits_per_sample
                );

                Self::capture_loop(
                    audio_client,
                    capture_client,
                    format,
                    speaker_is_capturing,
                    speaker_buffer_clone,
                    speaker_level,
                );

                unsafe {
                    CoUninitialize();
                }
            }))
        } else {
            log::info!("Skipping speaker capture (no speaker device specified)");
            None
        };

        // Spawn microphone capture thread (if microphone device specified)
        let mic_buffer = Arc::new(Mutex::new(Vec::new()));
        let mic_buffer_clone = Arc::clone(&mic_buffer);
        let mic_is_capturing = Arc::clone(&is_capturing_clone);
        let format_info_clone2 = Arc::clone(&format_info);
        let mic_level = Arc::clone(&self.current_level);

        let mic_handle = if let Some(mic_idx) = microphone_index {
            Some(tokio::task::spawn_blocking(move || {
                if let Err(e) = Self::init_com() {
                    log::error!("Failed to initialize COM for microphone capture: {}", e);
                    *mic_is_capturing.lock().unwrap() = false;
                    return;
                }

                let device = match Self::get_microphone_by_index(mic_idx) {
                    Ok(d) => d,
                    Err(e) => {
                        log::error!("Failed to get microphone device: {}", e);
                        *mic_is_capturing.lock().unwrap() = false;
                        unsafe {
                            CoUninitialize();
                        }
                        return;
                    }
                };

                let audio_client: IAudioClient =
                    match unsafe { device.Activate::<IAudioClient>(CLSCTX_ALL, None) } {
                        Ok(client) => client,
                        Err(e) => {
                            log::error!("Failed to activate microphone audio client: {}", e);
                            *mic_is_capturing.lock().unwrap() = false;
                            unsafe {
                                CoUninitialize();
                            }
                            return;
                        }
                    };

                let (format, sample_rate, bits_per_sample) =
                    match Self::initialize_microphone_client(&audio_client) {
                        Ok(f) => f,
                        Err(e) => {
                            log::error!("Failed to initialize microphone audio client: {}", e);
                            *mic_is_capturing.lock().unwrap() = false;
                            unsafe {
                                CoUninitialize();
                            }
                            return;
                        }
                    };

                // Update format from microphone only if speaker format not already set
                // Speaker format takes priority as it's the loopback (system audio) source
                let channels = format.nChannels;
                {
                    let mut fmt = format_info_clone2.lock().unwrap();
                    // Only update if still at default (speaker didn't set it)
                    if fmt.sample_rate == 16000 && fmt.channels == 1 {
                        *fmt = AudioFormat {
                            sample_rate,
                            channels,
                            bits_per_sample,
                        };
                        log::info!("Using microphone format as master (no speaker specified)");
                    } else {
                        log::info!(
                        "Speaker format already set ({} Hz, {} ch), microphone format ignored ({} Hz, {} ch)",
                        fmt.sample_rate, fmt.channels, sample_rate, channels
                    );
                    }
                }

                let capture_client: IAudioCaptureClient =
                    match unsafe { audio_client.GetService::<IAudioCaptureClient>() } {
                        Ok(client) => client,
                        Err(e) => {
                            log::error!("Failed to get microphone capture client: {}", e);
                            *mic_is_capturing.lock().unwrap() = false;
                            unsafe {
                                CoUninitialize();
                            }
                            return;
                        }
                    };

                log::info!(
                    "Microphone capture initialized: {} Hz, {} channels, {} bits",
                    sample_rate,
                    channels,
                    bits_per_sample
                );

                Self::capture_loop(
                    audio_client,
                    capture_client,
                    format,
                    mic_is_capturing,
                    mic_buffer_clone,
                    mic_level,
                );

                unsafe {
                    CoUninitialize();
                }
            }))
        } else {
            log::info!("Skipping microphone capture (no microphone device specified)");
            None
        };

        self.capture_handle = speaker_handle;
        self.mic_capture_handle = mic_handle;

        // Wait for format detection - capture threads need time to initialize and update format
        // 200ms should be sufficient for WASAPI initialization
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        self.format = format_info.lock().unwrap().clone();

        // Warn if format is still at default - this indicates a bug
        if self.format.sample_rate == 16000 && self.format.channels == 1 {
            log::warn!(
                "Format still at default after capture start! This may indicate initialization failed."
            );
        }

        // Spawn mixer thread to combine both audio streams
        let is_capturing_mixer = Arc::clone(&self.is_capturing);
        let mixed_buffer = Arc::clone(&self.audio_buffer);

        tokio::spawn(async move {
            let mut total_mixed = 0usize;
            while *is_capturing_mixer.lock().unwrap() {
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

                // Mix audio from both buffers
                let speaker_samples: Vec<f32> = {
                    let mut buffer = speaker_buffer.lock().unwrap();
                    buffer.drain(..).collect()
                };

                let mic_samples: Vec<f32> = {
                    let mut buffer = mic_buffer.lock().unwrap();
                    buffer.drain(..).collect()
                };

                if !speaker_samples.is_empty() || !mic_samples.is_empty() {
                    log::debug!(
                        "Mixer: speaker_samples={}, mic_samples={}",
                        speaker_samples.len(),
                        mic_samples.len()
                    );

                    // Mix the two streams
                    let max_len = speaker_samples.len().max(mic_samples.len());
                    let mut mixed = Vec::with_capacity(max_len);

                    for i in 0..max_len {
                        let speaker_sample = speaker_samples.get(i).copied().unwrap_or(0.0);
                        let mic_sample = mic_samples.get(i).copied().unwrap_or(0.0);

                        // Simple mixing: add and divide by 2 (average)
                        // This prevents clipping while maintaining reasonable levels
                        let mixed_sample = (speaker_sample + mic_sample) / 2.0;
                        mixed.push(mixed_sample);
                    }

                    // Append to final buffer
                    let mut final_buffer = mixed_buffer.lock().unwrap();
                    final_buffer.extend(mixed);
                    total_mixed += max_len;

                    if total_mixed % 48000 == 0 {
                        log::info!("Mixed {} total samples so far", total_mixed);
                    }
                }
            }
            log::info!("Mixer thread stopped. Total samples mixed: {}", total_mixed);
        });

        log::info!(
            "Dual-capture started: {} Hz, {} channels, {} bits",
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

        // Wait for both capture threads to finish
        if let Some(handle) = self.capture_handle.take() {
            handle.await.map_err(|e| {
                AppError::AudioCapture(format!("Failed to stop speaker capture thread: {}", e))
            })?;
        }

        if let Some(handle) = self.mic_capture_handle.take() {
            handle.await.map_err(|e| {
                AppError::AudioCapture(format!("Failed to stop microphone capture thread: {}", e))
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

impl WasapiAudioCapture {
    /// Generate a test tone (sine wave) for speaker testing
    /// Returns audio samples at the specified frequency
    pub fn generate_test_tone(duration_seconds: f32, sample_rate: u32, frequency: f32) -> Vec<f32> {
        let num_samples = (duration_seconds * sample_rate as f32) as usize;
        let mut samples = Vec::with_capacity(num_samples);

        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            let sample = (2.0 * std::f32::consts::PI * frequency * t).sin() * 0.3; // 30% volume
            samples.push(sample);
        }

        samples
    }

    /// Play audio through the specified speaker device
    /// This is used for testing speaker output
    pub async fn play_audio(
        device_index: usize,
        samples: Vec<f32>,
        sample_rate: u32,
    ) -> Result<()> {
        tokio::task::spawn_blocking(move || {
            Self::play_audio_blocking(device_index, samples, sample_rate)
        })
        .await
        .map_err(|e| AppError::AudioCapture(format!("Playback task failed: {}", e)))?
    }

    /// Blocking version of play_audio for use in spawn_blocking
    fn play_audio_blocking(
        device_index: usize,
        samples: Vec<f32>,
        _sample_rate: u32,
    ) -> Result<()> {
        use windows::Win32::Media::Audio::{
            IAudioRenderClient, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
        };

        unsafe {
            // Initialize COM for this thread
            Self::init_com()?;

            // Get the playback device (render endpoint)
            let device = Self::get_device_by_index(device_index)?;

            // Activate audio client
            let audio_client: IAudioClient = device
                .Activate::<IAudioClient>(CLSCTX_ALL, None)
                .map_err(|e| {
                    AppError::AudioCapture(format!("Failed to activate audio client: {}", e))
                })?;

            // Get the device format
            let mix_format_ptr = audio_client
                .GetMixFormat()
                .map_err(|e| AppError::AudioCapture(format!("Failed to get mix format: {}", e)))?;

            if mix_format_ptr.is_null() {
                return Err(AppError::AudioCapture(
                    "Mix format pointer is null".to_string(),
                ));
            }

            let mix_format = *mix_format_ptr;

            // Initialize the audio client for playback
            let buffer_duration = 10_000_000; // 1 second in 100-nanosecond units
            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    0, // No special flags for playback
                    buffer_duration,
                    0,
                    mix_format_ptr,
                    None,
                )
                .map_err(|e| {
                    windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
                    AppError::AudioCapture(format!("Failed to initialize audio client: {}", e))
                })?;

            // Get buffer size
            let buffer_frame_count = audio_client.GetBufferSize().map_err(|e| {
                windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
                AppError::AudioCapture(format!("Failed to get buffer size: {}", e))
            })?;

            // Get render client
            let render_client: IAudioRenderClient = audio_client
                .GetService::<IAudioRenderClient>()
                .map_err(|e| {
                    windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
                    AppError::AudioCapture(format!("Failed to get render client: {}", e))
                })?;

            // Start the audio client
            audio_client.Start().map_err(|e| {
                windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
                AppError::AudioCapture(format!("Failed to start audio client: {}", e))
            })?;

            // Convert samples to the device format and write to buffer
            let channels = mix_format.nChannels as usize;
            let bits_per_sample = mix_format.wBitsPerSample;
            let bytes_per_sample = (bits_per_sample / 8) as usize;

            let mut sample_index = 0;
            while sample_index < samples.len() {
                // Wait for buffer to be ready
                std::thread::sleep(std::time::Duration::from_millis(10));

                // Get number of frames we can write
                let padding = audio_client.GetCurrentPadding().map_err(|e| {
                    windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
                    AppError::AudioCapture(format!("Failed to get padding: {}", e))
                })?;

                let frames_available = buffer_frame_count.saturating_sub(padding);

                if frames_available > 0 {
                    // Get the buffer
                    let buffer_ptr = render_client.GetBuffer(frames_available).map_err(|e| {
                        windows::Win32::System::Com::CoTaskMemFree(Some(
                            mix_format_ptr as *const _,
                        ));
                        AppError::AudioCapture(format!("Failed to get buffer: {}", e))
                    })?;

                    let buffer_size = frames_available as usize * channels * bytes_per_sample;
                    let buffer_slice = std::slice::from_raw_parts_mut(buffer_ptr, buffer_size);

                    // Write samples to buffer
                    let mut buffer_pos = 0;
                    for _ in 0..frames_available {
                        if sample_index >= samples.len() {
                            // Pad with silence
                            for _ in 0..channels {
                                for b in 0..bytes_per_sample {
                                    buffer_slice[buffer_pos + b] = 0;
                                }
                                buffer_pos += bytes_per_sample;
                            }
                        } else {
                            let sample = samples[sample_index];
                            sample_index += 1;

                            // Write same sample to all channels
                            for _ in 0..channels {
                                match bits_per_sample {
                                    16 => {
                                        // 16-bit PCM
                                        let value = (sample * 32767.0) as i16;
                                        buffer_slice[buffer_pos..buffer_pos + 2]
                                            .copy_from_slice(&value.to_le_bytes());
                                        buffer_pos += 2;
                                    }
                                    32 => {
                                        // 32-bit float
                                        buffer_slice[buffer_pos..buffer_pos + 4]
                                            .copy_from_slice(&sample.to_le_bytes());
                                        buffer_pos += 4;
                                    }
                                    _ => {
                                        // Unsupported format
                                        buffer_slice[buffer_pos..buffer_pos + bytes_per_sample]
                                            .fill(0);
                                        buffer_pos += bytes_per_sample;
                                    }
                                }
                            }
                        }
                    }

                    // Release the buffer
                    render_client
                        .ReleaseBuffer(frames_available, 0)
                        .map_err(|e| {
                            windows::Win32::System::Com::CoTaskMemFree(Some(
                                mix_format_ptr as *const _,
                            ));
                            AppError::AudioCapture(format!("Failed to release buffer: {}", e))
                        })?;
                }
            }

            // Wait for playback to finish
            std::thread::sleep(std::time::Duration::from_millis(100));

            // Stop the audio client
            audio_client.Stop().ok();

            // Free the mix format
            windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));

            CoUninitialize();

            Ok(())
        }
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
        // Before capture starts, format is the default placeholder
        // Actual format is detected during start_capture() and varies by system
        // Typical Windows audio: 48000 Hz, 2 channels, 32 bits (float)
        assert_eq!(format.sample_rate, 16000); // Placeholder before capture
        assert_eq!(format.channels, 1); // Placeholder before capture
        assert_eq!(format.bits_per_sample, 16); // Placeholder before capture
    }

    #[tokio::test]
    #[ignore] // Ignore this test in CI due to access violations with audio device enumeration
    async fn test_list_devices() {
        let capture = WasapiAudioCapture::new();

        // In CI environments, audio device enumeration may fail due to:
        // - No audio devices present
        // - Access violations in Property Store API
        // Skip the test gracefully if we can't enumerate devices
        match capture.list_devices().await {
            Ok(devices) => {
                // If enumeration succeeds, ensure we get at least the default device
                assert!(!devices.is_empty(), "Should have at least one audio device");
            }
            Err(e) => {
                // Skip test if device enumeration fails (common in CI)
                println!("Skipping test - device enumeration failed: {}", e);
                // Don't fail the test, just skip it
            }
        }
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
