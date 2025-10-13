# CI/CD Setup Guide

This document describes the CI/CD infrastructure for Meet Scribe.

## Overview

Meet Scribe uses GitHub Actions for:
1. **Continuous Integration (CI)**: Automated testing on every push and PR
2. **Continuous Delivery (CD)**: Automated releases with version management

## GitHub Actions Workflows

### 1. CI Workflow (`.github/workflows/ci.yml`)

**Trigger:** Every push to `main` and all pull requests

**Jobs:**

#### Test Job (Windows & Linux)
- Runs on both `windows-latest` and `ubuntu-latest`
- Installs dependencies
- Runs frontend tests (`npm test`)
- Runs Rust tests (`cargo test`)
- Checks Rust formatting (`cargo fmt`)
- Runs Rust linter (`cargo clippy`)
- Builds frontend (`npm run build`)
- Builds Tauri app in debug mode

#### Lint Job (Ubuntu only)
- Runs ESLint on TypeScript/React code
- Checks TypeScript types (`tsc --noEmit`)

**Purpose:** Catch bugs and style issues early, ensure code quality

### 2. Release Workflow (`.github/workflows/release.yml`)

**Trigger:**
- Git tags matching `v*.*.*` (e.g., `v1.0.0`)
- Manual dispatch from GitHub UI (with version input)

**Jobs:**

#### Prepare Release
- Validates version format
- Updates version in all files (package.json, tauri.conf.json, Cargo.toml)
- Creates git commit and tag (for manual dispatch)
- Creates GitHub release with markdown description

#### Build Windows
- Runs on `windows-latest`
- Builds Windows MSI installer
- Uploads MSI to GitHub release

#### Build Linux
- Runs on `ubuntu-latest`
- Builds Debian package (`.deb`)
- Builds AppImage (`.AppImage`)
- Uploads both to GitHub release

**Purpose:** Automate the entire release process from version bump to binary distribution

## Local Development Scripts

### Version Bump Scripts

Located in `scripts/`:

1. **`bump-version.js`** (Node.js - cross-platform)
   - Updates version in all config files
   - Updates Cargo.lock and package-lock.json
   - Supports semantic versioning (patch/minor/major)

2. **`bump-version.sh`** (Bash - Linux/macOS)
   - Shell script alternative for Unix systems
   - Same functionality as Node.js version

### Root Package.json Scripts

Located in root `package.json`:

```json
{
  "bump:patch": "node scripts/bump-version.js patch",
  "bump:minor": "node scripts/bump-version.js minor",
  "bump:major": "node scripts/bump-version.js major",
  "bump": "node scripts/bump-version.js"
}
```

## Release Process

### Method 1: Tag-Based Release (Recommended)

```bash
# 1. Bump version
npm run bump:minor  # or bump:patch, bump:major

# 2. Update CHANGELOG.md
# (Edit manually)

# 3. Commit and tag
git add -A
git commit -m "chore: release v1.2.3"
git tag v1.2.3

# 4. Push
git push origin main
git push origin v1.2.3

# 5. GitHub Actions automatically builds and releases
```

### Method 2: Manual Dispatch

1. Go to GitHub repository → Actions tab
2. Select "Release" workflow
3. Click "Run workflow"
4. Enter version (e.g., `1.2.3`)
5. Click "Run workflow"

**This automatically:**
- Updates all version files
- Commits and tags
- Builds Windows and Linux binaries
- Creates GitHub release

## Configuration Requirements

### GitHub Repository Settings

#### 1. Actions Permissions

Go to: **Settings → Actions → General → Workflow permissions**

- Select: **"Read and write permissions"**
- Check: **"Allow GitHub Actions to create and approve pull requests"**

#### 2. Secrets (Optional - for signed updates)

Go to: **Settings → Secrets and variables → Actions**

Add the following secrets if you want to enable Tauri's automatic update feature:

- `TAURI_PRIVATE_KEY`: Private key for signing updates
- `TAURI_KEY_PASSWORD`: Password for the private key

**To generate keys:**
```bash
cd apps/desktop/src-tauri
cargo tauri signer generate -w ~/.tauri/myapp.key
```

### Branch Protection (Optional but Recommended)

Go to: **Settings → Branches → Add rule**

Protect the `main` branch:
- Require pull request reviews before merging
- Require status checks to pass (CI workflow)
- Require branches to be up to date before merging

## Files Created

```
meet-scribe/
├── .github/
│   └── workflows/
│       ├── ci.yml              # Continuous integration
│       └── release.yml         # Release automation
├── scripts/
│   ├── bump-version.js         # Version bump (Node.js)
│   └── bump-version.sh         # Version bump (Shell)
├── docs/
│   ├── RELEASE.md              # Release process documentation
│   └── CI-CD-SETUP.md          # This file
├── CHANGELOG.md                # Version history
└── package.json                # Root workspace config with scripts
```

## Versioning Strategy

Meet Scribe follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (X.0.0): Breaking changes, incompatible API changes
- **MINOR** (0.X.0): New features, backwards-compatible
- **PATCH** (0.0.X): Bug fixes, backwards-compatible

### Version Files

Version must be kept in sync across these files:

1. `apps/desktop/package.json` → `version`
2. `apps/desktop/src-tauri/tauri.conf.json` → `version`
3. `apps/desktop/src-tauri/Cargo.toml` → `version`

