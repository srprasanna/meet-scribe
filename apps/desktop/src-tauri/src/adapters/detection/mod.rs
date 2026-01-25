//! Participant detection adapters
//!
//! Platform-specific implementations for detecting meeting participants
//! using accessibility APIs (Windows UI Automation, Linux AT-SPI).

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "windows")]
pub use windows::UiAutomationDetector;

#[cfg(target_os = "linux")]
pub use linux::AtSpiDetector;

use crate::error::Result;
use crate::ports::detection::{DetectionConfig, DetectionResult, ParticipantDetectorPort};

/// Platform-specific type alias for the participant detector
#[cfg(target_os = "windows")]
pub type ParticipantDetector = UiAutomationDetector;

#[cfg(target_os = "linux")]
pub type ParticipantDetector = AtSpiDetector;

/// Creates a new platform-specific participant detector
pub fn create_detector() -> ParticipantDetector {
    ParticipantDetector::new()
}

/// Auto-detects participants from any running meeting
///
/// Convenience function that creates a detector and performs auto-detection.
pub async fn auto_detect_participants(
    config: Option<DetectionConfig>,
) -> Result<Option<DetectionResult>> {
    let detector = create_detector();
    let config = config.unwrap_or_default();
    detector.auto_detect(&config).await
}
