"""TTS engine abstraction and the Coqui XTTS v2 implementation.

The FastAPI app depends only on the :class:`TtsEngine` interface, so tests can
inject a lightweight stub and never load the ~1.8GB model.
"""

# Core
from __future__ import annotations
import io
import os
import wave
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Optional

# XTTS v2 supported languages (ISO codes used by the model).
XTTS_LANGUAGES = [
    "en", "es", "fr", "de", "it", "pt", "pl", "tr", "ru", "nl",
    "cs", "ar", "zh-cn", "ja", "hu", "ko", "hi",
]


@dataclass
class Voice:
    id: str
    name: str
    languages: list[str] = field(default_factory=lambda: list(XTTS_LANGUAGES))

    def as_dict(self) -> dict:
        return {"id": self.id, "name": self.name, "languages": self.languages}


class TtsEngine(ABC):
    """Minimal contract the HTTP layer relies on."""

    @abstractmethod
    def health(self) -> dict:
        ...

    @abstractmethod
    def list_voices(self) -> list[Voice]:
        ...

    @abstractmethod
    def synthesize(
        self,
        text: str,
        language: str,
        speaker: Optional[str] = None,
        speed: float = 1.0,
    ) -> bytes:
        """Return WAV-encoded audio bytes for ``text``."""
        ...


def encode_wav(samples: list[float], sample_rate: int) -> bytes:
    """Encode mono float32 samples in [-1, 1] to 16-bit PCM WAV bytes.

    Kept dependency-free so it can be unit-tested without torch/soundfile.
    """
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as wav:
        wav.setnchannels(1)
        wav.setsampwidth(2)  # 16-bit
        wav.setframerate(sample_rate)
        frames = bytearray()
        for sample in samples:
            clamped = max(-1.0, min(1.0, float(sample)))
            frames += int(clamped * 32767).to_bytes(2, "little", signed=True)
        wav.writeframes(bytes(frames))
    return buffer.getvalue()


class XttsEngine(TtsEngine):
    """Coqui XTTS v2 wrapper. Loads the model lazily on first use."""

    MODEL_NAME = "tts_models/multilingual/multi-dataset/xtts_v2"

    def __init__(self, device: Optional[str] = None) -> None:
        # XTTS on Apple Silicon is most stable on CPU; allow override via env.
        self.device = device or os.environ.get("XTTS_DEVICE", "cpu")
        self._tts = None  # loaded lazily
        os.environ.setdefault("COQUI_TOS_AGREED", "1")  # accept CPML non-interactively

    def _ensure_loaded(self):
        if self._tts is None:
            # Core (heavy import deferred until the model is actually needed)
            from TTS.api import TTS

            self._tts = TTS(self.MODEL_NAME).to(self.device)
        return self._tts

    def health(self) -> dict:
        return {
            "status": "ok",
            "model": self.MODEL_NAME,
            "device": self.device,
            "model_loaded": self._tts is not None,
        }

    def list_voices(self) -> list[Voice]:
        tts = self._ensure_loaded()
        names = list(getattr(tts, "speakers", None) or [])
        if not names:
            names = ["Ana Florence"]
        return [Voice(id=name, name=name) for name in names]

    def synthesize(
        self,
        text: str,
        language: str,
        speaker: Optional[str] = None,
        speed: float = 1.0,
    ) -> bytes:
        tts = self._ensure_loaded()
        speaker = speaker or os.environ.get("XTTS_SPEAKER", "Ana Florence")
        wav = tts.tts(text=text, speaker=speaker, language=language, speed=speed)
        sample_rate = getattr(
            getattr(tts, "synthesizer", None), "output_sample_rate", 24000
        )
        return encode_wav(list(wav), sample_rate)
