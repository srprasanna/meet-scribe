//! Mock implementations for testing

use crate::domain::models::{Insight, Meeting, Participant, ServiceConfig, Transcript};
use crate::error::Result;
use crate::ports::storage::StoragePort;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mock storage implementation for testing
#[derive(Clone, Default)]
pub struct MockStorage {
    meetings: Arc<Mutex<HashMap<i64, Meeting>>>,
    participants: Arc<Mutex<HashMap<i64, Participant>>>,
    transcripts: Arc<Mutex<Vec<Transcript>>>,
    insights: Arc<Mutex<Vec<Insight>>>,
    service_configs: Arc<Mutex<Vec<ServiceConfig>>>,
    next_id: Arc<Mutex<i64>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> i64 {
        let mut id = self.next_id.lock().unwrap();
        *id += 1;
        *id
    }
}

#[async_trait]
impl StoragePort for MockStorage {
    async fn create_meeting(&self, meeting: &Meeting) -> Result<i64> {
        let id = self.next_id();
        let mut m = meeting.clone();
        m.id = Some(id);
        self.meetings.lock().unwrap().insert(id, m);
        Ok(id)
    }

    async fn get_meeting(&self, id: i64) -> Result<Option<Meeting>> {
        Ok(self.meetings.lock().unwrap().get(&id).cloned())
    }

    async fn list_meetings(&self, limit: Option<i32>, offset: Option<i32>) -> Result<Vec<Meeting>> {
        let meetings = self.meetings.lock().unwrap();
        let mut list: Vec<_> = meetings.values().cloned().collect();
        list.sort_by_key(|m| -m.start_time);

        let offset = offset.unwrap_or(0) as usize;
        let limit = limit.map(|l| l as usize);

        let result = list.into_iter().skip(offset);
        if let Some(limit) = limit {
            Ok(result.take(limit).collect())
        } else {
            Ok(result.collect())
        }
    }

    async fn update_meeting(&self, meeting: &Meeting) -> Result<()> {
        if let Some(id) = meeting.id {
            self.meetings.lock().unwrap().insert(id, meeting.clone());
        }
        Ok(())
    }

    async fn delete_meeting(&self, id: i64) -> Result<()> {
        self.meetings.lock().unwrap().remove(&id);
        Ok(())
    }

    async fn create_participant(&self, participant: &Participant) -> Result<i64> {
        let id = self.next_id();
        let mut p = participant.clone();
        p.id = Some(id);
        self.participants.lock().unwrap().insert(id, p);
        Ok(id)
    }

    async fn get_participants(&self, meeting_id: i64) -> Result<Vec<Participant>> {
        Ok(self
            .participants
            .lock()
            .unwrap()
            .values()
            .filter(|p| p.meeting_id == meeting_id)
            .cloned()
            .collect())
    }

    async fn update_participant(&self, participant: &Participant) -> Result<()> {
        if let Some(id) = participant.id {
            self.participants
                .lock()
                .unwrap()
                .insert(id, participant.clone());
        }
        Ok(())
    }

    async fn delete_participant(&self, id: i64) -> Result<()> {
        self.participants.lock().unwrap().remove(&id);
        Ok(())
    }

    async fn create_transcript(&self, transcript: &Transcript) -> Result<i64> {
        let id = self.next_id();
        let mut t = transcript.clone();
        t.id = Some(id);
        self.transcripts.lock().unwrap().push(t);
        Ok(id)
    }

    async fn get_transcripts(&self, meeting_id: i64) -> Result<Vec<Transcript>> {
        Ok(self
            .transcripts
            .lock()
            .unwrap()
            .iter()
            .filter(|t| t.meeting_id == meeting_id)
            .cloned()
            .collect())
    }

    async fn create_transcripts_batch(&self, transcripts: &[Transcript]) -> Result<Vec<i64>> {
        let mut ids = Vec::new();
        for transcript in transcripts {
            let id = self.create_transcript(transcript).await?;
            ids.push(id);
        }
        Ok(ids)
    }

    async fn delete_transcripts(&self, meeting_id: i64) -> Result<()> {
        self.transcripts
            .lock()
            .unwrap()
            .retain(|t| t.meeting_id != meeting_id);
        Ok(())
    }

    async fn update_transcript(&self, transcript: &Transcript) -> Result<()> {
        if let Some(id) = transcript.id {
            let mut transcripts = self.transcripts.lock().unwrap();
            if let Some(existing) = transcripts.iter_mut().find(|t| t.id == Some(id)) {
                *existing = transcript.clone();
            }
        }
        Ok(())
    }

    async fn update_transcripts_by_speaker_label(
        &self,
        meeting_id: i64,
        speaker_label: &str,
        participant_id: i64,
    ) -> Result<usize> {
        let mut transcripts = self.transcripts.lock().unwrap();
        let mut count = 0;
        for transcript in transcripts.iter_mut() {
            if transcript.meeting_id == meeting_id
                && transcript.speaker_label.as_deref() == Some(speaker_label)
            {
                transcript.participant_id = Some(participant_id);
                count += 1;
            }
        }
        Ok(count)
    }

    async fn create_insight(&self, insight: &Insight) -> Result<i64> {
        let id = self.next_id();
        let mut i = insight.clone();
        i.id = Some(id);
        self.insights.lock().unwrap().push(i);
        Ok(id)
    }

    async fn get_insights(&self, meeting_id: i64) -> Result<Vec<Insight>> {
        Ok(self
            .insights
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.meeting_id == meeting_id)
            .cloned()
            .collect())
    }

    async fn delete_insights(&self, meeting_id: i64) -> Result<()> {
        self.insights
            .lock()
            .unwrap()
            .retain(|i| i.meeting_id != meeting_id);
        Ok(())
    }

    async fn save_service_config(&self, config: &ServiceConfig) -> Result<i64> {
        let mut configs = self.service_configs.lock().unwrap();

        // Find existing config
        if let Some(existing) = configs.iter_mut().find(|c| {
            format!("{:?}", c.service_type).to_lowercase()
                == format!("{:?}", config.service_type).to_lowercase()
                && c.provider == config.provider
        }) {
            // Update existing
            existing.is_active = config.is_active;
            existing.settings = config.settings.clone();
            existing.updated_at = chrono::Utc::now().timestamp();
            return Ok(existing.id.unwrap_or(1));
        }

        // Create new
        let id = self.next_id();
        let mut c = config.clone();
        c.id = Some(id);
        configs.push(c);
        Ok(id)
    }

    async fn get_service_config(
        &self,
        service_type: &str,
        provider: &str,
    ) -> Result<Option<ServiceConfig>> {
        Ok(self
            .service_configs
            .lock()
            .unwrap()
            .iter()
            .find(|c| {
                format!("{:?}", c.service_type).to_lowercase() == service_type.to_lowercase()
                    && c.provider == provider
            })
            .cloned())
    }

    async fn get_active_service_config(&self, service_type: &str) -> Result<Option<ServiceConfig>> {
        Ok(self
            .service_configs
            .lock()
            .unwrap()
            .iter()
            .find(|c| {
                format!("{:?}", c.service_type).to_lowercase() == service_type.to_lowercase()
                    && c.is_active
            })
            .cloned())
    }

    async fn list_service_configs(&self) -> Result<Vec<ServiceConfig>> {
        Ok(self.service_configs.lock().unwrap().clone())
    }
}
