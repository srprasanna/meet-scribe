# Release Process

This document describes how to create releases for Meet Scribe.

## Table of Contents
- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Version Bumping](#version-bumping)
- [Creating a Release](#creating-a-release)
- [GitHub Actions Workflow](#github-actions-workflow)
- [Manual Release (Advanced)](#manual-release-advanced)

## Overview

Meet Scribe uses GitHub Actions to automate the build and release process. The workflow:

1. **Version Bumping**: Updates version in all necessary files
2. **Building**: Creates Windows (MSI) and Linux (DEB/AppImage) binaries
3. **Release**: Creates a GitHub release with all artifacts
4. **Publishing**: Uploads binaries to the GitHub release

## Prerequisites

### For Automated Releases (Recommended)

1. **GitHub Secrets**: Configure the following secrets in your GitHub repository settings (Settings → Secrets and variables → Actions):
   - `GITHUB_TOKEN` (automatically provided by GitHub Actions)
   - `TAURI_PRIVATE_KEY` (optional - for signed updates)
   - `TAURI_KEY_PASSWORD` (optional - for signed updates)

2. **Permissions**: Ensure GitHub Actions has write permissions:
   - Go to Settings → Actions → General
   - Under "Workflow permissions", select "Read and write permissions"
   - Check "Allow GitHub Actions to create and approve pull requests"

### For Manual Version Bumping

1. **Node.js**: Version 18 or higher
2. **Git**: Installed and configured
3. **Rust**: Latest stable version

## Version Bumping

### Automatic Version Bump (Recommended)

Use the npm scripts in the root package.json:

```bash
# Bump patch version (0.1.0 -> 0.1.1)
npm run bump:patch

# Bump minor version (0.1.0 -> 0.2.0)
npm run bump:minor

# Bump major version (0.1.0 -> 1.0.0)
npm run bump:major

# Set specific version
npm run bump 1.2.3
```

These scripts will update:
- `apps/desktop/package.json`
- `apps/desktop/package-lock.json`
- `apps/desktop/src-tauri/tauri.conf.json`
- `apps/desktop/src-tauri/Cargo.toml`
- `apps/desktop/src-tauri/Cargo.lock`

### Manual Version Bump

#### Using Node.js Script (Windows/Linux/macOS)

```bash
# Specific version
node scripts/bump-version.js 1.2.3

# Semantic versioning
node scripts/bump-version.js patch
node scripts/bump-version.js minor
node scripts/bump-version.js major
```

#### Using Shell Script (Linux/macOS)

```bash
chmod +x scripts/bump-version.sh
./scripts/bump-version.sh 1.2.3
```

## Creating a Release

### Method 1: Push a Version Tag (Recommended)

1. **Bump the version** (see above)

2. **Update CHANGELOG.md**:
   ```bash
   # Edit CHANGELOG.md and add release notes
   ```

3. **Commit and tag**:
   ```bash
   git add -A
   git commit -m "chore: bump version to 1.2.3"
   git tag v1.2.3
   git push origin main
   git push origin v1.2.3
   ```

4. **GitHub Actions will automatically**:
   - Build Windows and Linux binaries
   - Create a GitHub release
   - Upload all artifacts

### Method 2: Manual Dispatch from GitHub UI

1. Go to your GitHub repository
2. Click on "Actions" tab
3. Select "Release" workflow
4. Click "Run workflow"
5. Enter the version (e.g., `1.2.3`)
6. Click "Run workflow"

This will:
- Update version in all files
- Commit and tag automatically
- Build and release

## GitHub Actions Workflow

The release workflow (`.github/workflows/release.yml`) consists of three jobs:

### 1. Prepare Release
- Validates version format
- Updates version in all files
- Creates git tag and commit (for manual dispatch)
- Creates GitHub release

### 2. Build Windows
- Runs on: `windows-latest`
- Builds: `.msi` installer
- Uploads: Windows MSI to GitHub release

### 3. Build Linux
- Runs on: `ubuntu-latest`
- Builds: `.deb` package and `.AppImage`
- Uploads: Linux packages to GitHub release

## Manual Release (Advanced)

If you need to build locally without GitHub Actions:

### Windows

```powershell
cd apps/desktop
npm install
npm run build
npm run tauri build
```

Artifacts location:
- MSI: `apps/desktop/src-tauri/target/release/bundle/msi/`

### Linux

```bash
cd apps/desktop
npm install
npm run build
npm run tauri build
```

Artifacts location:
- DEB: `apps/desktop/src-tauri/target/release/bundle/deb/`
- AppImage: `apps/desktop/src-tauri/target/release/bundle/appimage/`

## Troubleshooting

### Build Fails on Windows

**Issue**: Missing MSVC Build Tools
```
Solution: Install Visual Studio Build Tools
https://visualstudio.microsoft.com/downloads/
Select "Desktop development with C++"
```

### Build Fails on Linux

**Issue**: Missing system dependencies
```bash
sudo apt-get update
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  libssl-dev \
  libgtk-3-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libpulse-dev
```

### Version Mismatch

**Issue**: Versions don't match across files
```bash
# Re-run the bump script
node scripts/bump-version.js <version>

# Verify versions match
grep -r "version" apps/desktop/package.json apps/desktop/src-tauri/tauri.conf.json apps/desktop/src-tauri/Cargo.toml
```

### Release Not Created

**Issue**: GitHub Actions doesn't have permissions

**Solution**:
1. Go to Settings → Actions → General
2. Under "Workflow permissions", select "Read and write permissions"
3. Re-run the workflow

## Release Checklist

Before creating a release:

- [ ] All tests pass
- [ ] CHANGELOG.md is updated
- [ ] Version is bumped in all files
- [ ] README.md is up to date
- [ ] Documentation is current
- [ ] Breaking changes are documented
- [ ] Migration guide (if needed)

After release:

- [ ] Verify artifacts are uploaded
- [ ] Test Windows installer
- [ ] Test Linux packages
- [ ] Announce release (if applicable)
- [ ] Update documentation site (if applicable)

## Versioning Guidelines

Meet Scribe follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (X.0.0): Breaking changes, incompatible API changes
- **MINOR** (0.X.0): New features, backwards-compatible
- **PATCH** (0.0.X): Bug fixes, backwards-compatible

Examples:
- Bug fix: `0.1.0` → `0.1.1`
- New feature: `0.1.0` → `0.2.0`
- Breaking change: `0.1.0` → `1.0.0`

## Tauri Update Server (Future)

For automatic updates, configure the Tauri update server:

1. Generate signing keys:
   ```bash
   cd apps/desktop/src-tauri
   cargo tauri signer generate -w ~/.tauri/myapp.key
   ```

2. Add to GitHub secrets:
   - `TAURI_PRIVATE_KEY`: Content of the private key
   - `TAURI_KEY_PASSWORD`: Password for the key

3. Enable updates in `tauri.conf.json`:
   ```json
   {
     "updater": {
       "active": true,
       "endpoints": [
         "https://github.com/srprasanna/meet-scribe/releases/latest/download/latest.json"
       ],
       "dialog": true,
       "pubkey": "YOUR_PUBLIC_KEY_HERE"
     }
   }
   ```

## Resources

- [Tauri Build Documentation](https://tauri.app/v2/guides/building/)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Semantic Versioning](https://semver.org/)
- [Keep a Changelog](https://keepachangelog.com/)
