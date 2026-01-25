-- Add language column to meetings table for per-meeting transcription language setting
ALTER TABLE meetings ADD COLUMN language TEXT DEFAULT 'en';
