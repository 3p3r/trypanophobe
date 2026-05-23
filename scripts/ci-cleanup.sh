#!/usr/bin/env bash
# Drop CI/local cross-build trees so cross-target and .cargo-home do not grow without bound.
set -uo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
cd "${root}"

# Docker cross-builds create root-owned files under cross-target/.
if command -v docker >/dev/null 2>&1 && [[ -d cross-target ]]; then
	docker run --rm -v "${root}:/build" -w /build rust:1-trixie \
		bash -c 'rm -rf cross-target .cargo-home artifacts dist' 2>/dev/null || true
fi

rm -rf cross-target cross-target-* .cargo-home artifacts dist 2>/dev/null \
	|| sudo rm -rf cross-target cross-target-* .cargo-home artifacts dist 2>/dev/null \
	|| true

echo "pruned cross-target*, .cargo-home, artifacts/, dist/ under ${root}"
