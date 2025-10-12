# Developer Guide - Meet Scribe

Quick reference for developers working on Meet Scribe.

## Setup (5 minutes)

```bash
# Clone and install
git clone <repo-url>
cd meet-scribe/apps/desktop
npm install

# Generate icons (Windows requirement)
npx @tauri-apps/cli icon app-icon.svg

# Verify setup
npx @tauri-apps/cli info
```

## Daily Development

### Start Development Mode
```bash
cd apps/desktop
npm run tauri dev
```

**What happens:**
- Vite dev server: http://localhost:1420
- Rust backend compiles in debug mode
- App window opens with DevTools
- **First run:** 5-10 min (downloads dependencies)
- **Subsequent:** 30-60 sec

### Debugging

**Frontend (React):**
- Press `Ctrl+Shift+I` in app
- Chrome DevTools available
- Check Console for errors

**Backend (Rust):**
```rust
// Quick debug
println!("Value: {:?}", variable);
eprintln!("Error: {}", error);  // Red in terminal

// With logging
log::debug!("Starting process");
log::error!("Failed: {}", e);
```

Run with logging:
```bash
# Windows PowerShell
$env:RUST_LOG="debug"; npm run tauri dev

# Linux/Git Bash
RUST_LOG=debug npm run tauri dev
```

### Making Changes

**Frontend changes:**
- Edit files in `src/`
- Hot-reload happens automatically
- No restart needed

**Rust changes:**
- Edit files in `src-tauri/src/`
- Stop app (Ctrl+C)
- Run `npm run tauri dev` again
- Compilation: ~30-60 seconds

