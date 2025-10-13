#!/bin/bash

# Version Bump Script for Meet Scribe (Shell version for Linux/macOS)
#
# Usage:
#   ./scripts/bump-version.sh <version>
#
# Example:
#   ./scripts/bump-version.sh 1.2.3

set -e

VERSION=$1

if [ -z "$VERSION" ]; then
  echo "Error: Version argument required"
  echo "Usage: ./scripts/bump-version.sh <version>"
  echo "Example: ./scripts/bump-version.sh 1.2.3"
  exit 1
fi

# Validate version format
if ! [[ $VERSION =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  echo "Error: Invalid version format. Expected: X.Y.Z"
  exit 1
fi

PACKAGE_JSON="apps/desktop/package.json"
TAURI_CONF="apps/desktop/src-tauri/tauri.conf.json"
CARGO_TOML="apps/desktop/src-tauri/Cargo.toml"

# Get current version
CURRENT_VERSION=$(node -p "require('./$PACKAGE_JSON').version")

echo ""
echo "Bumping version: $CURRENT_VERSION -> $VERSION"
echo ""

# Update package.json using npm
echo "Updating package.json..."
cd apps/desktop
npm version $VERSION --no-git-tag-version --allow-same-version
cd ../..

# Update tauri.conf.json
echo "Updating tauri.conf.json..."
sed -i.bak "s/\"version\": \".*\"/\"version\": \"$VERSION\"/" $TAURI_CONF && rm -f ${TAURI_CONF}.bak

# Update Cargo.toml
echo "Updating Cargo.toml..."
sed -i.bak "s/^version = \".*\"/version = \"$VERSION\"/" $CARGO_TOML && rm -f ${CARGO_TOML}.bak

# Update Cargo.lock
echo "Updating Cargo.lock..."
cd apps/desktop/src-tauri
cargo generate-lockfile
cd ../../..

echo ""
echo "✅ Version bumped successfully!"
echo ""
echo "Updated files:"
echo "  - apps/desktop/package.json"
echo "  - apps/desktop/package-lock.json"
echo "  - apps/desktop/src-tauri/tauri.conf.json"
echo "  - apps/desktop/src-tauri/Cargo.toml"
echo "  - apps/desktop/src-tauri/Cargo.lock"
echo ""
echo "Next steps:"
echo "  1. Review changes: git diff"
echo "  2. Commit changes: git add -A && git commit -m \"chore: bump version to $VERSION\""
echo "  3. Create tag: git tag v$VERSION"
echo "  4. Push: git push && git push --tags"
echo ""
