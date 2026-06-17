#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"
BACKEND_PORT="${BACKEND_PORT:-8080}"
HOST="${HOST:-127.0.0.1}"

BACKEND_PID=""
FRONTEND_PID=""
TTS_PID=""
TTS_PORT="${TTS_LOCAL_PORT:-8123}"

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
}

start_backend() {
  echo "Starting backend with hot reload on http://${HOST}:${BACKEND_PORT}"
  (
    cd "${ROOT_DIR}/backend"
    PORT="${BACKEND_PORT}" cargo watch \
      --watch src \
      --watch Cargo.toml \
      --watch Cargo.lock \
      --ignore storage \
      -x run
  ) &
  BACKEND_PID="$!"
}

start_frontend() {
  echo "Starting frontend on http://${HOST}:${FRONTEND_PORT}"
  (
    cd "${ROOT_DIR}/frontend"
    PORT="${FRONTEND_PORT}" npm run dev -- --hostname "${HOST}" --port "${FRONTEND_PORT}"
  ) &
  FRONTEND_PID="$!"
}

start_tts() {
  # Optional: only start the local XTTS sidecar if it has been set up.
  if [[ ! -x "${ROOT_DIR}/tts_service/.venv/bin/python" ]]; then
    echo "TTS sidecar not set up (run tts_service/setup.sh) — skipping."
    return 0
  fi
  echo "Starting TTS sidecar on http://${HOST}:${TTS_PORT}"
  (
    cd "${ROOT_DIR}"
    TTS_LOCAL_PORT="${TTS_PORT}" tts_service/.venv/bin/python -m tts_service.run
  ) &
  TTS_PID="$!"
}

free_port() {
  local port="$1"
  local pids

  pids="$(lsof -tiTCP:"${port}" -sTCP:LISTEN 2>/dev/null || true)"
  if [[ -z "${pids}" ]]; then
    return 0
  fi

  echo "Freeing port ${port}: ${pids}"
  kill ${pids} 2>/dev/null || true
  sleep 1

  pids="$(lsof -tiTCP:"${port}" -sTCP:LISTEN 2>/dev/null || true)"
  if [[ -n "${pids}" ]]; then
    echo "Force killing port ${port}: ${pids}"
    kill -9 ${pids} 2>/dev/null || true
  fi
}

cleanup() {
  local exit_code="${1:-0}"

  trap - EXIT INT TERM

  for pid in "${FRONTEND_PID}" "${BACKEND_PID}" "${TTS_PID}"; do
    if [[ -n "${pid}" ]] && kill -0 "${pid}" 2>/dev/null; then
      kill "${pid}" 2>/dev/null || true
    fi
  done

  wait 2>/dev/null || true
  exit "${exit_code}"
}

monitor_processes() {
  while true; do
    if ! kill -0 "${BACKEND_PID}" 2>/dev/null; then
      wait "${BACKEND_PID}" || true
      echo "Backend stopped"
      cleanup 1
    fi

    if ! kill -0 "${FRONTEND_PID}" 2>/dev/null; then
      wait "${FRONTEND_PID}" || true
      echo "Frontend stopped"
      cleanup 1
    fi

    sleep 1
  done
}

require_command lsof
require_command npm
require_command cargo
require_command cargo-watch

free_port "${FRONTEND_PORT}"
free_port "${BACKEND_PORT}"
free_port "${TTS_PORT}"

start_tts
start_backend
start_frontend

trap 'cleanup $?' EXIT INT TERM

echo "Frontend PID: ${FRONTEND_PID}"
echo "Backend PID: ${BACKEND_PID}"
echo "TTS PID: ${TTS_PID:-<not started>}"

monitor_processes
