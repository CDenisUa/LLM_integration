"""FastAPI app for the local XTTS v2 TTS sidecar.

The Rust backend calls this service over HTTP as the ``local`` TTS provider.
Run with: ``python -m tts_service.run`` (see run.py) or ``uvicorn``.
"""

# Core
from typing import Optional
from fastapi import FastAPI, HTTPException, Response
from pydantic import BaseModel, Field
# Services
from .engine import TtsEngine, XttsEngine


class SynthesizeRequest(BaseModel):
    text: str = Field(..., min_length=1)
    language: str = "ru"
    speaker: Optional[str] = None
    speed: float = 1.0


def create_app(engine: TtsEngine) -> FastAPI:
    """Build the app around an injected engine (tests pass a stub)."""
    app = FastAPI(title="Audiobook TTS Sidecar", version="1.0.0")

    @app.get("/health")
    def health() -> dict:
        return engine.health()

    @app.get("/voices")
    def voices() -> dict:
        return {"voices": [v.as_dict() for v in engine.list_voices()]}

    @app.post("/synthesize")
    def synthesize(req: SynthesizeRequest) -> Response:
        if not req.text.strip():
            raise HTTPException(status_code=400, detail="text is empty")
        try:
            audio = engine.synthesize(
                text=req.text,
                language=req.language,
                speaker=req.speaker,
                speed=req.speed,
            )
        except Exception as exc:  # surface engine failures as 500 with a message
            raise HTTPException(status_code=500, detail=str(exc)) from exc
        return Response(content=audio, media_type="audio/wav")

    return app


def build_default_app() -> FastAPI:
    """Production app backed by the real XTTS engine."""
    return create_app(XttsEngine())
