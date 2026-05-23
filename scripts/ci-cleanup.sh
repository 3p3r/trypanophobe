#!/usr/bin/env bash
# Drop CI/local cross-build trees so cross-target and .cargo-home do not grow without bound.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

rm -rf cross-target cross-target-* .cargo-home artifacts dist
echo "removed cross-target*, .cargo-home, artifacts/, dist/ under ${root}"
