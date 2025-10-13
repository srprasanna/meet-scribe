#!/usr/bin/env node

/**
 * Version Bump Script for Meet Scribe
 *
 * Updates version in:
 * - apps/desktop/package.json
 * - apps/desktop/src-tauri/tauri.conf.json
 * - apps/desktop/src-tauri/Cargo.toml
 *
 * Usage:
 *   node scripts/bump-version.js <version>
 *   node scripts/bump-version.js patch|minor|major
 *
 * Examples:
 *   node scripts/bump-version.js 1.2.3
 *   node scripts/bump-version.js patch  # 1.0.0 -> 1.0.1
 *   node scripts/bump-version.js minor  # 1.0.0 -> 1.1.0
 *   node scripts/bump-version.js major  # 1.0.0 -> 2.0.0
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// File paths
const PACKAGE_JSON = path.join(__dirname, '../apps/desktop/package.json');
const TAURI_CONF = path.join(__dirname, '../apps/desktop/src-tauri/tauri.conf.json');
const CARGO_TOML = path.join(__dirname, '../apps/desktop/src-tauri/Cargo.toml');

/**
 * Parse semantic version
 */
function parseVersion(version) {
  const match = version.match(/^(\d+)\.(\d+)\.(\d+)$/);
  if (!match) {
    throw new Error(`Invalid version format: ${version}. Expected: X.Y.Z`);
  }
  return {
    major: parseInt(match[1], 10),
    minor: parseInt(match[2], 10),
    patch: parseInt(match[3], 10)
  };
}

/**
 * Increment version based on type
 */
function incrementVersion(current, type) {
  const v = parseVersion(current);

  switch (type) {
    case 'major':
      return `${v.major + 1}.0.0`;
    case 'minor':
      return `${v.major}.${v.minor + 1}.0`;
    case 'patch':
      return `${v.major}.${v.minor}.${v.patch + 1}`;
    default:
      throw new Error(`Invalid version type: ${type}. Expected: major, minor, or patch`);
  }
}

/**
 * Update package.json
 */
function updatePackageJson(newVersion) {
  const pkg = JSON.parse(fs.readFileSync(PACKAGE_JSON, 'utf8'));
  const oldVersion = pkg.version;
  pkg.version = newVersion;
  fs.writeFileSync(PACKAGE_JSON, JSON.stringify(pkg, null, 2) + '\n');
  return oldVersion;
}

/**
 * Update tauri.conf.json
 */
function updateTauriConf(newVersion) {
  const config = JSON.parse(fs.readFileSync(TAURI_CONF, 'utf8'));
  config.version = newVersion;
  fs.writeFileSync(TAURI_CONF, JSON.stringify(config, null, 2) + '\n');
}

/**
 * Update Cargo.toml
 */
function updateCargoToml(newVersion) {
  let content = fs.readFileSync(CARGO_TOML, 'utf8');
  content = content.replace(/^version = ".*"$/m, `version = "${newVersion}"`);
  fs.writeFileSync(CARGO_TOML, content);
}

/**
 * Update Cargo.lock
 */
function updateCargoLock() {
  try {
    console.log('Updating Cargo.lock...');
    const cargoDir = path.dirname(CARGO_TOML);
    execSync('cargo generate-lockfile', {
      cwd: cargoDir,
      stdio: 'inherit'
    });
  } catch (error) {
    console.warn('Warning: Failed to update Cargo.lock:', error.message);
  }
}

/**
 * Update package-lock.json
 */
function updatePackageLock() {
  try {
    console.log('Updating package-lock.json...');
    const desktopDir = path.dirname(PACKAGE_JSON);
    execSync('npm install --package-lock-only', {
      cwd: desktopDir,
      stdio: 'inherit'
    });
  } catch (error) {
    console.warn('Warning: Failed to update package-lock.json:', error.message);
  }
}

/**
 * Main function
 */
function main() {
  const arg = process.argv[2];

  if (!arg) {
    console.error('Error: Version argument required');
    console.error('Usage: node scripts/bump-version.js <version|patch|minor|major>');
    console.error('Examples:');
    console.error('  node scripts/bump-version.js 1.2.3');
    console.error('  node scripts/bump-version.js patch');
    process.exit(1);
  }

  try {
    // Read current version
    const pkg = JSON.parse(fs.readFileSync(PACKAGE_JSON, 'utf8'));
    const currentVersion = pkg.version;

    // Determine new version
    let newVersion;
    if (['major', 'minor', 'patch'].includes(arg)) {
      newVersion = incrementVersion(currentVersion, arg);
    } else {
      // Validate provided version
      parseVersion(arg);
      newVersion = arg;
    }

    console.log(`\nBumping version: ${currentVersion} -> ${newVersion}\n`);

    // Update all files
    console.log('Updating package.json...');
    updatePackageJson(newVersion);

    console.log('Updating tauri.conf.json...');
    updateTauriConf(newVersion);

    console.log('Updating Cargo.toml...');
    updateCargoToml(newVersion);

    updateCargoLock();
    updatePackageLock();

    console.log('\n✅ Version bumped successfully!\n');
    console.log('Updated files:');
    console.log('  - apps/desktop/package.json');
    console.log('  - apps/desktop/package-lock.json');
    console.log('  - apps/desktop/src-tauri/tauri.conf.json');
    console.log('  - apps/desktop/src-tauri/Cargo.toml');
    console.log('  - apps/desktop/src-tauri/Cargo.lock');
    console.log('\nNext steps:');
    console.log('  1. Review changes: git diff');
    console.log('  2. Commit changes: git add -A && git commit -m "chore: bump version to ' + newVersion + '"');
    console.log('  3. Create tag: git tag v' + newVersion);
    console.log('  4. Push: git push && git push --tags');

  } catch (error) {
    console.error('\n❌ Error:', error.message);
    process.exit(1);
  }
}

main();
