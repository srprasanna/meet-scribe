# Database Schema

This document describes the SQLite database schema for Meet Scribe.

## Overview

The database uses SQLite with the following design principles:
- All timestamps are stored as Unix timestamps (seconds since epoch)
- Foreign keys are enabled for referential integrity
- Cascading deletes are used where appropriate
- Indexes are added for common query patterns

## Tables

### meetings

Stores information about meeting sessions.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| platform | TEXT | Meeting platform: 'teams', 'zoom', or 'meet' |
| title | TEXT | Optional meeting title |
| start_time | INTEGER | Unix timestamp when meeting started |
| end_time | INTEGER | Unix timestamp when meeting ended (NULL if ongoing) |
| participant_count | INTEGER | Number of participants (optional) |
| created_at | INTEGER | Record creation timestamp |

**Indexes:**
- `idx_meetings_start_time` on `start_time`
- `idx_meetings_platform` on `platform`

### participants

Stores meeting participants and their speaker labels.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| meeting_id | INTEGER | Foreign key to meetings table |
| name | TEXT | Participant name |
| email | TEXT | Participant email (optional) |
| speaker_label | TEXT | Diarization label: 'Speaker 1', 'Speaker 2', etc. |

**Foreign Keys:**
- `meeting_id` → `meetings(id)` ON DELETE CASCADE

**Indexes:**
- `idx_participants_meeting_id` on `meeting_id`
- `idx_participants_speaker_label` on `speaker_label`

### transcripts

Stores transcribed text segments with speaker attribution.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| meeting_id | INTEGER | Foreign key to meetings table |
| participant_id | INTEGER | Foreign key to participants table (NULL if unidentified) |
| timestamp_ms | INTEGER | Milliseconds into the meeting |
| text | TEXT | Transcribed text segment |
| confidence | REAL | Transcription confidence (0.0 to 1.0) |
| created_at | INTEGER | Record creation timestamp |

**Foreign Keys:**
- `meeting_id` → `meetings(id)` ON DELETE CASCADE
- `participant_id` → `participants(id)` ON DELETE SET NULL

**Indexes:**
- `idx_transcripts_meeting_id` on `meeting_id`
- `idx_transcripts_timestamp` on `timestamp_ms`

### insights

Stores AI-generated insights from meetings.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| meeting_id | INTEGER | Foreign key to meetings table |
| type | TEXT | Insight type: 'summary', 'action_item', 'key_point', or 'decision' |
| content | TEXT | The insight content |
| metadata | TEXT | Optional JSON metadata (assignee, due_date, etc.) |
| created_at | INTEGER | Record creation timestamp |

**Foreign Keys:**
- `meeting_id` → `meetings(id)` ON DELETE CASCADE

**Indexes:**
- `idx_insights_meeting_id` on `meeting_id`
- `idx_insights_type` on `type`

### service_configs

Stores configuration for ASR and LLM services.

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY | Auto-incrementing ID |
| service_type | TEXT | Service type: 'asr' or 'llm' |
| provider | TEXT | Provider name: 'assemblyai', 'deepgram', 'openai', 'anthropic', etc. |
| is_active | BOOLEAN | Whether this service is currently active |
| settings | TEXT | JSON string with provider-specific settings |
| created_at | INTEGER | Record creation timestamp |
| updated_at | INTEGER | Record update timestamp |

**Constraints:**
- `UNIQUE(service_type, provider)` - Only one config per service type and provider combination

**Indexes:**
- `idx_service_configs_type` on `service_type`
- `idx_service_configs_active` on `is_active`

## Security Note

**API keys are NOT stored in the database.** They are stored securely in the operating system's keychain:
- Windows: Credential Manager
- Linux: Secret Service (libsecret)
- macOS: Keychain (when macOS support is added)

Keychain entry format:
- **Service:** `com.beehyv.meet-scribe`
- **Account:** `{service_type}_{provider}` (e.g., `asr_deepgram`, `llm_anthropic`)

## Migration Strategy

Migrations are handled by `rusqlite_migration` and run automatically on app startup. Migration files are located in `apps/desktop/src-tauri/migrations/` and are numbered sequentially (e.g., `001_initial.sql`, `002_add_feature.sql`).

To add a new migration:
1. Create a new `.sql` file with the next sequential number
2. Add the migration to the `Migrations::new()` call in `adapters/storage/sqlite.rs`
3. The migration will run automatically on the next app launch
