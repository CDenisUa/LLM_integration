#!/usr/bin/env bash
#
# One-time setup for the Coqui XTTS v2 sidecar.
# Creates a venv, installs deps, and (optionally) pre-downloads the model.
#
# Usage:
#   ./setup.sh            # venv + deps
#   ./setup.sh --download # venv + deps + download XTTS v2 model (~1.8GB)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

# Pick the best available Python (XTTS supports 3.9–3.12).
PYTHON=""
for candidate in python3.11 python3.10 python3.12 python3; do
  if command -v "${candidate}" >/dev/null 2>&1; then
    PYTHON="${candidate}"
    break
  fi
done
if [[ -z "${PYTHON}" ]]; then
  echo "No python3 found. Install Python 3.11 (recommended)." >&2
  exit 1
fi
echo "Using ${PYTHON} ($(${PYTHON} --version))"

if [[ ! -d .venv ]]; then
  "${PYTHON}" -m venv .venv
fi
# shellcheck disable=SC1091
source .venv/bin/activate

pip install --upgrade pip
pip install -r requirements.txt

if [[ "${1:-}" == "--download" ]]; then
  echo "Downloading XTTS v2 model (~1.8GB, one time)..."
  COQUI_TOS_AGREED=1 python -c "from TTS.api import TTS; TTS('tts_models/multilingual/multi-dataset/xtts_v2')"
  echo "Model cached. The sidecar now runs fully offline."
fi

echo
echo "Done. Start the sidecar with:"
echo "  cd ${SCRIPT_DIR} && source .venv/bin/activate && python -m tts_service.run"
echo "(run from the repo root, or set PYTHONPATH to the repo root)"
