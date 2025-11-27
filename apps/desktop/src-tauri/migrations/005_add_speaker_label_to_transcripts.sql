-- Add speaker_label to transcripts table for diarization support
-- This allows storing speaker identification directly on transcript segments
ALTER TABLE transcripts ADD COLUMN speaker_label TEXT;

CREATE INDEX IF NOT EXISTS idx_transcripts_speaker_label ON transcripts(speaker_label);
