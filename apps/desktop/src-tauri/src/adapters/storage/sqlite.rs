/// SQLite storage adapter
///
/// Implements StoragePort for SQLite database operations.
use crate::domain::models::{
    Insight, InsightType, Meeting, Participant, Platform, ServiceConfig, ServiceType, Transcript,
};
use crate::error::{AppError, Result};
use crate::ports::storage::StoragePort;
use async_trait::async_trait;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// SQLite storage implementation
pub struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    /// Create a new SQLite storage with the given database path
    pub fn new(db_path: PathBuf) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Run database migrations
    pub fn run_migrations(&self) -> Result<()> {
        use rusqlite_migration::{Migrations, M};

        let migrations = Migrations::new(vec![M::up(include_str!(
            "../../../migrations/001_initial.sql"
        ))]);

        let mut conn = self.conn.lock().unwrap();
        migrations
            .to_latest(&mut conn)
            .map_err(|e| AppError::Database(rusqlite::Error::ToSqlConversionFailure(Box::new(e))))?;

        Ok(())
    }
}

#[async_trait]
impl StoragePort for SqliteStorage {
    async fn create_meeting(&self, meeting: &Meeting) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO meetings (platform, title, start_time, end_time, participant_count, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                meeting.platform.to_string(),
                meeting.title,
                meeting.start_time,
                meeting.end_time,
                meeting.participant_count,
                meeting.created_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_meeting(&self, id: i64) -> Result<Option<Meeting>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, platform, title, start_time, end_time, participant_count, created_at
             FROM meetings WHERE id = ?1",
        )?;

        let mut rows = stmt.query(params![id])?;

        if let Some(row) = rows.next()? {
            let platform_str: String = row.get(1)?;
            let platform = match platform_str.as_str() {
                "teams" => Platform::Teams,
                "zoom" => Platform::Zoom,
                "meet" => Platform::Meet,
                _ => return Err(AppError::Database(rusqlite::Error::InvalidQuery)),
            };

            Ok(Some(Meeting {
                id: Some(row.get(0)?),
                platform,
                title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                participant_count: row.get(5)?,
                created_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_meetings(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<Meeting>> {
        let conn = self.conn.lock().unwrap();
        let query = format!(
            "SELECT id, platform, title, start_time, end_time, participant_count, created_at
             FROM meetings ORDER BY start_time DESC LIMIT ?1 OFFSET ?2"
        );

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map(params![limit.unwrap_or(100), offset.unwrap_or(0)], |row| {
            let platform_str: String = row.get(1)?;
            let platform = match platform_str.as_str() {
                "teams" => Platform::Teams,
                "zoom" => Platform::Zoom,
                "meet" => Platform::Meet,
                _ => Platform::Teams, // Default fallback
            };

            Ok(Meeting {
                id: Some(row.get(0)?),
                platform,
                title: row.get(2)?,
                start_time: row.get(3)?,
                end_time: row.get(4)?,
                participant_count: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut meetings = Vec::new();
        for meeting_result in rows {
            meetings.push(meeting_result?);
        }

        Ok(meetings)
    }

    async fn update_meeting(&self, meeting: &Meeting) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE meetings SET platform = ?1, title = ?2, start_time = ?3, end_time = ?4,
             participant_count = ?5 WHERE id = ?6",
            params![
                meeting.platform.to_string(),
                meeting.title,
                meeting.start_time,
                meeting.end_time,
                meeting.participant_count,
                meeting.id,
            ],
        )?;
        Ok(())
    }

    async fn delete_meeting(&self, id: i64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM meetings WHERE id = ?1", params![id])?;
        Ok(())
    }

    async fn create_participant(&self, participant: &Participant) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO participants (meeting_id, name, email, speaker_label)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                participant.meeting_id,
                participant.name,
                participant.email,
                participant.speaker_label,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_participants(&self, meeting_id: i64) -> Result<Vec<Participant>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, name, email, speaker_label
             FROM participants WHERE meeting_id = ?1",
        )?;

        let rows = stmt.query_map(params![meeting_id], |row| {
            Ok(Participant {
                id: Some(row.get(0)?),
                meeting_id: row.get(1)?,
                name: row.get(2)?,
                email: row.get(3)?,
                speaker_label: row.get(4)?,
            })
        })?;

        let mut participants = Vec::new();
        for participant_result in rows {
            participants.push(participant_result?);
        }

        Ok(participants)
    }

    async fn update_participant(&self, participant: &Participant) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE participants SET name = ?1, email = ?2, speaker_label = ?3 WHERE id = ?4",
            params![
                participant.name,
                participant.email,
                participant.speaker_label,
                participant.id,
            ],
        )?;
        Ok(())
    }

    async fn create_transcript(&self, transcript: &Transcript) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO transcripts (meeting_id, participant_id, timestamp_ms, text, confidence, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                transcript.meeting_id,
                transcript.participant_id,
                transcript.timestamp_ms,
                transcript.text,
                transcript.confidence,
                transcript.created_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_transcripts(&self, meeting_id: i64) -> Result<Vec<Transcript>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, participant_id, timestamp_ms, text, confidence, created_at
             FROM transcripts WHERE meeting_id = ?1 ORDER BY timestamp_ms",
        )?;

        let rows = stmt.query_map(params![meeting_id], |row| {
            Ok(Transcript {
                id: Some(row.get(0)?),
                meeting_id: row.get(1)?,
                participant_id: row.get(2)?,
                timestamp_ms: row.get(3)?,
                text: row.get(4)?,
                confidence: row.get(5)?,
                created_at: row.get(6)?,
            })
        })?;

        let mut transcripts = Vec::new();
        for transcript_result in rows {
            transcripts.push(transcript_result?);
        }

        Ok(transcripts)
    }

    async fn create_transcripts_batch(&self, transcripts: &[Transcript]) -> Result<Vec<i64>> {
        let conn = self.conn.lock().unwrap();
        let mut ids = Vec::new();

        let tx = conn.unchecked_transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO transcripts (meeting_id, participant_id, timestamp_ms, text, confidence, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?;

            for transcript in transcripts {
                stmt.execute(params![
                    transcript.meeting_id,
                    transcript.participant_id,
                    transcript.timestamp_ms,
                    transcript.text,
                    transcript.confidence,
                    transcript.created_at,
                ])?;
                ids.push(tx.last_insert_rowid());
            }
        }
        tx.commit()?;

        Ok(ids)
    }

    async fn create_insight(&self, insight: &Insight) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO insights (meeting_id, type, content, metadata, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                insight.meeting_id,
                insight.insight_type.to_string(),
                insight.content,
                insight.metadata,
                insight.created_at,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    async fn get_insights(&self, meeting_id: i64) -> Result<Vec<Insight>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, meeting_id, type, content, metadata, created_at
             FROM insights WHERE meeting_id = ?1",
        )?;

        let rows = stmt.query_map(params![meeting_id], |row| {
            let type_str: String = row.get(2)?;
            let insight_type = match type_str.as_str() {
                "summary" => InsightType::Summary,
                "action_item" => InsightType::ActionItem,
                "key_point" => InsightType::KeyPoint,
                "decision" => InsightType::Decision,
                _ => InsightType::Summary,
            };

            Ok(Insight {
                id: Some(row.get(0)?),
                meeting_id: row.get(1)?,
                insight_type,
                content: row.get(3)?,
                metadata: row.get(4)?,
                created_at: row.get(5)?,
            })
        })?;

        let mut insights = Vec::new();
        for insight_result in rows {
            insights.push(insight_result?);
        }

        Ok(insights)
    }

    async fn save_service_config(&self, config: &ServiceConfig) -> Result<i64> {
        let conn = self.conn.lock().unwrap();

        // Try to update first
        let rows_updated = conn.execute(
            "UPDATE service_configs SET is_active = ?1, settings = ?2, updated_at = ?3
             WHERE service_type = ?4 AND provider = ?5",
            params![
                config.is_active,
                config.settings,
                chrono::Utc::now().timestamp(),
                config.service_type.to_string(),
                config.provider,
            ],
        )?;

        if rows_updated == 0 {
            // Insert if doesn't exist
            conn.execute(
                "INSERT INTO service_configs (service_type, provider, is_active, settings, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    config.service_type.to_string(),
                    config.provider,
                    config.is_active,
                    config.settings,
                    config.created_at,
                    config.updated_at,
                ],
            )?;
            Ok(conn.last_insert_rowid())
        } else {
            // Return the ID of the updated row
            let mut stmt = conn.prepare(
                "SELECT id FROM service_configs WHERE service_type = ?1 AND provider = ?2",
            )?;
            let id: i64 = stmt.query_row(
                params![config.service_type.to_string(), config.provider],
                |row| row.get(0),
            )?;
            Ok(id)
        }
    }

