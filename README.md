# Meet Scribe

A bot-free desktop meeting assistant for Windows and Linux that captures, transcribes, and generates insights from Teams, Zoom, and Google Meet meetings.

## Overview

Meet Scribe uses system-level audio capture (no bots joining meetings) to record, transcribe with speaker diarization, and generate AI-powered insights from your meetings. All data stays local on your machine.

**Key Features:**
- 🎤 Bot-free audio capture (WASAPI on Windows, PulseAudio on Linux)
- 🗣️ Speaker diarization with participant linking
- 📝 AI-generated summaries, action items, and insights
- 🔐 Secure API key storage in OS keychain
- 💾 Local SQLite database (no cloud sync)
- 🔌 Pluggable ASR (AssemblyAI, Deepgram) and LLM services

## Prerequisites

- **Node.js** 18+ and npm
- **Rust** 1.70+ and Cargo
- **Platform-specific dependencies:**

  ### Windows
  - MSVC Build Tools (Visual Studio 2019+)
  - WebView2 runtime (usually pre-installed)

  ### Linux (Ubuntu/Debian)
  ```bash
  sudo apt install libwebkit2gtk-4.1-dev \
    build-essential curl wget libssl-dev \
    libgtk-3-dev libayatana-appindicator3-dev \
    librsvg2-dev libpulse-dev
  ```

## Getting Started

### 1. Clone Repository
```bash
git clone <repository-url>
cd meet-scribe
```

### 2. Install Dependencies

Navigate to the desktop app directory:
```bash
cd apps/desktop
```

Install Node.js dependencies:
```bash
npm install
```

The Rust dependencies will be downloaded automatically when you first build.

### 3. Generate Application Icons

The application requires icons to build on Windows. Generate them from the included SVG:

```bash
npx @tauri-apps/cli icon app-icon.svg
```

This creates all required icon formats in `src-tauri/icons/`.

### 4. First-Time Setup

Before running the app, ensure all platform-specific prerequisites are installed (see Prerequisites section above).

Verify your setup:
```bash
npx @tauri-apps/cli info
```

This should show:
- ✔ Rust toolchain installed
- ✔ System dependencies met
- ✔ Tauri CLI ready

### 5. Run in Development Mode

Start the app in development mode with hot-reload:

```bash
npm run tauri dev
```

**What happens:**
- Vite dev server starts on http://localhost:1420
- Rust backend compiles (first time takes 5-10 minutes)
- Application window opens with the React UI
- Frontend changes hot-reload instantly
- Rust changes require app restart

**Note:** First compile downloads ~600MB of Rust crates and takes 5-10 minutes. Subsequent builds are much faster (30-60 seconds).

### 6. Configure API Services

1. Open the **Settings** page in the app
2. Enter API keys for:
   - **ASR Service**: AssemblyAI or Deepgram (for transcription with diarization)
   - **LLM Service**: OpenAI or Anthropic (for generating insights)
3. API keys are stored securely in your OS keychain

