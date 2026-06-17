"""Contract tests for the TTS sidecar — run with a stub engine, no model load."""

# Core
import io
import wave
import pytest
from fastapi.testclient import TestClient
# Services
from tts_service.app import create_app
from tts_service.engine import TtsEngine, Voice, encode_wav


class StubEngine(TtsEngine):
    def __init__(self, fail: bool = False) -> None:
        self.fail = fail
        self.calls: list[dict] = []

    def health(self) -> dict:
        return {"status": "ok", "model": "stub", "device": "cpu", "model_loaded": True}

    def list_voices(self) -> list[Voice]:
        return [Voice(id="Ana Florence", name="Ana Florence", languages=["ru", "en"])]

    def synthesize(self, text, language, speaker=None, speed=1.0) -> bytes:
        self.calls.append(
            {"text": text, "language": language, "speaker": speaker, "speed": speed}
        )
        if self.fail:
            raise RuntimeError("boom")
        return encode_wav([0.0, 0.5, -0.5, 0.0], sample_rate=24000)


def client(fail: bool = False) -> tuple[TestClient, StubEngine]:
    engine = StubEngine(fail=fail)
    return TestClient(create_app(engine)), engine


def test_health_reports_ok():
    c, _ = client()
    res = c.get("/health")
    assert res.status_code == 200
    assert res.json()["status"] == "ok"


def test_voices_lists_speakers():
    c, _ = client()
    res = c.get("/voices")
    assert res.status_code == 200
    voices = res.json()["voices"]
    assert voices[0]["id"] == "Ana Florence"
    assert "ru" in voices[0]["languages"]


def test_synthesize_returns_wav_audio():
    c, engine = client()
    res = c.post("/synthesize", json={"text": "Привет", "language": "ru", "speed": 1.0})
    assert res.status_code == 200
    assert res.headers["content-type"] == "audio/wav"
    # body must be a parseable WAV stream
    with wave.open(io.BytesIO(res.content), "rb") as wav:
        assert wav.getframerate() == 24000
        assert wav.getnchannels() == 1
    assert engine.calls[0]["text"] == "Привет"
    assert engine.calls[0]["language"] == "ru"


def test_synthesize_rejects_empty_text():
    c, _ = client()
    # pydantic min_length=1 rejects "" with 422
    assert c.post("/synthesize", json={"text": "", "language": "ru"}).status_code == 422
    # whitespace-only passes validation but is rejected by the handler
    assert c.post("/synthesize", json={"text": "   ", "language": "ru"}).status_code == 400


def test_synthesize_surfaces_engine_error():
    c, _ = client(fail=True)
    res = c.post("/synthesize", json={"text": "hi", "language": "en"})
    assert res.status_code == 500
    assert "boom" in res.json()["detail"]


def test_encode_wav_roundtrip():
    data = encode_wav([0.0, 1.0, -1.0, 0.25], sample_rate=16000)
    with wave.open(io.BytesIO(data), "rb") as wav:
        assert wav.getframerate() == 16000
        assert wav.getsampwidth() == 2
        assert wav.getnframes() == 4
