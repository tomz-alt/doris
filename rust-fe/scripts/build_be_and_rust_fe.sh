#!/usr/bin/env bash
#
# Build Doris BE and the Rust FE (with real_be_proto) from within the
# rust-fe directory, without modifying the BE codebase. This script is
# intended as orchestration glue only; it respects AGENTS.md by keeping
# build logic and protocol details at the edges.
#
# Usage:
#   ./scripts/build_be_and_rust_fe.sh [--release]
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DORIS_ROOT="${ROOT_DIR}/.."

release_build=false
if [[ "${1:-}" == "--release" ]]; then
  release_build=true
fi

echo "==> Building Doris BE via ../build.sh (no code changes)"
(
  cd "${DORIS_ROOT}"
  # This relies on the standard Doris build entrypoint. Adjust flags
  # as needed for your environment (e.g., enable only BE if desired).
  ./build.sh --be
)

echo "==> Building Rust FE with real_be_proto"
(
  cd "${ROOT_DIR}"
  if $release_build; then
    cargo build --release --features real_be_proto
  else
    cargo build --features real_be_proto
  fi
)

echo "Build complete."

