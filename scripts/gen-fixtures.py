#!/usr/bin/env python3
import math
import struct
import wave
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "openmix-core" / "tests" / "fixtures"
RATE = 44100
FRAMES = RATE
AMP = 0.8


def sine_wav(path: Path) -> None:
    with wave.open(str(path), "wb") as f:
        f.setnchannels(1)
        f.setsampwidth(2)
        f.setframerate(RATE)
        pcm = b"".join(
            struct.pack(
                "<h",
                int(AMP * 32767 * math.sin(2 * math.pi * 1000.0 * i / RATE)),
            )
            for i in range(FRAMES)
        )
        f.writeframes(pcm)


if __name__ == "__main__":
    FIXTURES.mkdir(parents=True, exist_ok=True)
    sine_wav(FIXTURES / "sine1k_1s.wav")
    print(f"wrote {FIXTURES / 'sine1k_1s.wav'}")