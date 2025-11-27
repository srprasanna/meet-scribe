/// TypeScript type definitions for Meet Scribe

export type Platform = "teams" | "zoom" | "meet";

export interface Meeting {
  id?: number;
  platform: Platform;
  title?: string;
  start_time: number;
  end_time?: number;
  participant_count?: number;
  created_at: number;
}

export interface Participant {
  id?: number;
  meeting_id: number;
  name: string;
  email?: string;
  speaker_label?: string;
}

export interface Transcript {
  id?: number;
  meeting_id: number;
  participant_id?: number;
  speaker_label?: string;
  timestamp_ms: number;
  text: string;
  confidence?: number;
  created_at: number;
}

export type InsightType = "summary" | "action_item" | "key_point" | "decision";

export interface Insight {
  id?: number;
  meeting_id: number;
  insight_type: InsightType;
  content: string;
  metadata?: string;
  created_at: number;
}

export type ServiceType = "asr" | "llm";

export interface ServiceConfig {
  id?: number;
  service_type: ServiceType;
  provider: string;
  is_active: boolean;
  settings?: string;
  created_at: number;
  updated_at: number;
}

// Transcription types
export interface TranscriptionConfig {
  enable_diarization: boolean;
  num_speakers?: number;
  language?: string;
  additional_settings?: Record<string, unknown>;
}

export interface TranscriptionSegment {
  text: string;
  start_ms: number;
  end_ms: number;
  speaker_label?: string;
  confidence?: number;
}

export interface TranscriptionResult {
  text: string;
  segments: TranscriptionSegment[];
  confidence?: number;
}
