# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Meet-scribe is a bot-free desktop meeting assistant built with Rust + Tauri (React + Vite frontend) and SQLite. It captures system-level audio/video streams from Teams, Zoom, and Google Meet without joining as a bot or requiring consent from other participants (since recording happens locally on the user's machine).

**Key principle**: No bots, no meeting disruption - use existing A/V streams from the desktop.

## Core Architecture

### Technology Stack
- **Backend**: Rust + Tauri v2 (async with tokio)
- **Frontend**: React + Vite + TypeScript
- **Database**: SQLite via rusqlite with migrations
- **Audio Capture**: Platform-specific (WASAPI for Windows, PulseAudio/PipeWire for Linux)
- **Transcription**: AssemblyAI and Deepgram (both support diarization)
- **AI Processing**: Pluggable LLM services (user-configurable in UI)
- **Secret Storage**: OS keychain via keyring crate

### Key Rust Crates
- `tauri` v2.x - cross-platform desktop framework
- `tokio` - async runtime
- `rusqlite` - SQLite driver
- `serde` + `serde_json` - serialization
- `thiserror` / `anyhow` - error handling
- `keyring` - OS keychain access (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- `reqwest` - HTTP client for external APIs
- **Platform-specific** (Windows/Linux priority):
  - `windows` - WASAPI loopback audio capture (Windows)
  - `libpulse-binding` - PulseAudio monitor sources (Linux)
  - `screenshots` or `scrap` - screen capture for participant detection
  - `hound` - WAV file encoding for audio

### Architectural Pattern: Ports & Adapters

**Domain Layer** (core business logic):
- Models: `Meeting`, `Transcript`, `Participant`, `Insight`, `ServiceConfig`
- Use cases: `CaptureMeeting`, `TranscribeAudio`, `GenerateInsights`, `LinkParticipants`

**Ports** (trait interfaces):
- `AudioCapturePort` - capture system audio streams
- `ParticipantDetectorPort` - extract participant information
- `TranscriptionServicePort` - ASR interface
- `LLMServicePort` - AI analysis interface
- `StoragePort` - database operations

**Adapters** (implementations):
- Audio: Windows (WASAPI) and Linux (PulseAudio) implementations
- ASR Services: AssemblyAI and Deepgram adapters (both support diarization)
- LLM Services: OpenAI, Anthropic, and other provider adapters
- Storage: SQLite adapter with rusqlite

### High-Level Flow

1. **Stream Capture**: Desktop app monitors and captures system audio/video streams when Teams/Zoom/Google Meet is active
2. **Participant Detection**: Extract meeting participant information via accessibility APIs or OCR
3. **Audio Processing**: Send audio to user-selected ASR service (AssemblyAI or Deepgram) for transcription
4. **Diarization**: Speaker diarization with participant linking (map "Speaker 1" to actual participant names)
   - Both AssemblyAI and Deepgram provide built-in speaker diarization
   - User configures API keys via Settings UI
5. **AI Analysis**: Send transcript to user-configured LLM for:
   - Meeting summary
   - Action items extraction
   - Key insights generation
6. **Storage**: Save all data (transcript, participants, insights) to local SQLite database

### Key Design Decisions

**No Bot/Sidecar Approach**:
- Capture A/V streams at OS level, not through video platform APIs
- Use loopback/monitor audio devices where possible to minimize recording indicators
- No bot joining the meeting
- ⚠️ **Note**: Some OS permission indicators may still appear (screen recording on macOS, mic access on Windows)

**User-Configured Services (Bring Your Own API Keys)**:
- **ASR Services**: AssemblyAI and Deepgram (both with diarization support)
- **LLM Services**: User selects provider (OpenAI, Anthropic, etc.) and enters API key in Settings UI
- Users configure API keys via Settings page in the application
- API keys stored securely in OS keychain (never in SQLite or config files)
- Service selection and settings (model, language, etc.) stored in SQLite
- UI allows switching between providers without losing previous configurations

**Participant Linking**:
- Extract participant roster from meeting app UI via:
  - Accessibility APIs (preferred on macOS/Windows)
  - OCR on participant panel (fallback, fragile to UI changes)
- Map diarized speakers to actual participant names
- Allow manual correction in UI if auto-mapping fails
- TODO: Voice fingerprinting for auto-matching across meetings

**Function Design**:
- Keep functions small and focused
- Add Rust doc comments (`///`) for all public functions
- Write minimal happy-path tests where feasible
- Add `// TODO:` comments for deferred work

## Platform-Specific Integration

**Platform Priority**: Windows and Linux are prioritized. macOS support is deferred to later phases.

### Windows (Priority 1)
- **Audio**: WASAPI loopback (via `windows` crate) - captures desktop audio without mic indicator
- **Permissions**: May still require microphone permission in some cases
- **Participant Detection**: UI Automation API or screen capture + OCR
- **Build**: MSVC toolchain required
- **Status**: Primary development platform

### Linux (Priority 1)
- **Audio**: PulseAudio monitor sources (via `libpulse-binding`) or PipeWire
- **Permissions**: User must grant access to audio devices
- **Participant Detection**: X11/Wayland screen capture + OCR or AT-SPI
- **Build**: AppImage or .deb package
- **Status**: Primary development platform

### macOS (Deferred)
- **Audio**: Core Audio TAP (via `coreaudio-rs`) - captures application audio
- **Permissions**: Requires screen recording permission (shows indicator in menu bar)
- **Participant Detection**: Accessibility API (preferred) or Screen Capture Kit + OCR
- **Build**: Xcode command-line tools, universal binary (x64 + ARM64)
- **Status**: Deferred to Phase 6 (implementation complexity and crate maintenance issues)

## Project Structure

```
meet-scribe/
├── apps/
│   └── desktop/
│       ├── src/                          # React + Vite frontend
│       │   ├── components/               # UI components
│       │   ├── pages/                    # Page-level components
│       │   │   ├── Dashboard.tsx
│       │   │   ├── ActiveMeeting.tsx
│       │   │   ├── MeetingHistory.tsx
│       │   │   └── Settings.tsx          # ASR/LLM API key configuration UI
│       │   ├── hooks/                    # Custom React hooks
│       │   ├── store/                    # State management (Zustand/Context)
│       │   ├── types/                    # TypeScript types
│       │   └── main.tsx
│       ├── src-tauri/                    # Rust backend
│       │   ├── src/
│       │   │   ├── domain/               # Core domain models & use cases
│       │   │   │   ├── models.rs
│       │   │   │   └── use_cases.rs
│       │   │   ├── ports/                # Trait definitions (interfaces)
│       │   │   │   ├── audio.rs
│       │   │   │   ├── detection.rs
│       │   │   │   ├── transcription.rs
│       │   │   │   ├── llm.rs
│       │   │   │   └── storage.rs
│       │   │   ├── adapters/             # Platform-specific implementations
│       │   │   │   ├── audio/
│       │   │   │   │   ├── mod.rs
│       │   │   │   │   ├── windows.rs    # WASAPI implementation (Priority 1)
│       │   │   │   │   └── linux.rs      # PulseAudio implementation (Priority 1)
│       │   │   │   ├── services/         # External API adapters
│       │   │   │   │   ├── asr/
│       │   │   │   │   │   ├── mod.rs
│       │   │   │   │   │   ├── assemblyai.rs  # Priority ASR service
│       │   │   │   │   │   └── deepgram.rs    # Priority ASR service
│       │   │   │   │   └── llm/
│       │   │   │   │       ├── mod.rs
│       │   │   │   │       ├── openai.rs
│       │   │   │   │       └── anthropic.rs
│       │   │   │   └── storage/
│       │   │   │       └── sqlite.rs
│       │   │   ├── commands/             # Tauri IPC commands
│       │   │   │   ├── meeting.rs
│       │   │   │   ├── config.rs
│       │   │   │   └── transcript.rs
│       │   │   ├── error.rs              # Error types (thiserror)
│       │   │   └── main.rs
│       │   ├── migrations/               # SQL migration files
│       │   │   ├── 001_initial.sql
│       │   │   └── ...
│       │   ├── Cargo.toml
│       │   └── tauri.conf.json
│       ├── package.json
│       ├── vite.config.ts
│       └── tsconfig.json
├── docs/                                 # Module documentation
│   ├── audio-capture.md
│   ├── participant-detection.md
│   ├── transcription-services.md
│   └── database-schema.md
├── CLAUDE.md
└── README.md
```

## Development Commands

### Initial Setup
```bash
# Install Node.js dependencies
npm install

# Install Tauri CLI
cargo install tauri-cli --version "^2.0"

# Run database migrations (after first build)
# Migrations run automatically on app startup
```

### Development
```bash
# Run in development mode (hot-reload enabled)
npm run tauri dev

# Run frontend only (for UI development)
npm run dev

# Run Rust tests
cargo test

# Run frontend tests
npm test

# Format Rust code
cargo fmt

# Lint Rust code
cargo clippy -- -D warnings

# Format TypeScript/React code
npm run lint
```

### Building
```bash
# Build for current platform
npm run tauri build

# Build with debug symbols
npm run tauri build -- --debug

# Create installer/package
npm run tauri build
# Windows: Creates .msi installer
# macOS: Creates .dmg and .app bundle
# Linux: Creates .deb and AppImage
```

### Database
```bash
# Database file location (after first run):
# Windows: %APPDATA%/com.srprasanna.meet-scribe/meet-scribe.db
# macOS: ~/Library/Application Support/com.srprasanna.meet-scribe/meet-scribe.db
# Linux: ~/.local/share/com.srprasanna.meet-scribe/meet-scribe.db

# Migrations are in: apps/desktop/src-tauri/migrations/
# They run automatically on app startup via rusqlite_migration
```

### Platform-Specific Requirements

**Windows**:
- Install MSVC Build Tools (Visual Studio 2019+)
- WebView2 runtime (usually pre-installed on Windows 10+)

**macOS**:
- Install Xcode Command Line Tools: `xcode-select --install`
- For universal binary: Set `MACOSX_DEPLOYMENT_TARGET=10.13`

**Linux**:
- Install dependencies:
  ```bash
  # Ubuntu/Debian
  sudo apt install libwebkit2gtk-4.1-dev \
    build-essential curl wget libssl-dev \
    libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libpulse-dev
  ```

## Database Schema

See [docs/database-schema.md](docs/database-schema.md) for detailed schema documentation.

### Core Tables:

**meetings**
```sql
CREATE TABLE meetings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    platform TEXT NOT NULL,  -- 'teams', 'zoom', 'meet'
    title TEXT,
    start_time INTEGER NOT NULL,  -- Unix timestamp
    end_time INTEGER,
    participant_count INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);
```

**participants**
```sql
CREATE TABLE participants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    email TEXT,
    speaker_label TEXT,  -- 'Speaker 1', 'Speaker 2', etc. from diarization
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
```

**transcripts**
```sql
CREATE TABLE transcripts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id INTEGER NOT NULL,
    participant_id INTEGER,  -- NULL if speaker not yet identified
    timestamp_ms INTEGER NOT NULL,  -- Milliseconds into meeting
    text TEXT NOT NULL,
    confidence REAL,  -- 0.0 to 1.0 from ASR
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (participant_id) REFERENCES participants(id) ON DELETE SET NULL
);
```

**insights**
```sql
CREATE TABLE insights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    meeting_id INTEGER NOT NULL,
    type TEXT NOT NULL,  -- 'summary', 'action_item', 'key_point', 'decision'
    content TEXT NOT NULL,
    metadata TEXT,  -- JSON for additional data (assignee, due_date, etc.)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);
```

**service_configs**
```sql
CREATE TABLE service_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    service_type TEXT NOT NULL,  -- 'asr' or 'llm'
    provider TEXT NOT NULL,  -- 'openai', 'anthropic', 'deepgram', etc.
    is_active BOOLEAN NOT NULL DEFAULT 0,
    settings TEXT,  -- JSON for provider-specific settings (model, language, etc.)
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    UNIQUE(service_type, provider)
);
```

**Note**: API keys are stored in OS keychain, not in the database. The keychain entry format is:
- Service: `com.srprasanna.meet-scribe`
- Account: `{service_type}_{provider}` (e.g., `asr_deepgram`, `llm_anthropic`)

## Security & Privacy

**API Key Storage**:
- API keys MUST be stored in OS keychain using the `keyring` crate
- Never store API keys in SQLite, config files, or environment variables
- Keychain service name: `com.srprasanna.meet-scribe`
- Keychain account format: `{service_type}_{provider}` (e.g., `asr_deepgram`)

**Data Storage**:
- All meeting data stays local in SQLite (no cloud sync by default)
- Database location is in OS-appropriate app data directory
- Audio buffers are temporary and discarded after transcription

**Permissions**:
- Request microphone/screen recording permissions at appropriate times
- Handle permission denials gracefully with user-friendly error messages
- Document permission requirements clearly in UI

**Audio Capture**:
- Audio stream capture is always user-initiated (manual start/stop)
- Use loopback/monitor devices where possible to minimize OS recording indicators
- Clear visual indicator in app when capture is active

## Implementation Phases

### Phase 1: Foundation ✅ (Start Here)
- [x] Initialize Tauri v2 project with React + Vite
- [ ] Set up database schema and migrations (rusqlite + rusqlite_migration)
- [ ] Implement keyring-based secret storage
- [ ] Create domain models and port traits
- [ ] Build basic React UI shell (pages, routing)

### Phase 2: Audio Capture (Windows & Linux)
- [ ] Implement `AudioCapturePort` trait
- [ ] Windows: WASAPI loopback adapter (Priority 1)
- [ ] Linux: PulseAudio monitor adapter (Priority 1)
- [ ] Audio buffer management and WAV encoding (using `hound` crate)
- [ ] Settings UI for selecting audio input device

### Phase 3: ASR Integration (AssemblyAI & Deepgram)
- [ ] Implement `TranscriptionServicePort` trait
- [ ] AssemblyAI adapter with diarization support (Priority 1)
- [ ] Deepgram adapter with diarization support (Priority 1)
- [ ] Settings UI for ASR service selection and API key entry
- [ ] Handle streaming transcription responses
- [ ] Parse diarization output and store transcripts with speaker labels

### Phase 4: LLM Integration for Insights
- [ ] Implement `LLMServicePort` trait
- [ ] OpenAI adapter (GPT-4, GPT-3.5-turbo)
- [ ] Anthropic adapter (Claude Sonnet/Opus)
- [ ] Settings UI for LLM provider selection and API key entry
- [ ] Prompt engineering for summaries, action items, key points, decisions
- [ ] Store insights in database with proper categorization

### Phase 5: Participant Detection (Experimental)
- [ ] Implement `ParticipantDetectorPort` trait
- [ ] Screen capture utilities (Windows/Linux)
- [ ] OCR integration (tesseract or cloud OCR)
- [ ] Accessibility API integration (Windows UI Automation, Linux AT-SPI)
- [ ] Manual participant mapping UI (allow users to correct speaker-to-participant assignments)

### Phase 6: Polish & Enhancement
- [ ] Real-time transcription during meetings
- [ ] Export functionality (PDF, markdown, JSON)
- [ ] Search across historical meetings
- [ ] Multi-language support (configure in ASR settings)
- [ ] Custom prompt templates for LLM analysis
- [ ] macOS support (deferred from Phase 2)

## Known Limitations & TODOs

**ASR Services**:
- AssemblyAI and Deepgram both provide excellent diarization support
- Users must provide their own API keys (configured in Settings UI)
- Diarization accuracy depends on audio quality and speaker distinctness
- TODO: Add support for adjusting diarization sensitivity in settings

**Participant Detection**:
- No public APIs available from Teams/Zoom/Google Meet
- OCR-based detection is fragile to UI changes (meeting app updates can break it)
- Accessibility APIs are more robust but may have platform-specific limitations
- Manual mapping UI allows users to correct auto-detected mappings
- TODO: Research voice fingerprinting for speaker identification across meetings

**Platform Support**:
- **Windows & Linux**: Full support (Priority 1)
- **macOS**: Deferred to Phase 6 due to Core Audio complexity and crate maintenance concerns
- TODO: Consider using screen recording + system audio as fallback on macOS when implementing

**Performance**:
- Large meeting audio files may consume significant memory
- TODO: Implement streaming/chunked upload to ASR services for long meetings
- TODO: Add progress indicators for transcription and LLM analysis

**UI/UX**:
- Settings page must clearly explain API key requirements
- Provide links to AssemblyAI and Deepgram signup pages
- Show estimated costs based on meeting duration (both services charge by usage)

## Troubleshooting

**Build Failures**:
- Ensure platform-specific dependencies are installed (see Development Commands)
- Check Rust version: `rustc --version` (recommend 1.70+)
- Check Node version: `node --version` (recommend 18+)

**Audio Capture Issues**:
- Verify OS permissions are granted
- Check that target application (Teams/Zoom/Meet) is running
- Test with simple audio playback first

**Database Migration Errors**:
- Delete database file and restart (dev only)
- Check migration SQL syntax in `src-tauri/migrations/`

**API Integration Errors**:
- Verify API keys are correctly stored in keychain
- Check API rate limits and quota
- Review service-specific documentation
