//! Audio capture adapters
//!
//! Platform-specific implementations for audio capture

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub use windows::WasapiAudioCapture;

#[cfg(target_os = "linux")]
pub use linux::PulseAudioCapture;
