# TTS Sidecar — Coqui XTTS v2

Local text-to-speech service for the audiobook generator. The Rust backend calls
it over HTTP as the `local` TTS provider. The XTTS v2 model is downloaded **once**
and then runs fully offline on your Mac (CPU by default on Apple Silicon).

> License: XTTS v2 is under the Coqui Public Model License (CPML), non-commercial.
> This app is personal-use only, which is compliant.

## Install (one time)

```bash
cd tts_service
./setup.sh --download   # creates .venv, installs deps, downloads the model (~1.8GB)
```

After this, no internet is needed — the model loads from the local cache.

## Run

From the **repo root** (the package import needs it):

```bash
tts_service/.venv/bin/python -m tts_service.run
```

Listens on `http://127.0.0.1:8123` (override with `TTS_LOCAL_PORT`).

## Endpoints

| Method | Path          | Body / Result |
|--------|---------------|---------------|
| GET    | `/health`     | `{status, model, device, model_loaded}` |
| GET    | `/voices`     | `{voices: [{id, name, languages}]}` |
| POST   | `/synthesize` | `{text, language?, speaker?, speed?}` → `audio/wav` bytes |

## Config (env)

| Var             | Default        | Notes |
|-----------------|----------------|-------|
| `TTS_LOCAL_PORT`| `8123`         | HTTP port |
| `XTTS_DEVICE`   | `cpu`          | `cpu` is most stable on Apple Silicon |
| `XTTS_SPEAKER`  | `Ana Florence` | Built-in XTTS studio speaker |

## Tests (no model required)

Contract tests use a stub engine, so torch/the model are never loaded:

```bash
python3 -m venv tts_service/.venv-dev
tts_service/.venv-dev/bin/pip install -r tts_service/requirements-dev.txt
tts_service/.venv-dev/bin/python -m pytest tts_service/tests -q   # run from repo root
```
