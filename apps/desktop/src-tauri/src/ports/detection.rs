/// Participant detection port trait
///
/// Defines the interface for detecting meeting participants from running meeting applications.
/// Platform-specific implementations in adapters/detection/
use crate::domain::models::Platform;
use crate::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Information about a detected meeting window
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedMeeting {
    /// The meeting platform (Teams, Zoom, Meet)
    pub platform: Platform,
    /// Window title if available
    pub window_title: Option<String>,
    /// Process ID of the meeting application
    pub process_id: u32,
    /// Window handle (platform-specific identifier)
    pub window_handle: u64,
}

/// Information about a detected participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedParticipant {
    /// Participant's display name as shown in the meeting
    pub name: String,
    /// Whether this participant is the current user (self)
    pub is_self: bool,
    /// Whether the participant is currently speaking (if detectable)
    pub is_speaking: Option<bool>,
    /// Whether the participant has their camera on (if detectable)
    pub has_video: Option<bool>,
    /// Whether the participant is muted (if detectable)
    pub is_muted: Option<bool>,
}

/// Result of participant detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// The meeting where participants were detected
    pub meeting: DetectedMeeting,
    /// List of detected participants
    pub participants: Vec<DetectedParticipant>,
    /// Detection method used
    pub method: DetectionMethod,
    /// Confidence score (0.0 to 1.0) - higher means more reliable detection
    pub confidence: f32,
    /// Any warnings or notes about the detection
    pub warnings: Vec<String>,
}

/// Method used for participant detection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DetectionMethod {
    /// Windows UI Automation API
    UiAutomation,
    /// Linux AT-SPI (Assistive Technology Service Provider Interface)
    AtSpi,
    /// Screen capture with OCR (fallback method)
    ScreenCapture,
    /// Manual entry by user
    Manual,
}

impl std::fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DetectionMethod::UiAutomation => write!(f, "ui_automation"),
            DetectionMethod::AtSpi => write!(f, "at_spi"),
            DetectionMethod::ScreenCapture => write!(f, "screen_capture"),
            DetectionMethod::Manual => write!(f, "manual"),
        }
    }
}

/// Configuration for participant detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionConfig {
    /// Target platform to detect (None = auto-detect)
    pub target_platform: Option<Platform>,
    /// Whether to include the current user in results
    pub include_self: bool,
    /// Timeout in milliseconds for detection operations
    pub timeout_ms: u64,
    /// Whether to attempt OCR fallback if accessibility APIs fail
    pub use_ocr_fallback: bool,
}

impl Default for DetectionConfig {
    fn default() -> Self {
        Self {
            target_platform: None,
            include_self: true,
            timeout_ms: 5000,
            use_ocr_fallback: false,
        }
    }
}

/// Port trait for participant detection functionality
#[async_trait]
pub trait ParticipantDetectorPort: Send + Sync {
    /// Lists currently running meeting applications
    ///
    /// Scans for active windows from supported meeting platforms (Teams, Zoom, Google Meet).
    async fn list_active_meetings(&self) -> Result<Vec<DetectedMeeting>>;

    /// Detects participants in a specific meeting window
    ///
    /// Uses accessibility APIs to extract participant information from the meeting UI.
    ///
    /// # Arguments
    /// * `meeting` - The meeting window to scan
    /// * `config` - Detection configuration options
    async fn detect_participants(
        &self,
        meeting: &DetectedMeeting,
        config: &DetectionConfig,
    ) -> Result<DetectionResult>;

    /// Auto-detects participants from the first available meeting
    ///
    /// Convenience method that finds an active meeting and detects participants.
    async fn auto_detect(&self, config: &DetectionConfig) -> Result<Option<DetectionResult>>;

    /// Gets the detection method used by this implementation
    fn detection_method(&self) -> DetectionMethod;

    /// Checks if this detector is available on the current platform
    fn is_available(&self) -> bool;
}
