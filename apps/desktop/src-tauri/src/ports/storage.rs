/// Storage port trait
///
/// Defines the interface for database operations.
/// Implementation: SQLite adapter
use crate::domain::models::{Insight, Meeting, Participant, ServiceConfig, Transcript};
use crate::error::Result;
use async_trait::async_trait;

/// Port trait for storage operations
#[async_trait]
pub trait StoragePort: Send + Sync {
    // Meeting operations
    /// Create a new meeting
    async fn create_meeting(&self, meeting: &Meeting) -> Result<i64>;

    /// Get a meeting by ID
    async fn get_meeting(&self, id: i64) -> Result<Option<Meeting>>;

    /// List all meetings, optionally filtered
    async fn list_meetings(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<Meeting>>;

    /// Update a meeting
    async fn update_meeting(&self, meeting: &Meeting) -> Result<()>;

    /// Delete a meeting and all related data
    async fn delete_meeting(&self, id: i64) -> Result<()>;

    // Participant operations
    /// Create a new participant
    async fn create_participant(&self, participant: &Participant) -> Result<i64>;

    /// Get participants for a meeting
    async fn get_participants(&self, meeting_id: i64) -> Result<Vec<Participant>>;

    /// Update a participant
    async fn update_participant(&self, participant: &Participant) -> Result<()>;

    /// Delete a participant by ID
    async fn delete_participant(&self, id: i64) -> Result<()>;

    // Transcript operations
    /// Create a new transcript segment
    async fn create_transcript(&self, transcript: &Transcript) -> Result<i64>;

    /// Get transcripts for a meeting
    async fn get_transcripts(&self, meeting_id: i64) -> Result<Vec<Transcript>>;

    /// Batch insert transcripts (more efficient for large meetings)
    async fn create_transcripts_batch(&self, transcripts: &[Transcript]) -> Result<Vec<i64>>;

    /// Update a transcript
    async fn update_transcript(&self, transcript: &Transcript) -> Result<()>;

    /// Batch update transcripts by speaker label (more efficient for participant linking)
    async fn update_transcripts_by_speaker_label(
        &self,
        meeting_id: i64,
        speaker_label: &str,
        participant_id: i64,
    ) -> Result<usize>;

    /// Delete all transcripts for a meeting
    async fn delete_transcripts(&self, meeting_id: i64) -> Result<()>;

    // Insight operations
    /// Create a new insight
    async fn create_insight(&self, insight: &Insight) -> Result<i64>;

    /// Get insights for a meeting
    async fn get_insights(&self, meeting_id: i64) -> Result<Vec<Insight>>;

    /// Delete all insights for a meeting
    async fn delete_insights(&self, meeting_id: i64) -> Result<()>;

    // Service config operations
    /// Save or update service configuration
    async fn save_service_config(&self, config: &ServiceConfig) -> Result<i64>;

    /// Get service configuration
    async fn get_service_config(
        &self,
        service_type: &str,
        provider: &str,
    ) -> Result<Option<ServiceConfig>>;

    /// Get active service configuration for a service type
    async fn get_active_service_config(&self, service_type: &str) -> Result<Option<ServiceConfig>>;

    /// List all service configurations
    async fn list_service_configs(&self) -> Result<Vec<ServiceConfig>>;
}
