-- FTS5 Full-Text Search Migration
-- Creates FTS5 virtual tables and triggers for searching across transcripts, insights, and meetings

-- FTS5 virtual table for transcripts
-- Indexes text content and speaker labels for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS transcripts_fts USING fts5(
    text,
    speaker_label,
    meeting_id UNINDEXED,
    timestamp_ms UNINDEXED,
    content='transcripts',
    content_rowid='id'
);

-- Populate FTS table with existing transcript data
INSERT INTO transcripts_fts(rowid, text, speaker_label, meeting_id, timestamp_ms)
SELECT id, text, speaker_label, meeting_id, timestamp_ms
FROM transcripts;

-- Triggers to keep transcripts_fts in sync with transcripts table
CREATE TRIGGER IF NOT EXISTS transcripts_ai AFTER INSERT ON transcripts BEGIN
    INSERT INTO transcripts_fts(rowid, text, speaker_label, meeting_id, timestamp_ms)
    VALUES (new.id, new.text, new.speaker_label, new.meeting_id, new.timestamp_ms);
END;

CREATE TRIGGER IF NOT EXISTS transcripts_ad AFTER DELETE ON transcripts BEGIN
    DELETE FROM transcripts_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS transcripts_au AFTER UPDATE ON transcripts BEGIN
    UPDATE transcripts_fts
    SET text = new.text,
        speaker_label = new.speaker_label,
        meeting_id = new.meeting_id,
        timestamp_ms = new.timestamp_ms
    WHERE rowid = new.id;
END;

-- FTS5 virtual table for insights
-- Indexes insight content and type for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS insights_fts USING fts5(
    content,
    type,
    meeting_id UNINDEXED,
    content='insights',
    content_rowid='id'
);

-- Populate FTS table with existing insights data
INSERT INTO insights_fts(rowid, content, type, meeting_id)
SELECT id, content, type, meeting_id
FROM insights;

-- Triggers to keep insights_fts in sync with insights table
CREATE TRIGGER IF NOT EXISTS insights_ai AFTER INSERT ON insights BEGIN
    INSERT INTO insights_fts(rowid, content, type, meeting_id)
    VALUES (new.id, new.content, new.type, new.meeting_id);
END;

CREATE TRIGGER IF NOT EXISTS insights_ad AFTER DELETE ON insights BEGIN
    DELETE FROM insights_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS insights_au AFTER UPDATE ON insights BEGIN
    UPDATE insights_fts
    SET content = new.content,
        type = new.type,
        meeting_id = new.meeting_id
    WHERE rowid = new.id;
END;

-- FTS5 virtual table for meetings
-- Indexes meeting titles and platform for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS meetings_fts USING fts5(
    title,
    platform,
    content='meetings',
    content_rowid='id'
);

-- Populate FTS table with existing meetings data (only meetings with titles)
INSERT INTO meetings_fts(rowid, title, platform)
SELECT id, COALESCE(title, ''), platform
FROM meetings;

-- Triggers to keep meetings_fts in sync with meetings table
CREATE TRIGGER IF NOT EXISTS meetings_ai AFTER INSERT ON meetings BEGIN
    INSERT INTO meetings_fts(rowid, title, platform)
    VALUES (new.id, COALESCE(new.title, ''), new.platform);
END;

CREATE TRIGGER IF NOT EXISTS meetings_ad AFTER DELETE ON meetings BEGIN
    DELETE FROM meetings_fts WHERE rowid = old.id;
END;

CREATE TRIGGER IF NOT EXISTS meetings_au AFTER UPDATE ON meetings BEGIN
    UPDATE meetings_fts
    SET title = COALESCE(new.title, ''),
        platform = new.platform
    WHERE rowid = new.id;
END;