    async fn get_service_config(
        &self,
        service_type: &str,
        provider: &str,
    ) -> Result<Option<ServiceConfig>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, service_type, provider, is_active, settings, created_at, updated_at
             FROM service_configs WHERE service_type = ?1 AND provider = ?2",
        )?;

        let mut rows = stmt.query(params![service_type, provider])?;

        if let Some(row) = rows.next()? {
            let service_type_str: String = row.get(1)?;
            let service_type = match service_type_str.as_str() {
                "asr" => ServiceType::Asr,
                "llm" => ServiceType::Llm,
                _ => return Err(AppError::Database(rusqlite::Error::InvalidQuery)),
            };

            Ok(Some(ServiceConfig {
                id: Some(row.get(0)?),
                service_type,
                provider: row.get(2)?,
                is_active: row.get(3)?,
                settings: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_active_service_config(&self, service_type: &str) -> Result<Option<ServiceConfig>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, service_type, provider, is_active, settings, created_at, updated_at
             FROM service_configs WHERE service_type = ?1 AND is_active = 1 LIMIT 1",
        )?;

        let mut rows = stmt.query(params![service_type])?;

        if let Some(row) = rows.next()? {
            let service_type_str: String = row.get(1)?;
            let service_type = match service_type_str.as_str() {
                "asr" => ServiceType::Asr,
                "llm" => ServiceType::Llm,
                _ => return Err(AppError::Database(rusqlite::Error::InvalidQuery)),
            };

            Ok(Some(ServiceConfig {
                id: Some(row.get(0)?),
                service_type,
                provider: row.get(2)?,
                is_active: row.get(3)?,
                settings: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_service_configs(&self) -> Result<Vec<ServiceConfig>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, service_type, provider, is_active, settings, created_at, updated_at
             FROM service_configs ORDER BY service_type, provider",
        )?;

        let rows = stmt.query_map([], |row| {
            let service_type_str: String = row.get(1)?;
            let service_type = match service_type_str.as_str() {
                "asr" => ServiceType::Asr,
                "llm" => ServiceType::Llm,
                _ => ServiceType::Asr,
            };

            Ok(ServiceConfig {
                id: Some(row.get(0)?),
                service_type,
                provider: row.get(2)?,
                is_active: row.get(3)?,
                settings: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
            })
        })?;

        let mut configs = Vec::new();
        for config_result in rows {
            configs.push(config_result?);
        }

        Ok(configs)
    }
}
