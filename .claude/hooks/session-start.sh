#!/bin/bash
set -euo pipefail

# SessionStart hook for Momentum EZ Stack
# Installs Rust dependencies so linters and tests work in Claude Code web sessions.

# Only run in remote (Claude Code on the web) environments
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# ---------- Rust ----------
# Ensure rustfmt and clippy components are available
rustup component add rustfmt clippy 2>/dev/null || true

# Fetch all workspace dependencies (cached across sessions)
cargo fetch --manifest-path mez/Cargo.toml