**Get API Keys:**
- [AssemblyAI](https://www.assemblyai.com/) - Speech recognition with diarization
- [Deepgram](https://deepgram.com/) - Alternative ASR service
- [OpenAI](https://platform.openai.com/) - GPT models for insights
- [Anthropic](https://console.anthropic.com/) - Claude models for insights

## Project Structure

```
meet-scribe/
├── apps/desktop/
│   ├── src/                  # React frontend
│   │   ├── pages/            # Dashboard, Settings, etc.
│   │   ├── components/       # Reusable UI components
│   │   └── types/            # TypeScript type definitions
│   ├── src-tauri/            # Rust backend
│   │   ├── src/
│   │   │   ├── domain/       # Core business models
│   │   │   ├── ports/        # Trait interfaces
│   │   │   ├── adapters/     # Platform implementations
│   │   │   └── commands/     # Tauri IPC commands
│   │   └── migrations/       # Database migrations
│   └── package.json
├── docs/                     # Documentation
└── CLAUDE.md                 # Development guide for AI assistants
```

## Architecture

Meet Scribe uses a **ports-and-adapters (hexagonal) architecture**:

- **Domain Layer**: Core business logic and models (platform-agnostic)
- **Ports**: Trait interfaces defining contracts
- **Adapters**: Platform-specific implementations (WASAPI, PulseAudio, external APIs)

See [CLAUDE.md](CLAUDE.md) for detailed architecture documentation.

## Development Workflow

### Running in Development Mode

**Standard development mode** (recommended):
```bash
cd apps/desktop
npm run tauri dev
```

This starts:
- Vite dev server with hot module replacement (HMR)
- Rust backend in debug mode
- Application window with DevTools enabled

**Frontend-only development** (faster iteration for UI work):
```bash
npm run dev
```

Opens the React app in your browser at http://localhost:1420. Backend features won't work, but great for UI/CSS development.

### Debugging

#### Frontend (React) Debugging

**In the running app:**
1. Right-click anywhere in the app
2. Select "Inspect Element" or press `Ctrl+Shift+I` (Windows/Linux)
3. Chrome DevTools opens - use Console, Network, React DevTools

**In VS Code:**
1. Install "Debugger for Chrome" extension
2. Add to `.vscode/launch.json`:
```json
{
  "type": "chrome",
  "request": "attach",
  "name": "Attach to Tauri",
  "port": 9222,
  "webRoot": "${workspaceFolder}/apps/desktop/src"
}
```
3. Start app with: `npm run tauri dev`
4. Start debugging in VS Code (F5)

#### Backend (Rust) Debugging

**Print debugging:**
```rust
println!("Debug: {:?}", variable);
eprintln!("Error: {}", error); // Appears in terminal
```

**VS Code debugging:**
1. Install "CodeLLDB" or "C/C++" extension
2. Add to `.vscode/launch.json`:
```json
{
  "type": "lldb",
  "request": "launch",
  "name": "Debug Rust Backend",
  "cargo": {
    "args": ["build", "--manifest-path=apps/desktop/src-tauri/Cargo.toml"]
  },
  "cwd": "${workspaceFolder}"
}
```
3. Set breakpoints in Rust code
4. Start debugging (F5)

**Using Rust-analyzer:**
- Install rust-analyzer VS Code extension
- Provides inline type hints, error checking, and "go to definition"

**Logging in Rust:**
```rust
// Add to Cargo.toml
log = "0.4"
env_logger = "0.11"

// In main.rs
env_logger::init();
log::info!("Application started");
log::debug!("Database query: {}", query);
log::error!("Failed: {}", error);
```

Run with logging:
```bash
RUST_LOG=debug npm run tauri dev    # Linux/macOS
$env:RUST_LOG="debug"; npm run tauri dev  # Windows PowerShell
```

### Testing

**Rust backend tests:**
```bash
cd apps/desktop/src-tauri
cargo test
```

**Run specific test:**
```bash
cargo test test_name
```

**Run with output:**
```bash
cargo test -- --nocapture
```

**Frontend tests:**
```bash
cd apps/desktop
npm test
```

### Code Quality

**Format code:**
```bash
# Rust
cd apps/desktop/src-tauri
cargo fmt

# TypeScript/React
cd apps/desktop
npm run format
```

**Lint code:**
```bash
# Rust (with warnings)
cd apps/desktop/src-tauri
cargo clippy

# Rust (strict - fails on warnings)
cargo clippy -- -D warnings

# TypeScript
cd apps/desktop
npm run lint
```

**Check Rust code without building:**
```bash
cargo check
```

## Building for Production

### Create Executable/Installer

**Build release version:**
```bash
cd apps/desktop
npm run tauri build
```

**Build process:**
1. Compiles Rust in release mode (optimized, no debug symbols)
2. Builds React app for production (minified, optimized)
3. Creates platform-specific bundles:
   - **Windows**: `.msi` installer in `src-tauri/target/release/bundle/msi/`
   - **Linux**: `.deb` and `.AppImage` in `src-tauri/target/release/bundle/deb/` and `bundle/appimage/`

**Build output locations:**
```
apps/desktop/src-tauri/target/release/
├── meet-scribe.exe           # Windows executable
├── meet-scribe               # Linux executable
└── bundle/
    ├── msi/
    │   └── Meet Scribe_0.1.0_x64_en-US.msi  # Windows installer
    ├── deb/
    │   └── meet-scribe_0.1.0_amd64.deb      # Debian package
    └── appimage/
        └── meet-scribe_0.1.0_amd64.AppImage # Universal Linux
```

### Debug Build (with symbols)

For debugging production issues:
```bash
npm run tauri build -- --debug
```

Executable location: `src-tauri/target/debug/meet-scribe.exe`

### Build Options

**Enable specific features:**
```bash
npm run tauri build -- --features "feature-name"
```

**Target specific architecture:**
```bash
# 64-bit only
npm run tauri build -- --target x86_64-pc-windows-msvc

# 32-bit (if needed)
npm run tauri build -- --target i686-pc-windows-msvc
```

### Distribution Checklist

Before distributing the executable:

1. **Test the release build:**
   ```bash
   ./src-tauri/target/release/meet-scribe.exe  # Windows
   ./src-tauri/target/release/meet-scribe      # Linux
   ```

2. **Verify all features work:**
   - Database creation
   - Settings page
   - All navigation
   - No console errors

3. **Check file size:**
   - Windows .msi: ~15-20 MB
   - Linux AppImage: ~20-25 MB

4. **Code signing** (optional but recommended):
   - Windows: Sign with Authenticode certificate
   - Linux: No signing required for AppImage

5. **Create changelog** for version

### Optimizing Build Size

If the executable is too large:

```toml
# In Cargo.toml
[profile.release]
opt-level = 'z'     # Optimize for size
lto = true          # Link-time optimization
codegen-units = 1   # Better optimization
strip = true        # Remove debug symbols
```

## Development Commands Reference

```bash
# Development
npm run tauri dev              # Full app in debug mode
npm run dev                    # Frontend only

# Building
npm run tauri build            # Production build with installer
npm run tauri build -- --debug # Debug build

# Testing
cargo test                     # Rust tests
npm test                       # Frontend tests

# Code Quality
cargo fmt                      # Format Rust
cargo clippy                   # Lint Rust
npm run format                 # Format TypeScript
npm run lint                   # Lint TypeScript

# Utilities
cargo check                    # Fast compile check
cargo clean                    # Clean build artifacts
npx @tauri-apps/cli info      # System info
npx @tauri-apps/cli icon      # Generate icons
```

## Current Implementation Status

### ✅ Phase 1: Foundation (Complete)
- [x] Project structure and build system
- [x] Database schema and migrations
- [x] Domain models and port traits
- [x] SQLite storage adapter
- [x] Basic React UI shell

### 🚧 Next Steps: Phase 2 (Audio Capture)
- [ ] Implement Windows WASAPI audio capture
- [ ] Implement Linux PulseAudio audio capture
- [ ] Audio buffer management and WAV encoding
- [ ] UI controls for audio capture

### 📋 Future Phases
- Phase 3: ASR integration (AssemblyAI, Deepgram)
- Phase 4: LLM integration (OpenAI, Anthropic)
- Phase 5: Participant detection (experimental)
- Phase 6: Polish & enhancements

## How It Works

1. **Audio Capture**: Captures desktop audio using OS-level APIs (no separate recording)
2. **Participant Detection**: Identifies participants via accessibility APIs or OCR (optional)
3. **Transcription**: Sends audio to AssemblyAI or Deepgram for speech-to-text with diarization
4. **Speaker Mapping**: Links diarized speakers ("Speaker 1", "Speaker 2") to actual participant names
5. **Insight Generation**: Sends transcript to LLM to generate summaries, action items, and key points
6. **Storage**: Saves everything locally in SQLite database

## Security & Privacy

- **API keys** are stored in OS keychain (never in database or config files)
- **Meeting data** stays local on your machine (no cloud sync by default)
- **Audio buffers** are temporary and discarded after transcription
- **No bots** join your meetings - capture happens locally

## Troubleshooting

### Common Development Issues

#### "command not found: cargo" or "command not found: npm"
**Problem:** Required tools not installed or not in PATH.

**Solution:**
```bash
# Check installations
rustc --version
cargo --version
node --version
npm --version

# Install Rust (if needed)
# Windows: Download from https://rustup.rs
# Linux: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Node.js (if needed)
# Download from https://nodejs.org/ (LTS version recommended)
```

#### Build Failures - "linker not found"
**Problem:** C++ build tools missing.

**Solution (Windows):**
1. Install Visual Studio Build Tools 2019 or 2022
2. During installation, select "Desktop development with C++"
3. Restart terminal after installation

**Solution (Linux):**
```bash
sudo apt install build-essential
```

#### "icon.ico not found" Build Error
**Problem:** Icons not generated.

**Solution:**
```bash
cd apps/desktop
npx @tauri-apps/cli icon app-icon.svg
```

#### Rust Compilation is Slow
**Problem:** First compilation downloads 600+ MB of dependencies.

**Solutions:**
- **Normal:** First build takes 5-10 minutes - this is expected
- **Speed up future builds:**
  ```bash
  # Install sccache (optional)
  cargo install sccache
  export RUSTC_WRAPPER=sccache  # Add to shell profile
  ```
- **Check progress:** Rust compilation shows progress in terminal

#### Hot Reload Not Working
**Problem:** Frontend changes don't appear without restart.

**Solution:**
1. Check Vite dev server is running: http://localhost:1420
2. Check browser console for errors
3. Hard refresh: `Ctrl+Shift+R` (Windows/Linux)
4. Restart dev server: Stop `npm run tauri dev` and restart

#### Database Errors
**Problem:** Migration failed or corrupt database.

**Solution (Development):**
```bash
# Delete database and restart
# Windows
del "%APPDATA%\com.beehyv.meet-scribe\meet-scribe.db"

# Linux
rm ~/.local/share/com.beehyv.meet-scribe/meet-scribe.db

# Database will be recreated on next app start
```

#### "Port 1420 already in use"
**Problem:** Vite dev server port conflict.

**Solution:**
```bash
# Find and kill process using port 1420
# Windows
netstat -ano | findstr :1420
taskkill /PID <PID> /F

# Linux
lsof -i :1420
kill <PID>

# Or change port in vite.config.ts
```

#### WebView2 Runtime Error (Windows)
**Problem:** WebView2 not installed.

**Solution:**
Download and install: https://developer.microsoft.com/en-us/microsoft-edge/webview2/

#### "permission denied" Errors (Linux)
**Problem:** Missing execution permissions.

**Solution:**
```bash
chmod +x src-tauri/target/debug/meet-scribe
chmod +x src-tauri/target/release/meet-scribe
```

#### Cargo Build Fails with "blocking waiting for file lock"
**Problem:** Another Cargo process is running.

**Solution:**
```bash
# Wait for other process to finish, or:
rm ~/.cargo/.package-cache
```

### Debugging Build Issues

#### Get Detailed Error Information

**Rust build errors:**
```bash
cd apps/desktop/src-tauri
cargo build --verbose 2>&1 | tee build.log
# Check build.log for full error details
```

**Check system setup:**
```bash
cd apps/desktop
npx @tauri-apps/cli info
```

This shows:
- Installed versions of all tools
- Missing dependencies
- Configuration issues

#### Clean Build
If all else fails, clean everything and rebuild:

```bash
# Clean Rust
cd apps/desktop/src-tauri
cargo clean

# Clean Node modules
cd apps/desktop
rm -rf node_modules package-lock.json
npm install

# Rebuild
npm run tauri build
```

### Runtime Issues

#### Application Won't Start
**Check:**
1. Database directory exists and is writable
2. No antivirus blocking the executable
3. Run from terminal to see error messages:
   ```bash
   ./src-tauri/target/release/meet-scribe.exe
   ```

#### Database Location

Find your database file:
- **Windows:** `%APPDATA%\com.beehyv.meet-scribe\meet-scribe.db`
- **Linux:** `~/.local/share/com.beehyv.meet-scribe/meet-scribe.db`

View with SQLite browser:
```bash
# Install SQLite
# Windows: Download from https://www.sqlite.org/download.html
# Linux: sudo apt install sqlite3

# Open database
sqlite3 path/to/meet-scribe.db
.tables  # List tables
.schema  # View schema
```

#### Audio Capture Issues (Phase 2+)
- Verify OS permissions for microphone/screen recording
- Ensure meeting app is running
- Check audio device selection in Settings
- Test system audio is working in other apps

### Getting Help

If you encounter issues not covered here:

1. **Check existing issues:** [GitHub Issues](https://github.com/yourusername/meet-scribe/issues)
2. **Search Tauri docs:** https://tauri.app/v2/
3. **Create detailed bug report** with:
   - Output of `npx @tauri-apps/cli info`
   - Full error message
   - Steps to reproduce
   - Operating system version

## Contributing

This project follows a ports-and-adapters architecture. When adding new features:

1. Define the port (trait interface) in `src-tauri/src/ports/`
2. Implement the adapter in `src-tauri/src/adapters/`
3. Add Tauri commands in `src-tauri/src/commands/`
4. Build UI in `src/pages/` and `src/components/`
5. Write doc comments for all public functions
6. Add TODOs for deferred work

See [CLAUDE.md](CLAUDE.md) for detailed development guidelines.

## License

[Add your license here]

## Support

For issues and questions, please [open an issue](https://github.com/yourusername/meet-scribe/issues) on GitHub.
