# Quick Release Guide

This is a quick reference for creating releases. For detailed documentation, see [docs/RELEASE.md](docs/RELEASE.md).

## TL;DR - Release Checklist

```bash
# 1. Bump version (from repository root)
npm run bump:minor  # or bump:patch, bump:major, or bump 1.2.3

# 2. Update CHANGELOG.md
# (Edit manually - add release notes)

# 3. Commit, tag, and push
git add -A
git commit -m "chore: release v1.2.3"
git tag v1.2.3
git push origin main --tags

# 4. Wait for GitHub Actions to build and release
# Check: https://github.com/srprasanna/meet-scribe/actions
```

## Version Bump Commands

From repository root:

```bash
# Semantic versioning
npm run bump:patch  # 1.0.0 → 1.0.1 (bug fixes)
npm run bump:minor  # 1.0.0 → 1.1.0 (new features)
npm run bump:major  # 1.0.0 → 2.0.0 (breaking changes)

# Specific version
npm run bump 2.5.3  # Set exact version
```

## What Gets Updated

The bump scripts automatically update:
- ✅ `apps/desktop/package.json`
- ✅ `apps/desktop/package-lock.json`
- ✅ `apps/desktop/src-tauri/tauri.conf.json`
- ✅ `apps/desktop/src-tauri/Cargo.toml`
- ✅ `apps/desktop/src-tauri/Cargo.lock`

## Release Artifacts

GitHub Actions automatically builds and uploads:

### Windows
- `meet-scribe_{version}_x64_en-US.msi` - MSI installer

### Linux
- `meet-scribe_{version}_amd64.deb` - Debian package
- `meet-scribe_{version}_amd64.AppImage` - Portable app

## Alternative: Manual Release via GitHub UI

1. Go to: https://github.com/srprasanna/meet-scribe/actions
2. Select "Release" workflow
3. Click "Run workflow"
4. Enter version (e.g., `1.2.3`)
5. Click "Run workflow"

## Versioning Guidelines

- **Patch** (0.0.X): Bug fixes, no new features
- **Minor** (0.X.0): New features, backwards-compatible
- **Major** (X.0.0): Breaking changes

## Pre-Release Checklist

- [ ] All tests pass locally (`cargo test && npm test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] No linter warnings (`cargo clippy`)
- [ ] CHANGELOG.md is updated
- [ ] README.md is current
- [ ] Breaking changes are documented

## Post-Release Checklist

- [ ] Verify release appears on GitHub
- [ ] Download and test Windows MSI
- [ ] Download and test Linux packages
- [ ] Verify all artifacts are present
- [ ] Update documentation (if needed)

## Troubleshooting

### "Permission denied" on GitHub
- Go to Settings → Actions → General
- Select "Read and write permissions"

### "Tag already exists"
```bash
git tag -d v1.2.3                    # Delete local tag
git push origin :refs/tags/v1.2.3   # Delete remote tag
npm run bump 1.2.4                   # Bump to new version
```

### Build fails on GitHub Actions
- Check workflow logs: https://github.com/srprasanna/meet-scribe/actions
- Common issues documented in [docs/RELEASE.md](docs/RELEASE.md#troubleshooting)

## Resources

- **Detailed Release Guide**: [docs/RELEASE.md](docs/RELEASE.md)
- **CI/CD Setup**: [docs/CI-CD-SETUP.md](docs/CI-CD-SETUP.md)
- **Changelog**: [CHANGELOG.md](CHANGELOG.md)
- **GitHub Actions**: https://github.com/srprasanna/meet-scribe/actions
- **Releases**: https://github.com/srprasanna/meet-scribe/releases
