"""Generate Phase 2 analysis fixtures with known BPM and key.
WAVs only; FLAC/MP3 variants produced by the Phase 1 afconvert/lame dev flow.
"""
import math, struct, wave, os

RATE = 44100

def write_wav(path, mono_f32):
    frames = bytearray()
    for s in mono_f32:
        v = int(max(-1.0, min(1.0, s)) * 32767)
        frames += struct.pack("<h", v)
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
        w.writeframes(bytes(frames))

def write_stereo_wav(path, mono_f32):
    frames = bytearray()
    for s in mono_f32:
        v = int(max(-1.0, min(1.0, s)) * 32767)
        frames += struct.pack("<hh", v, v)  # left = right = mono
    with wave.open(path, "wb") as w:
        w.setnchannels(2); w.setsampwidth(2); w.setframerate(RATE)
        w.writeframes(bytes(frames))

def kick_track(bpm, seconds, intro_silence=0.0, hats=False):
    n = int(RATE * seconds)
    out = [0.0] * n
    interval = 60.0 / bpm
    t = intro_silence
    while t < seconds - interval:
        start = int(t * RATE); dur = int(RATE * 0.03)
        for i in range(dur):
            env = 1.0 - i / dur
            out[start + i] = 0.9 * env * math.sin(2 * math.pi * 55.0 * i / RATE)
        if hats and False:  # placeholder if off-beat hats are added later
            pass
        t += interval
    return out

def pad_track(freqs, seconds):
    n = int(RATE * seconds)
    out = [0.0] * n
    for f in freqs:
        for i in range(n):
            a = min(1.0, i / (RATE * 0.01))  # 10 ms attack
            out[i] += 0.2 * a * math.sin(2 * math.pi * f * i / RATE)
    return out

NOTE = { "C": 261.63, "C#": 277.18, "D": 293.66, "D#": 311.13, "E": 329.63,
         "F": 349.23, "F#": 369.99, "G": 392.00, "G#": 415.30, "A": 440.00,
         "A#": 466.16, "B": 493.88 }

def triad(root_hz, minor=False):
    m3 = 2 ** (3 / 12) if minor else 2 ** (4 / 12)
    return [root_hz, root_hz * m3, root_hz * 2 ** (7 / 12)]

os.makedirs("openmix-core/tests/fixtures", exist_ok=True)
os.chdir("openmix-core/tests/fixtures")

for bpm in [70, 87, 100, 120, 128, 140, 174, 180]:
    write_wav(f"kick_{bpm}bpm.wav", kick_track(bpm, 24.0))
write_wav("kick_120bpm_intro.wav", kick_track(120, 24.0, intro_silence=0.87))
write_wav("kick_120bpm_hats.wav", kick_track(120, 24.0, hats=True))  # stereo variant below instead
write_stereo_wav("kick_120bpm_stereo.wav", kick_track(120, 24.0))

ROOTS = ["A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#"]
for root in ROOTS:
    write_wav(f"pad_{root}_major.wav", pad_track(triad(NOTE[root], minor=False), 8.0))
for root in ["A", "C", "D", "E", "F", "G"]:
    write_wav(f"pad_{root}_minor.wav", pad_track(triad(NOTE[root], minor=True), 8.0))
print("fixtures written")