**The bump scripts automatically update all three files.**

## Build Artifacts

### Windows Build
- **Output**: `.msi` installer
- **Location**: `apps/desktop/src-tauri/target/release/bundle/msi/`
- **Size**: ~15-20 MB
- **Naming**: `meet-scribe_{version}_x64_en-US.msi`

### Linux Build
- **Output 1**: `.deb` package
  - Location: `apps/desktop/src-tauri/target/release/bundle/deb/`
  - Size: ~15-20 MB
  - Naming: `meet-scribe_{version}_amd64.deb`
- **Output 2**: `.AppImage` (portable)
  - Location: `apps/desktop/src-tauri/target/release/bundle/appimage/`
  - Size: ~20-25 MB
  - Naming: `meet-scribe_{version}_amd64.AppImage`

## Troubleshooting

### CI Build Failures

#### Windows Build Fails
**Common issues:**
- MSVC Build Tools not installed (GitHub Actions includes this)
- WebView2 runtime missing (GitHub Actions includes this)

**Solution:** Usually not an issue on GitHub Actions, but local builds need these installed.

#### Linux Build Fails
**Common issues:**
- Missing system dependencies

**Solution:** Workflow already includes all required dependencies in `apt-get install` step.

#### Rust Cache Issues
**Symptom:** Build takes longer than expected

**Solution:** The workflow uses `Swatinem/rust-cache@v2` which should cache Rust dependencies. If it's not working, check if the cache is being invalidated.

### Release Workflow Failures

#### "Permission denied" Creating Release
**Cause:** GitHub Actions doesn't have write permissions

**Solution:**
1. Go to Settings → Actions → General
2. Select "Read and write permissions"
3. Re-run the workflow

#### Version Mismatch Error
**Cause:** Version in files don't match

**Solution:**
```bash
# Re-run bump script
npm run bump 1.2.3

# Or manually update:
# - apps/desktop/package.json
# - apps/desktop/src-tauri/tauri.conf.json
# - apps/desktop/src-tauri/Cargo.toml
```

#### Tag Already Exists
**Cause:** Trying to release same version twice

**Solution:**
```bash
# Delete local and remote tag
git tag -d v1.2.3
git push origin :refs/tags/v1.2.3

# Or bump to new version
npm run bump:patch
```

### Local Script Failures

#### `node scripts/bump-version.js` Fails
**Common errors:**

1. **"Invalid version format"**
   - Ensure version is in format: `X.Y.Z`
   - Example: `node scripts/bump-version.js 1.2.3`

2. **"ENOENT: no such file or directory"**
   - Run from repository root, not from subdirectory
   - Check file paths in script

3. **Cargo/npm command not found**
   - Ensure Rust and Node.js are installed
   - Check PATH environment variable

## Best Practices

### Before Pushing

1. **Run tests locally:**
   ```bash
   cd apps/desktop/src-tauri
   cargo test
   cd ..
   npm test
   ```

2. **Check formatting:**
   ```bash
   cargo fmt
   cargo clippy
   ```

3. **Build locally to verify:**
   ```bash
   npm run tauri build
   ```

### When Creating Releases

1. **Update CHANGELOG.md** with:
   - New features
   - Bug fixes
   - Breaking changes
   - Migration instructions (if needed)

2. **Test the release candidate:**
   - Build locally
   - Test on target platforms
   - Verify all features work

3. **Use descriptive commit messages:**
   - `chore: release v1.2.3`
   - `feat: add audio capture for Windows`
   - `fix: resolve database migration error`

4. **Tag format:**
   - Always prefix with `v`: `v1.2.3`, not `1.2.3`
   - Use semantic versioning

### After Release

1. **Verify release on GitHub:**
   - Check that all artifacts are uploaded
   - Verify release notes are correct

2. **Test installers:**
   - Download Windows MSI and test installation
   - Download Linux packages and test installation

3. **Announce release** (if applicable):
   - Update documentation
   - Notify users
   - Post on social media/forums

## Monitoring

### CI Status Badge

Add to README.md:

```markdown
![CI](https://github.com/srprasanna/meet-scribe/workflows/CI/badge.svg)
```

### Build Times

Expected build times on GitHub Actions:

- **CI workflow**: 5-10 minutes per platform
- **Release workflow**: 15-25 minutes total
  - Prepare release: 1-2 minutes
  - Windows build: 8-12 minutes
  - Linux build: 8-12 minutes

### Resource Usage

GitHub Actions free tier:
- **2,000 minutes/month** for private repos
- **Unlimited** for public repos

Each full release (Windows + Linux) consumes ~20 minutes.

## Future Enhancements

### Planned Improvements

1. **Automated changelog generation**
   - Use conventional commits
   - Generate CHANGELOG.md automatically

2. **Tauri update server**
   - Enable automatic in-app updates
   - Use signed updates for security

3. **macOS builds**
   - Add macOS support in Phase 6
   - Apple Developer certificate for signing

4. **Pre-release builds**
   - Beta/RC releases for testing
   - Separate workflow for pre-releases

5. **Test coverage reporting**
   - Add code coverage tools
   - Upload coverage reports to Codecov

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Tauri Build Documentation](https://tauri.app/v2/guides/building/)
- [Semantic Versioning](https://semver.org/)
- [Conventional Commits](https://www.conventionalcommits.org/)
