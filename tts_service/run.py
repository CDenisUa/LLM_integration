"""Entry point: start the sidecar with the real XTTS engine."""

# Core
import os
import uvicorn
# Services
from .app import build_default_app

app = build_default_app()


def main() -> None:
    port = int(os.environ.get("TTS_LOCAL_PORT", "8123"))
    uvicorn.run(app, host="127.0.0.1", port=port)


if __name__ == "__main__":
    main()
