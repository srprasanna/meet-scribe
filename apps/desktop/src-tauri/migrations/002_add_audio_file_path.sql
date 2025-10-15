-- Add audio file path to meetings table
-- This allows tracking of recorded audio files for each meeting

ALTER TABLE meetings ADD COLUMN audio_file_path TEXT;

-- Index for searching meetings with audio files
CREATE INDEX idx_meetings_audio_file ON meetings(audio_file_path) WHERE audio_file_path IS NOT NULL;
