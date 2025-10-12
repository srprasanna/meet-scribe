-- Initial database schema for Meet Scribe
-- This migration creates all core tables

-- Meetings table
CREATE TABLE IF NOT EXISTS meetings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    platform TEXT NOT NULL CHECK(platform IN ('teams', 'zoom', 'meet')),
    title TEXT,
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    participant_count INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX idx_meetings_start_time ON meetings(start_time);
CREATE INDEX idx_meetings_platform ON meetings(platform);

-- Participants table
CREATE TABLE IF NOT EXISTS participants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    speaker_label TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX idx_participants_meeting_id ON participants(meeting_id);
CREATE INDEX idx_participants_speaker_label ON participants(speaker_label);

-- Transcripts table
CREATE TABLE IF NOT EXISTS transcripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id INTEGER NOT NULL,
    participant_id INTEGER,
    timestamp_ms INTEGER NOT NULL,
    text TEXT NOT NULL,
    confidence REAL CHECK(confidence IS NULL OR (confidence >= 0.0 AND confidence <= 1.0)),
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (participant_id) REFERENCES participants(id) ON DELETE SET NULL
);

CREATE INDEX idx_transcripts_meeting_id ON transcripts(meeting_id);
CREATE INDEX idx_transcripts_timestamp ON transcripts(timestamp_ms);

-- Insights table
CREATE TABLE IF NOT EXISTS insights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id INTEGER NOT NULL,
    type TEXT NOT NULL CHECK(type IN ('summary', 'action_item', 'key_point', 'decision')),
    content TEXT NOT NULL,
    metadata TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX idx_insights_meeting_id ON insights(meeting_id);
CREATE INDEX idx_insights_type ON insights(type);

-- Service configurations table
CREATE TABLE IF NOT EXISTS service_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_type TEXT NOT NULL CHECK(service_type IN ('asr', 'llm')),
    provider TEXT NOT NULL,
    is_active BOOLEAN NOT NULL DEFAULT 0,
    settings TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(service_type, provider)
);

CREATE INDEX idx_service_configs_type ON service_configs(service_type);
CREATE INDEX idx_service_configs_active ON service_configs(is_active);
