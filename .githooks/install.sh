#!/usr/bin/env bash
# Install git hooks for Oppskrift development

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Installing git hooks..."

# Configure git to use .githooks directory
git config core.hooksPath .githooks

echo "Git hooks installed successfully!"
echo ""
echo "The following hooks are now active:"
echo "  - pre-commit: Runs cargo fmt and clippy before each commit"
echo ""
echo "To generate SQLx offline cache (recommended for faster checks):"
echo "  1. Ensure DATABASE_URL is set and migrations are run"
echo "  2. Run: cargo sqlx prepare"
echo "  3. Commit the .sqlx directory"
