#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_PORT="${FRONTEND_PORT:-3000}"
BACKEND_PORT="${BACKEND_PORT:-8080}"
HOST="${HOST:-127.0.0.1}"

BACKEND_PID=""
FRONTEND_PID=""

require_command() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing required command: $1" >&2
    exit 1
  fi
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

  for pid in "${FRONTEND_PID}" "${BACKEND_PID}"; do
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

free_port "${FRONTEND_PORT}"
free_port "${BACKEND_PORT}"

echo "Starting backend on http://${HOST}:${BACKEND_PORT}"
(
  cd "${ROOT_DIR}/backend"
  PORT="${BACKEND_PORT}" cargo run
) &
BACKEND_PID="$!"

echo "Starting frontend on http://${HOST}:${FRONTEND_PORT}"
(
  cd "${ROOT_DIR}/frontend"
  PORT="${FRONTEND_PORT}" npm run dev -- --hostname "${HOST}" --port "${FRONTEND_PORT}"
) &
FRONTEND_PID="$!"

trap 'cleanup $?' EXIT INT TERM

echo "Frontend PID: ${FRONTEND_PID}"
echo "Backend PID: ${BACKEND_PID}"

monitor_processes
