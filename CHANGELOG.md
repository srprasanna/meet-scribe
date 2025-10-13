# Changelog

All notable changes to Meet Scribe will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project setup with Tauri v2 + React + Vite
- SQLite database with automatic migrations
- Basic application structure with ports-and-adapters architecture
- Dashboard, ActiveMeeting, MeetingHistory, and Settings UI pages
- Database health check and version commands

### Changed
- N/A

### Fixed
- N/A

### Security
- API keys stored securely in OS keychain (Windows Credential Manager, Linux Secret Service)

## [0.1.0] - 2025-01-XX (Initial Release)

### Added
- Cross-platform desktop application (Windows and Linux)
- System-level audio capture support
  - Windows: WASAPI loopback
  - Linux: PulseAudio monitor sources
- Meeting transcription with speaker diarization
  - AssemblyAI integration
  - Deepgram integration
- AI-powered meeting insights
  - Meeting summaries
  - Action item extraction
  - Key points identification
- Local SQLite database for meeting data
- User-configurable ASR and LLM services
- Settings UI for API key management
- Meeting history and transcript viewing

---

## Release Template

Copy this template for new releases:

```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Changes in existing functionality

### Deprecated
- Soon-to-be removed features

### Removed
- Removed features

### Fixed
- Bug fixes

### Security
- Security fixes and improvements
```

---

[Unreleased]: https://github.com/srprasanna/meet-scribe/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/srprasanna/meet-scribe/releases/tag/v0.1.0