**Database changes:**
1. Create new file: `migrations/002_your_change.sql`
2. Update `SqliteStorage::run_migrations()` in `sqlite.rs`
3. Delete local database (it'll recreate)
4. Restart app

## Testing

```bash
# Rust tests
cd apps/desktop/src-tauri
cargo test

# Frontend tests
cd apps/desktop
npm test

# Quick compile check (no run)
cargo check
```

## Building Release

```bash
cd apps/desktop
npm run tauri build
```

**Output:**
- Windows: `src-tauri/target/release/bundle/msi/Meet Scribe_0.1.0_x64_en-US.msi`
- Linux: `src-tauri/target/release/bundle/deb/meet-scribe_0.1.0_amd64.deb`
- Executable: `src-tauri/target/release/meet-scribe.exe` (Windows) or `meet-scribe` (Linux)

**Build time:**
- First: ~15 minutes
- Subsequent: ~5 minutes

## Common Tasks

### Add New Tauri Command

1. **Define in Rust** (`src-tauri/src/main.rs` or create in `commands/`):
```rust
#[tauri::command]
async fn my_command(param: String) -> Result<String, String> {
    Ok(format!("Received: {}", param))
}
```

2. **Register in main:**
```rust
.invoke_handler(tauri::generate_handler![
    get_version,
    check_db_health,
    my_command  // Add here
])
```

3. **Call from Frontend:**
```typescript
import { invoke } from '@tauri-apps/api/core';

const result = await invoke('my_command', { param: 'test' });
```

### Add Database Table

1. **Create migration** (`migrations/002_add_table.sql`):
```sql
CREATE TABLE my_table (
    id INTEGER PRIMARY KEY,
    data TEXT NOT NULL
);
```

2. **Update migrations** in `sqlite.rs`:
```rust
let migrations = Migrations::new(vec![
    M::up(include_str!("../../../migrations/001_initial.sql")),
    M::up(include_str!("../../../migrations/002_add_table.sql")),
]);
```

3. **Add domain model** in `domain/models.rs`
4. **Add storage methods** in `ports/storage.rs` and `adapters/storage/sqlite.rs`

### Add New Page

1. **Create component** (`src/pages/MyPage.tsx`):
```typescript
function MyPage() {
  return (
    <div>
      <h1>My Page</h1>
    </div>
  );
}
export default MyPage;
```

2. **Add route** in `App.tsx`:
```typescript
import MyPage from "./pages/MyPage";

<Routes>
  {/* existing routes */}
  <Route path="/mypage" element={<MyPage />} />
</Routes>
```

3. **Add nav link** in `App.tsx` sidebar

## Project Structure

```
apps/desktop/
├── src/                        # React frontend
│   ├── pages/                 # Page components
│   ├── components/            # Reusable components
│   ├── types/                 # TypeScript types
│   └── App.tsx                # Main app + routing
├── src-tauri/                  # Rust backend
│   ├── src/
│   │   ├── domain/            # Business models
│   │   ├── ports/             # Trait interfaces
│   │   ├── adapters/          # Implementations
│   │   ├── commands/          # Tauri IPC commands
│   │   └── main.rs            # App entry
│   └── migrations/            # SQL migrations
└── package.json
```

## Troubleshooting

### Build fails - "icon.ico not found"
```bash
npx @tauri-apps/cli icon app-icon.svg
```

### "Port 1420 already in use"
```bash
# Windows
netstat -ano | findstr :1420
taskkill /PID <PID> /F

# Linux
lsof -i :1420 | grep LISTEN
kill <PID>
```

### Database is corrupt
```bash
# Windows
del "%APPDATA%\com.srprasanna.meet-scribe\meet-scribe.db"

# Linux
rm ~/.local/share/com.srprasanna.meet-scribe/meet-scribe.db
```

### Clean rebuild
```bash
cd apps/desktop/src-tauri
cargo clean
cd ..
rm -rf node_modules
npm install
npm run tauri dev
```

## Code Quality

```bash
# Before commit
cd apps/desktop

# Format
cd src-tauri && cargo fmt
cd .. && npm run format

# Lint
cd src-tauri && cargo clippy
cd .. && npm run lint

# Test
cd src-tauri && cargo test
cd .. && npm test
```

## VS Code Setup

**Recommended Extensions:**
- rust-analyzer
- Tauri
- ESLint
- Prettier
- CodeLLDB (for debugging)

**Settings** (`.vscode/settings.json`):
```json
{
  "rust-analyzer.linkedProjects": [
    "apps/desktop/src-tauri/Cargo.toml"
  ],
  "editor.formatOnSave": true
}
```

## Performance Tips

### Speed up Rust compilation
```bash
# Install sccache (caching compiler)
cargo install sccache

# Add to shell profile
export RUSTC_WRAPPER=sccache
```

### Faster iteration on UI
```bash
# Run frontend only (no Rust compilation)
npm run dev
# Opens in browser, backend calls won't work
```

## Database Inspection

```bash
# Install SQLite
# Windows: https://www.sqlite.org/download.html
# Linux: sudo apt install sqlite3

# Open database
sqlite3 %APPDATA%/com.srprasanna.meet-scribe/meet-scribe.db

# Commands
.tables              # List tables
.schema meetings     # Show table schema
SELECT * FROM meetings LIMIT 10;
.quit
```

## Useful Commands Cheat Sheet

```bash
# Development
npm run tauri dev              # Run app in debug mode
npm run dev                    # Frontend only
cargo check                    # Quick Rust compile check

# Building
npm run tauri build            # Production build
npm run tauri build -- --debug # Debug build with symbols

# Testing
cargo test                     # Run Rust tests
cargo test -- --nocapture      # See println! output
npm test                       # Run frontend tests

# Cleaning
cargo clean                    # Clean Rust build
rm -rf node_modules           # Clean Node modules
rm -rf src-tauri/target       # Clean all Rust artifacts

# Info
npx @tauri-apps/cli info      # System information
rustc --version               # Rust version
node --version                # Node version
```

## Getting Help

1. Check [README.md](README.md) - comprehensive guide
2. Check [CLAUDE.md](CLAUDE.md) - architecture details
3. Check [Tauri docs](https://tauri.app/v2/)
4. Create issue with:
   - Output of `npx @tauri-apps/cli info`
   - Error message
   - Steps to reproduce
