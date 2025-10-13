/// Domain models for Meet Scribe
///
/// These models represent core business entities and are platform-agnostic.
use serde::{Deserialize, Serialize};

/// Represents a meeting platform
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Teams,
    Zoom,
    Meet,
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Teams => write!(f, "teams"),
            Platform::Zoom => write!(f, "zoom"),
            Platform::Meet => write!(f, "meet"),
        }
    }
}

/// Represents a meeting session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meeting {
    pub id: Option<i64>,
    pub platform: Platform,
    pub title: Option<String>,
    pub start_time: i64, // Unix timestamp
    pub end_time: Option<i64>,
    pub participant_count: Option<i32>,
    pub created_at: i64,
}

impl Meeting {
    /// Creates a new meeting instance
    pub fn new(platform: Platform, title: Option<String>) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: None,
            platform,
            title,
            start_time: now,
            end_time: None,
            participant_count: None,
            created_at: now,
        }
    }

    /// Marks the meeting as ended
    pub fn end(&mut self) {
        self.end_time = Some(chrono::Utc::now().timestamp());
    }
}

/// Represents a meeting participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Participant {
    pub id: Option<i64>,
    pub meeting_id: i64,
    pub name: String,
    pub email: Option<String>,
    pub speaker_label: Option<String>, // "Speaker 1", "Speaker 2", etc.
}

impl Participant {
    /// Creates a new participant
    pub fn new(meeting_id: i64, name: String, email: Option<String>) -> Self {
        Self {
            id: None,
            meeting_id,
            name,
            email,
            speaker_label: None,
        }
    }
}

/// Represents a transcript segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    pub id: Option<i64>,
    pub meeting_id: i64,
    pub participant_id: Option<i64>,
    pub timestamp_ms: i64, // Milliseconds into meeting
    pub text: String,
    pub confidence: Option<f32>, // 0.0 to 1.0
    pub created_at: i64,
}

impl Transcript {
    /// Creates a new transcript segment
    pub fn new(
        meeting_id: i64,
        timestamp_ms: i64,
        text: String,
        confidence: Option<f32>,
    ) -> Self {
        Self {
            id: None,
            meeting_id,
            participant_id: None,
            timestamp_ms,
            text,
            confidence,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Type of insight generated from meeting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InsightType {
    Summary,
    ActionItem,
    KeyPoint,
    Decision,
}

impl std::fmt::Display for InsightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsightType::Summary => write!(f, "summary"),
            InsightType::ActionItem => write!(f, "action_item"),
            InsightType::KeyPoint => write!(f, "key_point"),
            InsightType::Decision => write!(f, "decision"),
        }
    }
}

/// Represents an AI-generated insight from a meeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Insight {
    pub id: Option<i64>,
    pub meeting_id: i64,
    pub insight_type: InsightType,
    pub content: String,
    pub metadata: Option<String>, // JSON string for additional data
    pub created_at: i64,
}

impl Insight {
    /// Creates a new insight
    pub fn new(meeting_id: i64, insight_type: InsightType, content: String) -> Self {
        Self {
            id: None,
            meeting_id,
            insight_type,
            content,
            metadata: None,
            created_at: chrono::Utc::now().timestamp(),
        }
    }
}

/// Service configuration type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ServiceType {
    Asr, // Automatic Speech Recognition
    Llm, // Large Language Model
}

impl std::fmt::Display for ServiceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceType::Asr => write!(f, "asr"),
            ServiceType::Llm => write!(f, "llm"),
        }
    }
}

/// Represents service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub id: Option<i64>,
    pub service_type: ServiceType,
    pub provider: String, // "assemblyai", "deepgram", "openai", "anthropic", etc.
    pub is_active: bool,
    pub settings: Option<String>, // JSON string for provider-specific settings
    pub created_at: i64,
    pub updated_at: i64,
}

impl ServiceConfig {
    /// Creates a new service configuration
    pub fn new(service_type: ServiceType, provider: String) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id: None,
            service_type,
            provider,
            is_active: false,
            settings: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the active status (builder pattern)
    pub fn with_active(mut self, is_active: bool) -> Self {
        self.is_active = is_active;
        self
    }

    /// Sets the settings JSON (builder pattern)
    pub fn with_settings(mut self, settings: Option<String>) -> Self {
        self.settings = settings;
        self
    }
}
