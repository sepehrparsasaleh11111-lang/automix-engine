# OpenMix AI — Architecture

A free, open-source, fully-local automatic DJ mixing desktop application.

**Hard constraints (never violated):**
- All audio processing is local. User audio never leaves the device.
- No cloud processing, cloud storage, paid APIs, required accounts, or GPU servers.
- MIT licensed, open source on GitHub.

**Targets:** Windows 10/11 (x64, ARM64 when possible) and macOS (Apple Silicon + Intel).
Installers: `.exe` (Windows) and `.dmg` (macOS).
**Performance target:** modern 4+ core CPU, 16 GB RAM, SSD; projects up to 2 hours; chunked processing; no full-file RAM loading.

---

## 1. System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                        Frontend (React)                     │
│   Track library · Waveform + beat grid view · Mixer ·       │
│   Export · Project management                               │
└──────────────────────────┬──────────────────────────────────┘
                           │ Tauri IPC (commands + events)
┌──────────────────────────▼──────────────────────────────────┐
│                    openmix-app (Tauri 2)                    │
│   command layer · window/plugin wiring · app lifecycle      │
└──────────────────────────┬──────────────────────────────────┘
                           │
┌──────────────────────────▼──────────────────────────────────┐
│                     openmix-core (Rust)                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌───────────────┐  │
│  │ Analysis │ │ Beat Grid│ │ AutoMix  │ │   Rendering   │  │
│  │  (bpm,   │ │   (grid  │ │  engine  │ │  (export to   │  │
│  │ beats,   │ │  sync +  │ │ (match,  │ │  MP3/WAV/FLAC)│  │
│  │ key,     │ │ correct) │ │transitions│ │               │  │
│  │ energy)  │ └────┬─────┘ └────┬─────┘ └──────┬────────┘  │
│  └────┬─────┘      │            │              │           │
│       │            │            │              │           │
│  ┌────▼────────────▼────────────▼──────────────▼─────────┐ │
│  │               Audio Pipeline (decoder/stream)         │ │
│  │  symphonia · rubato time-stretch · resampling · EQ    │ │
│  └──────────────────────────┬────────────────────────────┘ │
│  ┌──────────────────────────▼────────────────────────────┐ │
│  │              Storage (rusqlite / SQLite)              │ │
│  │   projects · tracks · preferences · mix presets       │ │
│  └───────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────┘
        Audio files stay on the user's device (never copied to cloud)
```

**Crate boundaries:**

- `openmix-core` — engine crate. Zero Tauri dependencies. Every module is unit-testable
  with plain `cargo test`. CLI harness for headless analysis/render testing.
- `openmix-app` — Tauri 2 application crate. Thin: maps IPC commands to core APIs,
  owns the app event loop, plugin wiring.
- `frontend/` — Vite + React + TypeScript + Tailwind. Talks only to `openmix-app` IPC.

This split guarantees: the audio engine can be tested, benchmarked, and developed
without the GUI; the business logic never depends on the shell.

---

## 2. Module Design

### 2.1 Analysis (analysis/)

Interface-driven so every detector is replaceable:

```rust
trait TempoDetector { fn bpm(&self, stream: &DecodedStream) -> Option<f64>; }
trait OnsetDetector { fn onsets(&self, stream: &DecodedStream) -> Vec<Timestamp>; }
trait BeatTracker { fn beats(&self, stream: &DecodedStream) -> Vec<Beat>; }
trait KeyDetector { fn key(&self, stream: &DecodedStream) -> Option<KeyResult>; }
```

| Detector | Primary impl | Fallback / future |
|----------|--------------|-------------------|
| Tempo | aubio (`aubio-rs`) | pure-Rust autocorrelation |
| Onsets | aubio | pure-Rust spectral flux |
| Beats | aubio beat tracker | in-house histogram + grid fit |
| Key | KeyFinder (C++ bindings) | Krumhansl-Schmuckler chroma; confidence comparison between both |
| Loudness/energy | in-house (RMS + LUFS-style) | — |
| Sections/segments | in-house (novelty + energy structure) | — |
| Waveform peaks | in-house (min/max decimation) | — |

All analysis runs in a Rayon thread pool; a task can be cancelled.
Analysis results are cached in SQLite (per-track `analysis_cache` table);
re-analysis only when file hash or settings change.

### 2.2 Decoded Stream (audio/)

- `symphonia` decodes MP3, WAV, FLAC in **chunks** (e.g. 4096 frames).
- The stream adapter exposes: sample rate, channels, duration, and a
  callback/channel-based chunk consumer.
- Rendering and preview share the same streaming primitives; nothing ever loads
  a whole file into RAM. (2-hour mix requirement.)
- Waveform peaks are downsampled min/max pairs — a 2-hour file yields a small,
  zoomable peak table.

### 2.3 Beat Grid (beatgrid/)

The DJ-style beat grid is the backbone of all mixing.

- Models: `Beat { position, label: Downbeat|Beat }`, `Phase { start, length, kind: Intro|Break|Drop|Outro|... }`, `BpmCurve`.
- Grid fitting: start from aubio beats → fit uniform grid (default) or
  variable grid (tempo drift, live recordings).
- Grid state: `{ first_beat_offset, bpm, beat_interval, confidence, curve[] }`.
- Correction:
  - **Automatic**: re-fit with tighter windows when confidence is low;
    detect tempo drift by residual analysis.
  - **Manual**: the UI edits first-beat offset, BPM, and per-bar markers;
    every edit lands in SQLite and triggers re-fit of dependent data
    (grid markers, phrase estimates).
- Visualization data (markers) is generated by core and rendered by frontend
  canvas at any zoom level.

### 2.4 AutoMix Engine (automix/)

- **Beat matching:** compute tempo ratio between outgoing/incoming track,
  apply rubato time-stretch (pitch-preserving) via a replaceable
  `TimeStretcher` trait, align grids phase-locked; drift watchdog re-aligns.
- **Transition selection:** candidate transitions scored by
  `score = w1·bpm_compat + w2·key_compat (camelot wheel) + w3·energy_distance + w4·structural_fit + w5·history_penalty`.
  - Chooses transition point (section boundary), duration (8–32 beats),
    and track order.
- **Effect chain (per channel):** volume automation, EQ (3-band), bass swap,
  filter sweep, reverb tail (convolution or feedback delay), delay (tempo-synced).
  All effects are in-core DSP; parameterized by beat-aligned envelopes.
- Output routing: mix bus → preview live or render to disk.

### 2.5 Rendering (render/)

- Same streaming pipeline as preview, minus real-time constraint.
- Background task (own thread + progress channel → Tauri event → UI).
- Encoders: `symphonia` (WAV/FLAC) and a bundled MP3 encoder
  (`lame` via `lame-rs` bindings or pure-Rust MP3 encoder crate).
- Bounded memory: fixed-size ring of chunk buffers even for 2+ hour renders.
- Output policies: normalize (loudness-aware), dither, format/bitrate selection.

### 2.6 Storage (storage/)

- `rusqlite` (bundled SQLite). Tables: `projects`, `tracks`, `track_analysis`,
  `beat_grids`, `mix_presets`, `preferences`, `renders`.
- Audio files referenced by path on user's device — never copied.
- Storage module API only; schema migrations via a small versioned migration runner.

### 2.7 Preview & Playback

- Real-time-ish preview for auditioning transitions: render the mix buffer
  ahead of the playhead via a producer thread; Tauri side plays PCM through a
  native output (cpal or platform API). This is distinct from the offline
  render path and is lower priority than analysis/render.

---

## 3. Data Flow

1. **Import:** file path → decoder probe → metadata read → register `track`
   row → enqueue analysis task.
2. **Analyze:** stream chunks (rayon) → tempo/beats/key/energy/sections →
   persist `track_analysis` → emit `analysis:done` event → frontend renders
   waveform + beat grid.
3. **Mix:** user picks tracks (or AutoMix suggests) → core builds transition
   graph → user previews (live) or exports (offline render).
4. **Render:** `render:start` → chunk pipeline → encoder → `render:progress`
   events → final file + `renders` row; "recent mixes" screen reads this table.

---

## 4. Threading & Concurrency

- **tokio** runtime in `openmix-app` for async IPC and event fan-out.
- **rayon** pool for CPU-heavy analysis and render chunks.
- Render engine: dedicated worker thread owning the streaming pipeline;
  heartbeats as progress events; cancel via a `CancellationToken`.
- No shared-mutable state across threads: channels for chunk streaming,
  immutable analysis results behind `Arc`, UI state in frontend.

---

## 5. Error Handling

- Typed `AppError` in core (decode, analysis, grid, render, storage, io).
- IPC boundary: commands return `Result<T, AppError>`; frontend maps to
  user-facing messages with retry guidance.
- Render failures: partial output is discarded or kept per user choice;
  progress + error surfaced mid-render, not at the end.
- Analysis failures: per-field `Option`/confidence; a failed detector never
  blocks the rest of the pipeline.

---

## 6. Performance Strategy

- Chunked streaming everywhere; hard cap on resident buffers.
- Waveform peaks precomputed and cached (no re-scan on zoom).
- Analysis runs once per file (hash-keyed cache).
- Multi-threaded analysis across tracks; single-core-locked render pipeline
  for deterministic output (parallelism at chunk level where safe).
- Benchmarks in core (`cargo bench`) on fixture corpus: 2-hour render time,
  peak RAM, analysis wall time.

---

## 7. Testing Strategy

- **Core unit tests:** detectors against golden fixtures (synthetic tracks with
  known BPM/key; generated offline and checked in).
- **Core integration tests:** end-to-end analysis → grid → mix → render
  produces a file with expected duration/BPM alignment.
- **Quality gates:** ≥90% BPM accuracy on clean fixture suite (Gate 2);
  render output byte-compared against golden WAVs within tolerance.
- **Frontend:** Vitest (units, state) + Playwright (E2E against built app).
- **CI:** fmt + clippy + test + frontend build on every PR (macOS + Windows
  matrix), benches on schedule.

---

## 8. Phases & Gates

| Phase | Deliverable | Gate |
|-------|-------------|------|
| 0 | architecture.md, build guide, deps, folder structure, repo | docs committed, repo live, CI smoke |
| 1 | Tauri+React+SQLite skeleton, import MP3/WAV/FLAC, waveform, project CRUD | app opens & manages projects |
| 2 | analysis pipeline: BPM, beat grid (+correction), key, energy | ≥90% accuracy on fixture suite |
| 3 | AutoMix: beat match, transition selection, crossfade, EQ, effects | beat-matched auto-mix + manual grid edits |
| 4 | render to MP3/WAV/FLAC, background progress, 2-hour stability | stable long render, bounded RAM |
| 5 | polish, installers (.exe/.dmg), GitHub release, docs | release quality |

Build order rationale: foundation → understanding → mixing → exporting → polish.
Each phase ends with tests + a review stop.

---

## 9. Future AI Features (never required for base app)

AI transition/song-compatibility scoring, intelligent playlist mixing, neural
models — all optional local additions; may hook into the replaceable
detector/scorer traits. No cloud dependency, ever.

## 10. Open Source & Repository

- GitHub repo, MIT license, README, CONTRIBUTING, issue templates, CI.
- `architecture.md` is the living design doc; phase changes update it.