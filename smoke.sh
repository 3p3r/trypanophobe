#!/usr/bin/env bash
# Smoke-test trypanophobe CLI and REST (no full English model inference).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

BIN="${BIN:-$ROOT/target/debug/trypanophobe}"

pick_port() {
  python3 -c 'import socket; s=socket.socket(); s.bind(("", 0)); print(s.getsockname()[1]); s.close()'
}

PORT="${PORT:-$(pick_port)}"
BASE_URL="http://127.0.0.1:${PORT}"

SERVER_PID=""
SMOKE_LOG="${TMPDIR:-/tmp}/trypanophobe-smoke-${PORT}.log"

stop_server() {
  if [[ -n "${SERVER_PID}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    kill -TERM "${SERVER_PID}" 2>/dev/null || true
    local i=0
    while kill -0 "${SERVER_PID}" 2>/dev/null && (( i < 30 )); do
      sleep 0.1
      i=$((i + 1))
    done
    if kill -0 "${SERVER_PID}" 2>/dev/null; then
      kill -KILL "${SERVER_PID}" 2>/dev/null || true
    fi
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
  SERVER_PID=""

  if command -v fuser >/dev/null 2>&1; then
    fuser -k "${PORT}/tcp" 2>/dev/null || true
  elif command -v lsof >/dev/null 2>&1; then
    local pids
    pids="$(lsof -ti ":${PORT}" 2>/dev/null || true)"
    if [[ -n "$pids" ]]; then
      kill -TERM $pids 2>/dev/null || true
      sleep 0.3
      kill -KILL $pids 2>/dev/null || true
    fi
  fi

  rm -f "$SMOKE_LOG"
}

cleanup() {
  stop_server
}

trap cleanup EXIT INT TERM

log() { printf '\n==> %s\n' "$*"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

build_binary() {
  log "building trypanophobe"
  cargo build -q
  [[ -x "$BIN" ]] || fail "binary not found: $BIN"
}

cli_smoke() {
  log "CLI smoke tests"

  "$BIN" version | grep -q trypanophobe || fail "CLI: version missing name"
  echo " ok version"

  if "$BIN" check "Bonjour le monde" >/dev/null 2>&1; then
    fail "CLI: expected failure for non-English"
  fi
  echo " ok check non-English (exit 1)"

  "$BIN" --help | grep -q check || fail "CLI: help missing check subcommand"
  echo " ok --help"
}

start_server() {
  stop_server
  log "starting REST server on port $PORT"
  "$BIN" serve --host 127.0.0.1 --port "$PORT" >"$SMOKE_LOG" 2>&1 &
  SERVER_PID=$!
  for _ in $(seq 1 50); do
    if curl -sf "$BASE_URL/api/version" >/dev/null 2>&1; then
      return 0
    fi
    if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
      cat "$SMOKE_LOG" >&2 || true
      fail "server exited before becoming ready (pid ${SERVER_PID})"
    fi
    sleep 0.1
  done
  cat "$SMOKE_LOG" >&2 || true
  stop_server
  fail "server did not become ready at $BASE_URL"
}

rest_smoke() {
  log "REST smoke tests"
  local code body tmp

  code="$(curl -s -o /dev/null -w '%{http_code}' "$BASE_URL/")"
  [[ "$code" == "307" || "$code" == "302" || "$code" == "303" ]] \
    || fail "REST: / redirect expected 302/303/307, got $code"
  echo " ok GET / (redirect to swagger-ui)"

  body="$(curl -sf "$BASE_URL/api/version")"
  [[ "$body" == *'"name":"trypanophobe"'* ]] || fail "REST: /api/version unexpected body: $body"
  echo " ok GET /api/version"

  code="$(curl -s -o /dev/null -w '%{http_code}' "$BASE_URL/api-doc/openapi.json")"
  [[ "$code" == "200" ]] || fail "REST: openapi.json returned $code"
  echo " ok GET /api-doc/openapi.json"

  code="$(curl -s -o /dev/null -w '%{http_code}' "$BASE_URL/swagger-ui/")"
  [[ "$code" == "200" ]] || fail "REST: swagger-ui returned $code"
  echo " ok GET /swagger-ui/"

  tmp="$(mktemp)"
  code="$(curl -s -o "$tmp" -w '%{http_code}' \
    -H 'Content-Type: application/json' \
    -d '{"text":"Bonjour le monde"}' \
    "$BASE_URL/api/check")"
  if [[ "$code" != "400" ]]; then
    echo "body: $(cat "$tmp")" >&2
    rm -f "$tmp"
    fail "REST: expected 400 for non-English, got $code"
  fi
  grep -q '"label":"REJECTED"' "$tmp" || fail "REST: expected REJECTED label"
  rm -f "$tmp"
  echo " ok POST /api/check non-English"

  tmp="$(mktemp)"
  code="$(curl -s -o "$tmp" -w '%{http_code}' \
    -H 'Content-Type: application/json' \
    -d '{"text":"  "}' \
    "$BASE_URL/api/check")"
  [[ "$code" == "400" ]] || fail "REST: expected 400 for empty text, got $code"
  rm -f "$tmp"
  echo " ok POST /api/check empty text"
}

main() {
  build_binary
  cli_smoke
  start_server
  rest_smoke
  stop_server
  log "all smoke tests passed"
}

main "$@"
