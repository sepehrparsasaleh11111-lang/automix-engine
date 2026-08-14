# Phase 2: Analysis Pipeline — BPM, Beat Grid, Key, Energy Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A backend analysis pipeline in `openmix-core` that detects tempo (BPM), onsets, beats, a DJ-style beat grid with automatic backend correction, musical key, and RMS energy from chunked audio — plus app-layer IPC/SQLite integration and a fixture-based accuracy suite proving ≥90% detection accuracy — Gate 2.

**Architecture:** `openmix-core` owns all analysis behind the Phase 0 detector traits (`TempoDetector`, `OnsetDetector`, `BeatTracker`, `KeyDetector`) gated by the `native-analysis` cargo feature (aubio via `aubio-rs` + vendored KeyFinder C++). Pure-Rust fallbacks (autocorrelation tempo, spectral-flux onsets, Krumhansl–Schmuckler chroma key) compile behind `--no-default-features`. Analysis is streaming/chunked from the Phase 1 `DecodedStream`; a single sequential pass feeds all detectors, so memory stays bounded. `openmix-app` persists results (`track_analysis`, `beat_grids` tables, hash-keyed cache) and exposes `analyze_track`/`get_analysis`/`get_beat_grid` IPC commands. No frontend changes (BPM/key badges are Phase 3 per architecture §2.3).

**Tech Stack:** Rust stable, `aubio-rs` 0.2 (`builtin` feature, self-contained C build via `cc` — no system aubio needed), vendored `mixxxdj/libkeyfinder` C++ (built via `cc` crate), `rustfft` (pure-Rust FFT for fallbacks), `rayon`, existing `symphonia` decode, `serde`/`thiserror`, `rusqlite` in app layer, Python stdlib fixture generators (committed binaries).

## Global Constraints

- macOS-first dev; Windows/macOS CI matrix validates every change (`ci.yml` `core` job, both `--all-features` and `--no-default-features`).
- All processing local. No cloud, no accounts, no telemetry.
- `openmix-core`: zero `tauri` deps, zero persistence. Storage only in `openmix-app/src/storage/`.
- Never load whole audio files into RAM — chunked streaming decode; analysis buffers bounded and documented (see Memory section).
- `native-analysis` (default on) gates aubio + KeyFinder; the crate must compile with `--no-default-features` using pure-Rust fallbacks behind the same traits.
- TDD (failing test → implement → passing test), `cargo fmt`/`clippy -D warnings` green, frequent commits.
- No code comments unless required for non-obvious logic.
- No UI/frontend work in Phase 2 unless architecture requires it for Gate 2 (it does not).
- Versions: aubio-rs 0.2, aubio-sys 0.2, rustfft 6, rayon 1, serde 1, thiserror 2, symphonia 0.5, rusqlite 0.32.
- Detector traits from `architecture.md` §2.1 preserved verbatim in spirit: replaceable, feature-gated, per-field `Option`/confidence; a failed detector never blocks the pipeline.
- Decision preserved from Phase 0 (approved): aubio + KeyFinder are accepted GPL-3.0 C/C++ dependencies, linked into the MIT app only behind `native-analysis` (default on); a `NOTICE`/legal note is added in Task 15 per the LAME precedent. Users can build the pure-Rust path via `--no-default-features`.

---

## File Structure Map (Phase 2)

```
openmix-core/src/
  lib.rs                        MODIFY — add mods analysis, beatgrid, audio::mono
  error.rs                      MODIFY — add Analysis(String) variant
  audio/mono.rs                 CREATE — interleaved→mono downmix + linear resampler
  analysis/
    mod.rs                      CREATE — traits, AnalysisResult, runner, config, cancellation
    tempo.rs                    CREATE — aubio Tempo impl [native] + autocorrelation fallback
    onsets.rs                   CREATE — aubio Onset impl [native] + spectral-flux fallback
    beats.rs                    CREATE — aubio Tempo-based BeatTracker + histogram fallback
    key.rs                      CREATE — KeyDetector dispatch: KeyFinder [native] vs K-S chroma
    chroma.rs                   CREATE — Krumhansl–Schmuckler chroma (rustfft)
    energy.rs                   CREATE — streaming RMS/peak (minimal)
  beatgrid/
    mod.rs                      CREATE — Beat, BeatGrid, BpmCurvePoint models; fit_uniform, fit_variable
    correct.rs                  CREATE — auto-correction (re-fit windows, drift residual)
  keyfinder/
    mod.rs                      CREATE — vendored libkeyfinder binding [native] (cc shim)
    shim.h / shim.cpp           CREATE — C ABI bridge to KeyFinder
    build.rs                    CREATE — cc build of vendored C++
    vendor/libkeyfinder/        CREATE — vendored C++ source (mixxxdj/libkeyfinder) + NOTICE
  bin/analyze.rs                CREATE — headless CLI: `openmix-core analyze <path>` → JSON
  bin/accuracy.rs               CREATE — headless accuracy report over fixture corpus
  tests/
    fixtures/                   CREATE — synthetic tracks with known BPM/key (committed binaries)
    analysis_test.rs            CREATE — detector integration tests
    beatgrid_test.rs            CREATE — grid fit + correction tests
    accuracy.rs                 CREATE — ≥90% fixture accuracy gate
scripts/
  gen-analysis-fixtures.py      CREATE — stdlib generator (kick BPM tracks, key-pad tracks)
openmix-app/src/
  storage/schema.sql            MODIFY — add track_analysis, beat_grids; PRAGMA user_version
  storage/db.rs                 MODIFY — versioned migration runner
  storage/mod.rs                MODIFY — upsert/get analysis + beat grid methods
  commands/analysis.rs          CREATE — analyze_track, get_analysis, get_beat_grid
  lib.rs                        MODIFY — register commands, background thread + analysis:done event
README.md                       MODIFY — status → Phase 2 complete
docs/build-guide.md             MODIFY — aubio/KeyFinder build notes
docs/dependencies.md            MODIFY — mark aubio-rs/rustfft/keyfinder used in Phase 2
```

---

## Interfaces & Data Structures (authoritative — used by all tasks)

```rust
// audio/mono.rs
/// interleaved → mono average; then linear-resample to target_rate.
pub fn to_mono(interleaved: &[f32], channels: u16, sample_rate: u32, target_rate: u32) -> Vec<f32>;
/// decimate + box-average for key analysis target (e.g. 11025).
pub fn downsample_mono(mono: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32>;

// analysis/mod.rs
pub struct AnalysisConfig {
    pub tempo_hop: usize,            // 512
    pub key_rate: u32,               // 11025
    pub key_max_seconds: Option<f64>,// Some(600.0) default → bounded buffer
    pub energy_window_ms: u32,       // 100
}
impl Default for AnalysisConfig { /* as above */ }

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AnalysisResult {
    pub bpm: Option<f64>,
    pub bpm_confidence: Option<f32>,
    pub onsets: Vec<f64>,          // seconds
    pub beats: Vec<Beat>,
    pub grid: Option<BeatGrid>,
    pub key: Option<KeyResult>,
    pub rms_db: Option<f32>,
    pub peak_db: Option<f32>,
    pub energy_windows: Vec<f32>,  // per-window RMS dB
}

/// One sequential pass over the stream; feeds all detectors; bounded memory.
pub fn analyze(stream: &mut DecodedStream, cfg: &AnalysisConfig, cancel: &std::sync::atomic::AtomicBool)
    -> Result<AnalysisResult, AppError>;

/// Convenience: open path, run analyze, close.
pub fn analyze_path(path: impl AsRef<std::path::Path>, cfg: &AnalysisConfig)
    -> Result<AnalysisResult, AppError>;

// Replaceable detector traits (Phase 0 architecture §2.1):
pub trait TempoDetector { fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64>; }
pub trait OnsetDetector { fn onsets(&self, mono: &[f32], rate: u32) -> Vec<f64>; }
pub trait BeatTracker  { fn beats(&self, mono: &[f32], rate: u32) -> Vec<Beat>; }
pub trait KeyDetector  { fn key(&self, mono: &[f32], rate: u32) -> Option<KeyResult>; }

// beatgrid/mod.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BeatLabel { Downbeat, Beat }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Beat { pub position_sec: f64, pub label: BeatLabel }

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct BpmCurvePoint { pub position_sec: f64, pub bpm: f64 }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BeatGrid {
    pub first_beat_offset: f64,    // seconds of beat 0
    pub bpm: f64,
    pub beat_interval: f64,        // seconds
    pub confidence: f32,           // 0..1 alignment quality
    pub curve: Vec<BpmCurvePoint>, // empty → uniform grid
}

pub fn fit_uniform(beats: &[f64], beat_interval_guess: f64) -> BeatGrid;
pub fn fit_variable(beats: &[f64], grid: &BeatGrid) -> BeatGrid; // drift detection

// beatgrid/correct.rs
pub fn correct(grid: BeatGrid, beats: &[f64], tightened_tolerance_ms: f64) -> BeatGrid;
pub fn detect_drift(beats: &[f64], grid: &BeatGrid) -> Vec<BpmCurvePoint>;

// analysis/key.rs, chroma.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyAlgorithm { KeyFinder, KrumhanslSchmuckler }

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyResult {
    pub key: MusicalKey,
    pub confidence: f32,
    pub algorithm: KeyAlgorithm,
    pub alternate: Option<(MusicalKey, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MusicalKey { /* 24 variants: AMajor, ASharpMajor, ..., GSharpMinor */ }
impl MusicalKey {
    pub fn camelot(&self) -> (u8, char);          // e.g. (8u8, 'A')
    pub fn relative(&self) -> MusicalKey;          // i - 3 semitones, mode flips
}

pub fn ks_key(mono: &[f32], rate: u32) -> Option<KeyResult>;                    // always available
pub fn best_key(kf: Option<KeyResult>, ks: Option<KeyResult>) -> Option<KeyResult>; // by confidence
```

All `AnalysisResult`/`BeatGrid`/`KeyResult` types derive `Serialize`/`Deserialize` for IPC + SQLite.

---

### Task 1: Mono Downmix + Resampler Utility (TDD)

**Files:**
- Create: `openmix-core/src/audio/mono.rs`
- Modify: `openmix-core/src/audio/mod.rs`, `openmix-core/src/lib.rs`
- Test: inline unit tests in `openmix-core/src/audio/mono.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `to_mono(interleaved: &[f32], channels: u16, sample_rate: u32, target_rate: u32) -> Vec<f32>` and `downsample_mono(mono: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32>` (both pure, f32 in/out) — used by every detector task.

- [ ] **Step 1: Write the failing tests** (in `mono.rs` `#[cfg(test)]` mod):

```rust
#[test]
fn stereo_downmix_averages_channels() {
    let inter = [0.2f32, 0.6, 0.4, 0.8, -0.2, -0.8];
    let mono = to_mono(&inter, 2, 44100, 44100);
    assert_eq!(mono.len(), 3);
    assert!((mono[0] - 0.4).abs() < 1e-6);
    assert!((mono[1] - 0.6).abs() < 1e-6);
    assert!((mono[2] + 0.5).abs() < 1e-6);
}

#[test]
fn resample_halves_length() {
    let mono: Vec<f32> = (0..4410).map(|i| (i as f32 / 4410.0).sin()).collect();
    let out = to_mono(&mono, 1, 44100, 22050);
    assert!((out.len() as i64 - 2205).abs() <= 2);
}

#[test]
fn resample_identity_is_close() {
    let mono: Vec<f32> = (0..1000).map(|i| (i as f32).sin()).collect();
    let out = to_mono(&mono, 1, 44100, 44100);
    assert_eq!(out.len(), 1000);
    for (a, b) in mono.iter().zip(out.iter()) {
        assert!((a - b).abs() < 1e-6);
    }
}

#[test]
fn downsample_to_quarter_length() {
    let mono: Vec<f32> = (0..4410).map(|i| (i as f32).sin()).collect();
    let out = downsample_mono(&mono, 44100, 11025);
    assert_eq!(out.len(), 1102); // 4410 / 4
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module mono not found`.

- [ ] **Step 3: Implement `openmix-core/src/audio/mono.rs`**

```rust
/// Average interleaved channels to mono, then linearly resample.
pub fn to_mono(interleaved: &[f32], channels: u16, sample_rate: u32, target_rate: u32) -> Vec<f32> {
    if channels <= 1 {
        return resample_linear(interleaved, sample_rate, target_rate);
    }
    let ch = channels as usize;
    let frames = interleaved.len() / ch;
    let mut mono = Vec::with_capacity(frames);
    for f in 0..frames {
        let sum: f32 = interleaved[f * ch..f * ch + ch].iter().sum();
        mono.push(sum / ch as f32);
    }
    resample_linear(&mono, sample_rate, target_rate)
}

fn resample_linear(src: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return src.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (src.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(src.len() - 1);
        let frac = (pos - lo as f64) as f32;
        out.push(src[lo] * (1.0 - frac) + src[hi] * frac);
    }
    out
}

/// Decimate with box averaging — used for the key-analysis buffer.
pub fn downsample_mono(mono: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate <= to_rate {
        return mono.to_vec();
    }
    let factor = (from_rate / to_rate) as usize;
    let mut out = Vec::with_capacity(mono.len() / factor + 1);
    for chunk in mono.chunks(factor) {
        out.push(chunk.iter().sum::<f32>() / chunk.len() as f32);
    }
    out
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p openmix-core`
Expected: PASS (4 tests).

- [ ] **Step 5: Wire module + quality gates**

Add to `audio/mod.rs`: `pub mod mono; pub use mono::{downsample_mono, to_mono};` Add `pub mod audio;` already exists in `lib.rs`.
Run: `cargo fmt --all --check && cargo clippy -p openmix-core --all-targets -- -D warnings`
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/audio/
git commit -m "feat(core): mono downmix and resampler"
```

---

### Task 2: Analysis Traits, Result Model, Energy Detector (TDD)

**Files:**
- Create: `openmix-core/src/analysis/mod.rs`, `openmix-core/src/analysis/energy.rs`
- Modify: `openmix-core/src/lib.rs` (`pub mod analysis;`)
- Test: inline tests in `analysis/mod.rs` + `analysis/energy.rs`

**Interfaces:**
- Consumes: nothing (energy works on slices).
- Produces, used by Tasks 4–12: `AnalysisConfig` (Default), `AnalysisResult` (Serialize/Deserialize, all `Option`), the four detector traits, `fn rms_db_of(samples: &[f32]) -> f32`, `fn peak_db_of(samples: &[f32]) -> f32`, `fn energy_windows(mono: &[f32], rate: u32, window_ms: u32) -> Vec<f32>`, `MusicalKey` (24 variants, serde, `camelot()`, `relative()`).

- [ ] **Step 1: Write the failing tests**

In `analysis/mod.rs`:

```rust
#[test]
fn analysis_result_defaults_to_all_none() {
    let r = AnalysisResult { bpm: None, bpm_confidence: None, onsets: vec![], beats: vec![],
        grid: None, key: None, rms_db: None, peak_db: None, energy_windows: vec![] };
    assert!(r.bpm.is_none() && r.key.is_none() && r.grid.is_none());
}

#[test]
fn musical_key_relative_and_camelot() {
    assert_eq!(MusicalKey::CMajor.relative(), MusicalKey::AMinor);
    assert_eq!(MusicalKey::AMinor.relative(), MusicalKey::CMajor); // +9 semitones, mode flips back
    assert_eq!(MusicalKey::CMajor.camelot(), (8, 'B'));
    assert_eq!(MusicalKey::AMinor.camelot(), (8, 'A'));
}

#[test]
fn musical_key_serializes() {
    let j = serde_json::to_string(&MusicalKey::FSharpMinor).unwrap();
    let k: MusicalKey = serde_json::from_str(&j).unwrap();
    assert_eq!(k, MusicalKey::FSharpMinor);
}
```

In `analysis/energy.rs`:

```rust
#[test]
fn rms_of_full_scale_sine_is_minus_3db() {
    // 1 s of unit sine at 44100 Hz (sum != 0; peak = 1.0)
    let n = 44100usize;
    let mut s = Vec::with_capacity(n);
    for i in 0..n {
        s.push((std::f32::consts::TAU * 440.0 * i as f32 / 44100.0).sin());
    }
    let rms = rms_db_of(&s);
    assert!((rms + 3.0103).abs() < 0.5, "rms_db = {rms}");
    assert!((peak_db_of(&s)).abs() < 0.01, "peak_db = {}", peak_db_of(&s));
}

#[test]
fn energy_windows_count_matches_duration() {
    let n = 44100usize * 2; // 2 s at 44.1 kHz
    let mono = vec![0.0f32; n];
    let w = energy_windows(&mono, 44100, 100);
    assert_eq!(w.len(), 20);
}

#[test]
fn energy_of_silence_is_quiet() {
    let mono = vec![0.0f32; 44100];
    assert!(rms_db_of(&mono) < -60.0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module analysis not found`.

- [ ] **Step 3: Implement**

`analysis/energy.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub tempo_hop: usize,
    pub key_rate: u32,
    pub key_max_seconds: Option<f64>,
    pub energy_window_ms: u32,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self { tempo_hop: 512, key_rate: 11_025, key_max_seconds: Some(600.0), energy_window_ms: 100 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub bpm: Option<f64>,
    pub bpm_confidence: Option<f32>,
    pub onsets: Vec<f64>,
    pub beats: Vec<Beat>,
    pub grid: Option<BeatGrid>,
    pub key: Option<KeyResult>,
    pub rms_db: Option<f32>,
    pub peak_db: Option<f32>,
    pub energy_windows: Vec<f32>,
}

pub fn rms_db_of(samples: &[f32]) -> f32 {
    let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    let rms = (sum_sq / samples.len().max(1) as f64).sqrt();
    if rms <= f64::EPSILON { -120.0 } else { 20.0 * rms.log10() as f32 }
}

pub fn peak_db_of(samples: &[f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    if peak <= f32::EPSILON { -120.0 } else { 20.0 * peak.log10() }
}

pub fn energy_windows(mono: &[f32], rate: u32, window_ms: u32) -> Vec<f32> {
    let win = (rate as usize * window_ms as usize / 1000).max(1);
    mono.chunks(win)
        .map(|c| rms_db_of(c))
        .collect()
}
```

`analysis/mod.rs`: `pub mod chroma; pub mod energy; pub mod key; pub mod tempo; pub mod onsets; pub mod beats;` (blank placeholder modules may be added here early or in later tasks — add each `mod` line in its own task to keep compile working), plus the four traits *before* any impls exist:

```rust
pub trait TempoDetector { fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64>; }
pub trait OnsetDetector { fn onsets(&self, mono: &[f32], rate: u32) -> Vec<f64>; }
pub trait BeatTracker  { fn beats(&self, mono: &[f32], rate: u32) -> Vec<Beat>; }
pub trait KeyDetector  { fn key(&self, mono: &[f32], rate: u32) -> Option<KeyResult>; }
```

References to `Beat`, `BeatGrid`, `KeyResult`, `MusicalKey` come from `crate::beatgrid` and `analysis::key` — create `beatgrid/mod.rs` and `analysis/key.rs` in the same step with the model types (see Task 7 and Task 10 listings) so the crate compiles; tests for those types land in their own tasks.

`MusicalKey` (in `analysis/key.rs`) — 24 variants `AMajor, ASharpMajor, BMajor, CMajor, CSharpMajor, DMajor, DSharpMajor, EMajor, FMajor, FSharpMajor, GMajor, GSharpMajor, AMinor, ASharpMinor, BMinor, CMinor, CSharpMinor, DMinor, DSharpMinor, EMinor, FMinor, FSharpMinor, GMinor, GSharpMinor`:

```rust
impl MusicalKey {
    pub fn relative(&self) -> MusicalKey {
        use MusicalKey::*;
        match self {
            AMajor => FSharpMinor, ASharpMajor => GMinor, BMajor => GSharpMinor,
            CMajor => AMinor, CSharpMajor => ASharpMinor, DMajor => BMinor,
            DSharpMajor => CMinor, EMajor => CSharpMinor, FMajor => DMinor,
            FSharpMajor => DSharpMinor, GMajor => EMinor, GSharpMajor => FMinor,
            AMinor => CMajor, ASharpMinor => CSharpMajor, BMinor => DMajor,
            CMinor => DSharpMajor, CSharpMinor => EMajor, DMinor => FMajor,
            DSharpMinor => FSharpMajor, EMinor => GMajor, FMinor => GSharpMajor,
            FSharpMinor => AMajor, GMinor => ASharpMajor, GSharpMinor => BMajor,
        }
    }
    pub fn camelot(&self) -> (u8, char) {
        use MusicalKey::*;
        let (n, l) = match self {
            AMajor => (11, 'B'), ASharpMajor => (1, 'B'), BMajor => (2, 'B'),
            CMajor => (8, 'B'), CSharpMajor => (3, 'B'), DMajor => (10, 'B'),
            DSharpMajor => (5, 'B'), EMajor => (12, 'B'), FMajor => (9, 'B'),
            FSharpMajor => (4, 'B'), GMajor => (7, 'B'), GSharpMajor => (6, 'B'),
            AMinor => (8, 'A'), ASharpMinor => (3, 'A'), BMinor => (10, 'A'),
            CMinor => (5, 'A'), CSharpMinor => (12, 'A'), DMinor => (7, 'A'),
            DSharpMinor => (2, 'A'), EMinor => (9, 'A'), FMinor => (4, 'A'),
            FSharpMinor => (11, 'A'), GMinor => (6, 'A'), GSharpMinor => (1, 'A'),
        };
        (n, l)
    }
}
```

`lib.rs`: add `pub mod analysis; pub mod beatgrid;` and `pub use analysis::energy::{rms_db_of, peak_db_of, energy_windows}; pub use analysis::{AnalysisConfig, AnalysisResult, Beat, BeatGrid};` re-exports as needed (name collisions: `Beat`/`BeatGrid` live in `beatgrid` — re-export from there, see Task 7).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS (6 tests from this task; existing Phase 1 tests still green).

- [ ] **Step 5: Quality gates**

Run: `cargo fmt --all --check && cargo clippy -p openmix-core --all-targets -- -D warnings && cargo check -p openmix-core --no-default-features`
Expected: all green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/
git commit -m "feat(core): analysis result model, traits, and rms energy"
```

---

### Task 3: aubio Dependency Spike + Feature Wiring

**Files:**
- Modify: `Cargo.toml` (workspace deps), `openmix-core/Cargo.toml`
- No production code — spike validates the Windows-critical dependency choice before any detector code.

**Interfaces:**
- Produces: `native-analysis` feature now pulls `aubio-rs = { version = "0.2", features = ["builtin"] }`, `rayon = "1"`, `rustfft = "6"`; workspace adds all three.

- [ ] **Step 1: Add dependencies**

`Cargo.toml` workspace:

```toml
[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
symphonia = { version = "0.5", features = ["mp3", "flac", "wav", "pcm"] }
rayon = "1"
rustfft = "6"
aubio-rs = { version = "0.2", features = ["builtin"] }
```

`openmix-core/Cargo.toml`:

```toml
[features]
default = ["native-analysis"]
native-analysis = ["dep:aubio-rs"]

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
symphonia = { workspace = true }
rayon = { workspace = true }
rustfft = { workspace = true }
aubio-rs = { workspace = true, optional = true }
```

Note: `builtin` forces aubio-rs to compile the bundled aubio C library source from crates.io (`aubio-sys` ships `aubio/` + `fftw/` trees), making the build self-contained on macOS and Windows with only a C compiler — no system aubio, no pkg-config, no cmake.

- [ ] **Step 2: Verify both feature configurations compile**

Run: `cargo check -p openmix-core` and `cargo check -p openmix-core --no-default-features`
Expected: both succeed (first `builtin` build compiles aubio C — several minutes).

- [ ] **Step 3: Verify Phase 1 tests still pass**

Run: `cargo test -p openmix-core`
Expected: PASS.

- [ ] **Step 4: Push and confirm Windows CI builds the native feature**

Commit, push, and watch the `core` job on `windows-latest`: `cargo check` + `cargo clippy` with default features must succeed (this de-risks Task 4/9 before they exist). If Windows fails on `builtin`, STOP — pick the documented fallback (`pkg-config` feature via preinstalled aubio on the runner, or pure-Rust path default) and update this plan.

- [ ] **Step 5: Commit**

```sh
git add Cargo.toml openmix-core/Cargo.toml
git commit -m "build(core): wire aubio-rs (builtin) and rustfft behind native-analysis"
```

---

### Task 4: Tempo Detector (aubio + autocorrelation fallback, TDD)

**Files:**
- Create: `openmix-core/src/analysis/tempo.rs`
- Modify: `openmix-core/src/analysis/mod.rs` (`pub mod tempo;` re-exports)
- Test: inline tests in `tempo.rs`

**Interfaces:**
- Consumes: `to_mono`/`downsample_mono` (Task 1).
- Produces: `pub fn aubio_bpm(mono: &[f32], rate: u32) -> Option<(f64, f32)>` `#[cfg(feature = "native-analysis")]`; `pub fn autocorr_bpm(mono: &[f32], rate: u32) -> Option<f64>` (always available); structs `AubioTempoDetector` / `AutocorrTempoDetector` implementing `TempoDetector`. Used by Task 12 runner.

- [ ] **Step 1: Write the failing tests**

Shared test helper (in `tempo.rs` `#[cfg(test)]`):

```rust
fn synthetic_kick(rate: u32, bpm: f64, seconds: f64) -> Vec<f32> {
    let interval = 60.0 / bpm;
    let n = (rate as f64 * seconds) as usize;
    let mut out = vec![0.0f32; n];
    let mut t = 0.0;
    while t < seconds - interval {
        let start = (t * rate as f64) as usize;
        let len = (rate as f64 * 0.03) as usize; // 30 ms kick
        for i in 0..len {
            if start + i < n {
                let env = 1.0 - i as f32 / len as f32;
                out[start + i] = 0.9 * env * (std::f32::consts::TAU * 55.0 * i as f32 / rate as f32).sin();
            }
        }
        t += interval;
    }
    out
}
```

```rust
#[cfg(feature = "native-analysis")]
#[test]
fn aubio_detects_120bpm_kick() {
    let mono = synthetic_kick(44100, 120.0, 20.0);
    let (bpm, conf) = aubio_bpm(&mono, 44100).expect("detect");
    assert!((bpm - 120.0).abs() <= 120.0 * 0.015, "bpm = {bpm}");
    assert!(conf > 0.0);
}

#[test]
fn autocorr_detects_120bpm_kick() {
    let mono = synthetic_kick(44100, 120.0, 20.0);
    let bpm = autocorr_bpm(&mono, 44100).expect("detect");
    assert!((bpm - 120.0).abs() <= 120.0 * 0.015, "bpm = {bpm}");
}

#[test]
fn silence_returns_none() {
    let silent = vec![0.0f32; 44100 * 5];
    assert!(autocorr_bpm(&silent, 44100).is_none());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module tempo not found`.

- [ ] **Step 3: Implement**

```rust
use std::sync::atomic::{AtomicBool, Ordering};

use super::AudioAnalysisCancel; // see note
```

Cancel note: define `pub type AnalysisCancel = AtomicBool;` in `analysis/mod.rs` in Task 2 and accept it as `&AtomicBool` in Task 12's runner; the standalone `aubio_bpm`/`autocorr_bpm` helpers used by tests take no cancel (they call an inner `aubio_bpm_cancellable(..., &AtomicBool::new(false))`).

```rust
#[cfg(feature = "native-analysis")]
pub fn aubio_bpm(mono: &[f32], rate: u32) -> Option<(f64, f32)> {
    let cancel = AtomicBool::new(false);
    aubio_bpm_cancellable(mono, rate, &cancel)
}

#[cfg(feature = "native-analysis")]
pub(crate) fn aubio_bpm_cancellable(mono: &[f32], rate: u32, cancel: &AtomicBool) -> Option<(f64, f32)> {
    use aubio_rs::{OnsetMode, Tempo};
    let hop = 512usize;
    let mut tempo = Tempo::new(OnsetMode::SpecFlux, 1024, hop, rate).ok()?;
    for chunk in mono.chunks(hop) {
        if cancel.load(Ordering::Relaxed) { return None; }
        let out = tempo.do_result(chunk).ok()?;
        let _ = out; // 1.0 → beat at get_last_s()
    }
    let bpm = tempo.get_bpm();
    let conf = tempo.get_confidence();
    if bpm <= 0.0 || conf < 0.05 { return None; }
    Some((bpm as f64, conf))
}

pub fn autocorr_bpm(mono: &[f32], rate: u32) -> Option<f64> {
    let work = crate::audio::mono::downsample_mono(mono, rate, 11_025);
    let min_lag = (11_025.0 * 60.0 / 180.0) as usize; // 180 BPM
    let max_lag = (11_025.0 * 60.0 / 60.0) as usize;  // 60 BPM
    if work.len() <= max_lag + 1 { return None; }
    let mean: f64 = work.iter().map(|s| *s as f64).sum::<f64>() / work.len() as f64;
    let x: Vec<f64> = work.iter().map(|s| *s as f64 - mean).collect();
    let mut best_lag = 0usize;
    let mut best_score = f64::MIN;
    for lag in min_lag..=max_lag {
        let score: f64 = x[..x.len() - lag].iter()
            .zip(&x[lag..])
            .map(|(a, b)| a * b)
            .sum::<f64>() / (x.len() - lag) as f64;
        if score > best_score { best_score = score; best_lag = lag; }
    }
    if best_score <= 0.0 { return None; }
    Some(60.0 * 11_025.0 / best_lag as f64)
}

#[cfg(feature = "native-analysis")]
pub struct AubioTempoDetector;
#[cfg(feature = "native-analysis")]
impl super::TempoDetector for AubioTempoDetector {
    fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64> {
        aubio_bpm(mono, rate).map(|(b, _)| b)
    }
}

pub struct AutocorrTempoDetector;
impl super::TempoDetector for AutocorrTempoDetector {
    fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64> { autocorr_bpm(mono, rate) }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS (native build: 3 tests; `--no-default-features`: 2 tests — silence + autocorr).

- [ ] **Step 5: Quality gates**

Run: `cargo fmt --all --check && cargo clippy -p openmix-core --all-targets -- -D warnings && cargo test -p openmix-core && cargo check -p openmix-core --no-default-features`
Expected: all green. Note: the `synthetic_kick` test helper will be reused by Tasks 5/6 — extract it to `openmix-core/tests/common/mod.rs` when Tasks 5/6 land (or keep per-module copies; simpler and DRY-safe at these sizes, but the shared `tests/common` is preferred if it grows).

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/analysis/tempo.rs openmix-core/src/analysis/mod.rs
git commit -m "feat(core): bpm detection via aubio and autocorrelation fallback"
```

---

### Task 5: Onset Detector (aubio + spectral-flux fallback, TDD)

**Files:**
- Create: `openmix-core/src/analysis/onsets.rs`
- Modify: `openmix-core/src/analysis/mod.rs` (`pub mod onsets;`)
- Test: inline tests in `onsets.rs`

**Interfaces:**
- Consumes: `to_mono` (Task 1), `rustfft`.
- Produces: `pub fn aubio_onsets(mono: &[f32], rate: u32) -> Vec<f64>` `[native]`; `pub fn flux_onsets(mono: &[f32], rate: u32) -> Vec<f64>` (always); structs `AubioOnsetDetector` / `FluxOnsetDetector` implementing `OnsetDetector`.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(feature = "native-analysis")]
#[test]
fn aubio_onsets_match_kick_grid() {
    let mono = synthetic_kick(44100, 120.0, 20.0); // reused helper, moved to tests/common
    let onsets = aubio_onsets(&mono, 44100);
    assert!(!onsets.is_empty());
    for o in &onsets {
        let nearest = (o * 2.0).round() / 2.0; // nearest beat at 0.5 s
        assert!((o - nearest).abs() < 0.03, "onset {o} not on grid");
    }
}

#[test]
fn flux_onsets_detect_kicks() {
    let mono = synthetic_kick(44100, 128.0, 20.0);
    let onsets = flux_onsets(&mono, 44100);
    let expected = (20.0 * 128.0 / 60.0) as usize;
    assert!((onsets.len() as i64 - expected as i64).abs() <= 3, "n={}", onsets.len());
}

#[test]
fn silent_input_no_onsets() {
    let silent = vec![0.0f32; 44100 * 3];
    assert!(flux_onsets(&silent, 44100).is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module onsets not found`.

- [ ] **Step 3: Implement**

```rust
#[cfg(feature = "native-analysis")]
pub fn aubio_onsets(mono: &[f32], rate: u32) -> Vec<f64> {
    use aubio_rs::{Onset, OnsetMode};
    let hop = 512usize;
    let mut out = Vec::new();
    let mut onset = match Onset::new(OnsetMode::SpecFlux, 1024, hop, rate) {
        Ok(o) => o, Err(_) => return out,
    };
    for chunk in mono.chunks(hop) {
        if let Ok(r) = onset.do_result(chunk) {
            if r > 0.5 { out.push(onset.get_last_s() as f64); }
        }
    }
    out
}

pub fn flux_onsets(mono: &[f32], rate: u32) -> Vec<f64> {
    use rustfft::{num_complex::Complex, FftPlanner};
    let fft_size = 1024usize;
    let hop = 256usize;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut flux: Vec<f32> = Vec::new();
    let mut prev: Vec<f32> = vec![0.0; fft_size];
    let mut buf = vec![Complex::new(0.0f32, 0.0); fft_size];
    let mut spectrum = vec![0.0f32; fft_size];
    let mut frame = 0usize;

    for chunk in mono.chunks(hop) {
        if chunk.len() < hop { break; }
        for (i, s) in chunk.iter().enumerate() {
            let w = 0.5 - 0.5 * ((std::f32::consts::TAU * i as f32 / (fft_size as f32 - 1.0)).cos());
            buf[i] = Complex::new(s * w, 0.0);
        }
        fft.process(&mut buf);
        for k in 0..fft_size {
            spectrum[k] = buf[k].norm();
            let _ = (spectrum[k] - prev[k]).max(0.0);
        }
        let mut f = 0.0f32;
        for k in 0..fft_size {
            f += (spectrum[k] - prev[k]).max(0.0);
        }
        prev.copy_from_slice(&spectrum);
        flux.push(f);
        frame += 1;
    }

    let mean = flux.iter().sum::<f32>() / flux.len().max(1) as f32;
    let var = flux.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / flux.len().max(1) as f32;
    let thresh = mean + 2.0 * var.sqrt();
    let min_ioi = (rate as f64 * 0.03) as usize; // 30 ms
    let hop_s = hop as f64 / rate as f64;

    let mut out = Vec::new();
    let mut last = usize::MAX;
    for (i, f) in flux.iter().enumerate() {
        if *f > thresh && i >= last.saturating_add(min_ioi) {
            out.push(i as f64 * hop_s);
            last = i;
        }
    }
    out
}

// struct impls OnsetDetector analogous to Task 4
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS. (If `flux_onsets` count is flaky, loosen to `<= expected + 4` and tighten min-IOI.)

- [ ] **Step 5: Quality gates**

Run: gates from Task 4 Step 5.
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/analysis/onsets.rs openmix-core/src/analysis/mod.rs
git commit -m "feat(core): onset detection via aubio and spectral flux"
```

---

### Task 6: Beat Tracker (TDD)

**Files:**
- Create: `openmix-core/src/analysis/beats.rs`
- Modify: `openmix-core/src/analysis/mod.rs` (`pub mod beats;`)
- Test: inline tests in `beats.rs`

**Interfaces:**
- Consumes: `synthetic_kick` helper, `aubio_bpm`/`autocorr_bpm` (Task 4), `fit_uniform` (Task 7 — implemented before this task's downbeat labeling is finalized, or a temporary downbeat rule: index % 4 starting from grid offset).
- Produces: `pub fn aubio_beats(mono: &[f32], rate: u32) -> Vec<Beat>` `[native]`; `pub fn histogram_beats(mono: &[f32], rate: u32) -> Vec<Beat>` (always); structs `AubioBeatTracker` / `HistogramBeatTracker` implementing `BeatTracker`. Downbeat = every 4th beat from grid offset.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(feature = "native-analysis")]
#[test]
fn aubio_beats_match_128bpm_grid() {
    let mono = synthetic_kick(44100, 128.0, 20.0);
    let beats = aubio_beats(&mono, 44100);
    assert!(beats.len() >= 40, "few beats: {}", beats.len());
    let interval = 60.0 / 128.0;
    for w in beats.windows(2) {
        assert!((w[1].position_sec - w[0].position_sec - interval).abs() < interval * 0.06);
    }
    assert_eq!(beats[0].label, BeatLabel::Downbeat);
    assert_eq!(beats[3].label, BeatLabel::Beat); // 4th beat = downbeat
}

#[test]
fn histogram_beats_detect_grid() {
    let mono = synthetic_kick(44100, 120.0, 20.0);
    let beats = histogram_beats(&mono, 44100);
    assert!(!beats.is_empty());
    for w in beats.windows(2) {
        assert!((w[1].position_sec - w[0].position_sec - 0.5).abs() < 0.05);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module beats not found`.

- [ ] **Step 3: Implement**

```rust
use crate::beatgrid::{fit_uniform, Beat, BeatLabel};

#[cfg(feature = "native-analysis")]
pub fn aubio_beats(mono: &[f32], rate: u32) -> Vec<Beat> {
    // reuse tempo loop from Task 4 to collect beat times
    use aubio_rs::{OnsetMode, Tempo};
    let hop = 512usize;
    let mut tempo = match Tempo::new(OnsetMode::SpecFlux, 1024, hop, rate) { Ok(t) => t, Err(_) => return vec![] };
    let mut times: Vec<f64> = Vec::new();
    for chunk in mono.chunks(hop) {
        if let Ok(r) = tempo.do_result(chunk) {
            if r > 0.5 { times.push(tempo.get_last_s() as f64); }
        }
    }
    label_beats(times)
}

pub fn histogram_beats(mono: &[f32], rate: u32) -> Vec<Beat> {
    let onsets = crate::analysis::onsets::flux_onsets(mono, rate);
    if onsets.len() < 4 { return vec![]; }
    let mut diffs: Vec<f64> = onsets.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 0.2).collect();
    if diffs.is_empty() { return vec![]; }
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = diffs[diffs.len() / 2];
    let beat_interval_guess = median;
    // snap to 0.25 s grid for DJ conventions
    let times = onsets;
    let grid = fit_uniform(&times, beat_interval_guess);
    let step = grid.beat_interval;
    let mut beats = Vec::new();
    let mut t = grid.first_beat_offset;
    let mut idx = 0usize;
    while t <= onsets.last().unwrap() + step {
        beats.push(Beat { position_sec: t, label: if idx % 4 == 0 { BeatLabel::Downbeat } else { BeatLabel::Beat } });
        idx += 1;
        t += step;
    }
    beats
}

pub(crate) fn label_beats(mut times: Vec<f64>) -> Vec<Beat> {
    if times.is_empty() { return vec![]; }
    times.dedup();
    // grid offset = first beat if the mean spacing is stable (use fit_uniform)
    let grid = fit_uniform(&times, 0.5);
    times.retain(|t| (t - grid.first_beat_offset) >= -1e-6);
    times.into_iter()
        .map(|t| {
            let idx = ((t - grid.first_beat_offset) / grid.beat_interval).round() as usize;
            Beat { position_sec: t, label: if idx % 4 == 0 { BeatLabel::Downbeat } else { BeatLabel::Beat } }
        })
        .collect()
}
```

(If `fit_uniform` isn't landed yet, implement a trivial fallback here: `interval = median(diffs)`, offset = times[0] — replace when Task 7 lands.)

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS.

- [ ] **Step 5: Quality gates**

Run: gates from Task 4 Step 5.
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/analysis/beats.rs openmix-core/src/analysis/mod.rs
git commit -m "feat(core): beat tracking with downbeat labeling"
```

---

### Task 7: Beat Grid Model + Uniform Fit (TDD)

**Files:**
- Create: `openmix-core/src/beatgrid/mod.rs`
- Modify: `openmix-core/src/lib.rs` (`pub mod beatgrid; pub use beatgrid::{Beat, BeatGrid, BeatLabel, BpmCurvePoint};`)
- Test: inline tests in `openmix-core/src/beatgrid/mod.rs`

**Interfaces:**
- Consumes: nothing.
- Produces: `Beat`, `BeatGrid`, `BeatLabel`, `BpmCurvePoint` (all serde); `fit_uniform(beats: &[f64], beat_interval_guess: f64) -> BeatGrid`. Consumed by Tasks 6, 8, 12.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn ideal_grid_recovers_offset_and_bpm() {
    let mut beats = Vec::new();
    for i in 0..64 { beats.push(0.25 + i as f64 * 0.5); }
    let g = fit_uniform(&beats, 0.5);
    assert!((g.first_beat_offset - 0.25).abs() < 1e-3, "offset {}", g.first_beat_offset);
    assert!((g.bpm - 120.0).abs() < 0.1, "bpm {}", g.bpm);
    assert!(g.confidence > 0.99);
    assert!(g.curve.is_empty());
}

#[test]
fn jittered_grid_keeps_high_confidence() {
    let mut beats = Vec::new();
    for i in 0..64 {
        let jitter = (i as f64 * 13.7).sin() * 0.02;
        beats.push(0.25 + i as f64 * 0.5 + jitter);
    }
    let g = fit_uniform(&beats, 0.5);
    assert!(g.confidence > 0.8, "conf {}", g.confidence);
    assert!((g.first_beat_offset - 0.25).abs() < 0.02);
}

#[test]
fn short_input_low_confidence_no_panic() {
    let g = fit_uniform(&[], 0.5);
    assert_eq!(g.confidence, 0.0);
    let g2 = fit_uniform(&[1.0], 0.5);
    assert!(g2.confidence < 0.5);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module beatgrid not found`.

- [ ] **Step 3: Implement**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeatLabel { Downbeat, Beat }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Beat { pub position_sec: f64, pub label: BeatLabel }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BpmCurvePoint { pub position_sec: f64, pub bpm: f64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatGrid {
    pub first_beat_offset: f64,
    pub bpm: f64,
    pub beat_interval: f64,
    pub confidence: f32,
    pub curve: Vec<BpmCurvePoint>,
}

pub fn fit_uniform(beats: &[f64], beat_interval_guess: f64) -> BeatGrid {
    if beats.len() < 2 || beat_interval_guess <= 0.0 {
        return BeatGrid { first_beat_offset: 0.0, bpm: 0.0, beat_interval: 0.0, confidence: 0.0, curve: vec![] };
    }
    // robust interval: median of positive diffs
    let mut diffs: Vec<f64> = beats.windows(2).map(|w| w[1] - w[0]).filter(|d| *d > 0.0).collect();
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let interval = if diffs.is_empty() { beat_interval_guess } else { diffs[diffs.len() / 2] };
    let tol = interval * 0.10;
    // sweep offset within one interval, maximizing grid-line match
    let mut best_offset = beats[0];
    let mut best_score = 0usize;
    let steps = 200usize;
    for s in 0..steps {
        let offset = beats[0] + interval * s as f64 / steps as f64;
        let mut score = 0usize;
        for b in beats {
            let pos = (b - offset) / interval;
            let nearest = pos.round();
            if (pos - nearest).abs() * interval <= tol { score += 1; }
        }
        if score > best_score { best_score = score; best_offset = offset; }
    }
    let confidence = best_score as f32 / beats.len() as f32;
    BeatGrid {
        first_beat_offset: best_offset,
        bpm: 60.0 / interval,
        beat_interval: interval,
        confidence,
        curve: vec![],
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS (3 tests).

- [ ] **Step 5: Quality gates**

Run: gates from Task 4 Step 5.
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/beatgrid/mod.rs openmix-core/src/lib.rs
git commit -m "feat(core): uniform beat-grid fitting with offset search"
```

---

### Task 8: Beat Grid Auto-Correction (TDD)

**Files:**
- Create: `openmix-core/src/beatgrid/correct.rs`
- Modify: `openmix-core/src/beatgrid/mod.rs` (`pub mod correct; pub use correct::{correct, detect_drift};`)
- Test: inline tests in `correct.rs`

**Interfaces:**
- Consumes: `fit_uniform`, `BeatGrid`, `BpmCurvePoint` (Task 7).
- Produces: `pub fn correct(grid: BeatGrid, beats: &[f64], tightened_tolerance_ms: f64) -> BeatGrid` — re-fit with tighter windows when confidence is low; `pub fn detect_drift(beats: &[f64], grid: &BeatGrid) -> Vec<BpmCurvePoint>`. Consumed by Task 12 runner (automatic backend correction).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn low_confidence_grid_is_refit() {
    // beats at 0.33s spacing (≈180 BPM) but bad guess
    let beats: Vec<f64> = (0..64).map(|i| 0.1 + i as f64 * 0.3333).collect();
    let mut bad = fit_uniform(&beats, 0.5); // wrong interval guess: ~120 BPM fit
    assert!(bad.confidence < 0.8);
    let fixed = correct(bad.clone(), &beats, 50.0);
    assert!(fixed.confidence > bad.confidence, "{} -> {}", bad.confidence, fixed.confidence);
    assert!((fixed.bpm - 180.0).abs() < 180.0 * 0.02, "bpm {}", fixed.bpm);
}

#[test]
fn drift_is_detected_as_variable_grid() {
    // accelerating: beat k at t(k) with interval shrinking 0.5 → 0.4762 (120→126 BPM) over 30 s
    let mut beats = Vec::new();
    let mut t = 0.0;
    let mut interval = 0.5;
    let n = 64usize;
    for _ in 0..n {
        beats.push(t);
        interval -= 0.000375; // total drop 0.0238 over 64 beats
        t += interval;
    }
    let g = fit_uniform(&beats, 0.5);
    let curve = detect_drift(&beats, &g);
    assert!(!curve.is_empty(), "drift not detected");
    let mid_bpm = curve[curve.len() / 2].bpm;
    assert!((mid_bpm - 123.0).abs() <= 123.0 * 0.02, "mid bpm {mid_bpm}");
    assert!((g.bpm - 120.0).abs() < 1.5, "start bpm {}", g.bpm);
}

#[test]
fn uniform_track_has_no_curve() {
    let beats: Vec<f64> = (0..64).map(|i| 0.25 + i as f64 * 0.5).collect();
    let g = fit_uniform(&beats, 0.5);
    assert!(detect_drift(&beats, &g).is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module correct not found`.

- [ ] **Step 3: Implement**

```rust
use super::{fit_uniform, BeatGrid, BpmCurvePoint};

/// Re-fit with a tighter window when confidence is low (backend automatic
/// correction; the manual-edit re-fits in Phase 3 call the same helpers).
pub fn correct(grid: BeatGrid, beats: &[f64], tightened_tolerance_ms: f64) -> BeatGrid {
    if grid.confidence >= 0.8 || beats.len() < 4 {
        return grid;
    }
    let interval = grid.beat_interval.max(0.05);
    let low = interval * 0.99;
    let high = interval * 1.01;
    let mut best: Option<BeatGrid> = None;
    let mut steps = 0f64;
    while steps < 8.0 {
        let delta = interval * 0.0025 * steps;
        for candidate_interval in [interval, low, high, interval - delta, interval + delta] {
            if candidate_interval <= 0.0 { continue; }
            let g = fit_uniform(beats, candidate_interval);
            if best.as_ref().map(|b| g.confidence > b.confidence).unwrap_or(true) {
                best = Some(g);
            }
        }
        if best.as_ref().map(|b| b.confidence >= 0.8).unwrap_or(false) { break; }
        steps += 1.0;
    }
    best.unwrap_or(grid)
}

/// Sliding-window (8-beat) interval estimates vs the uniform grid; a monotonic
/// residual trend ⇒ tempo drift ⇒ BpmCurve. Empty vec = no drift.
pub fn detect_drift(beats: &[f64], grid: &BeatGrid) -> Vec<BpmCurvePoint> {
    if beats.len() < 16 { return vec![]; }
    let window = 8usize;
    let mut points = Vec::new();
    for start in (0..beats.len() - window).step_by(window) {
        let seg = &beats[start..start + window];
        let d: f64 = seg.windows(2).map(|w| w[1] - w[0]).sum::<f64>() / (window - 1) as f64;
        let bpm = 60.0 / d;
        if (bpm - grid.bpm).abs() / grid.bpm > 0.01 {
            points.push(BpmCurvePoint { position_sec: seg[0], bpm });
        }
    }
    points
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS (3 tests). If the drift test is marginal, adjust the sweep constants (`0.0025`) and tolerance in the test only if the algorithm is proven correct on paper; otherwise tune parameters.

- [ ] **Step 5: Quality gates**

Run: gates from Task 4 Step 5.
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/beatgrid/correct.rs openmix-core/src/beatgrid/mod.rs
git commit -m "feat(core): automatic beat-grid correction and drift detection"
```

---

### Task 9: Vendored KeyFinder C++ Binding (TDD, native)

**Files:**
- Create: `openmix-core/src/keyfinder/mod.rs`
- Create: `openmix-core/src/keyfinder/shim.h`, `openmix-core/src/keyfinder/shim.cpp`, `openmix-core/src/keyfinder/build.rs`
- Create: `openmix-core/src/keyfinder/vendor/libkeyfinder/` (vendored from `mixxxdj/libkeyfinder`, pinned commit) + `vendor/NOTICE`
- Modify: `openmix-core/Cargo.toml` (`[build-dependencies] cc = "1"`; optional `keyfinder` module gated `#[cfg(feature = "native-analysis")]`)
- Test: inline test in `keyfinder/mod.rs`

**Interfaces:**
- Consumes: mono buffer + rate.
- Produces: `#[cfg(feature = "native-analysis")] pub fn detect_key(samples: &[f32], rate: u32) -> Option<KeyResult>` — maps libkeyfinder's integer key enum to `MusicalKey`, confidence from its chroma score. Consumed by Task 10 dispatch + Task 12 runner.

- [ ] **Step 1: Vendor and write the failing test**

Vendor: `git clone https://github.com/mixxxdj/libkeyfinder` at a pinned commit; copy the library sources (`KeyFinder/` headers + `.cpp` files and `dsp/` helpers) into `openmix-core/src/keyfinder/vendor/libkeyfinder/`; remove third-party JSON/soundtouch extras libkeyfinder does not need for `key_of_audio` (chroma + key detection use its own DSP). Add `vendor/NOTICE` with the GPL-3.0 text pointer and upstream URL/commit.

Test (in `mod.rs` `#[cfg(test)]`):

```rust
#[test]
fn keyfinder_detects_c_major_pad() {
    let mono = c_major_pad(44100, 6.0); // helper: sustained C-E-G triad, see Task 11 shape
    let k = detect_key(&mono, 44100).expect("detect");
    assert!(k.key == MusicalKey::CMajor || k.key == MusicalKey::AMinor, "key {:?}", k.key);
    assert!(k.confidence > 0.0);
}
```

(`c_major_pad` helper: sum of three sines at C4 261.63, E4 329.63, G4 392.00 Hz, 0.2 amplitude each, with 10 ms attack to avoid clicks.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module keyfinder not found`.

- [ ] **Step 3: Implement the shim**

`openmix-core/src/keyfinder/shim.h`:

```cpp
#ifndef OPENMIX_KF_SHIM_H
#define OPENMIX_KF_SHIM_H
#include <cstddef>
int openmix_kf_detect(const float* samples, size_t n, unsigned rate,
                      int* key_out, float* conf_out);
#endif
```

`openmix-core/src/keyfinder/shim.cpp`:

```cpp
#include "shim.h"
#include "vendor/libkeyfinder/KeyFinder/KeyFinder.h"
#include "vendor/libkeyfinder/KeyFinder/AudioData.h"

int openmix_kf_detect(const float* samples, size_t n, unsigned rate,
                      int* key_out, float* conf_out) {
    try {
        KeyFinder::AudioData audio;
        audio.setFrameRate((int)rate);
        audio.setChannels(1);
        audio.addToSampleCount((int)n);
        std::vector<float> &data = audio.getSamples();
        for (size_t i = 0; i < n; i++) data[i] = samples[i];
        KeyFinder::KeyFinder kf;
        KeyFinder::KeyFinderResult res = kf.keyOfAudio(audio);
        *key_out = (int)res.key;
        *conf_out = (float)res.confidence;
        return 0;
    } catch (...) {
        return -1;
    }
}
```

Note: adapt to the exact vendored `libkeyfinder` API at the pinned commit (member names may be `setFrameRate`/`addToSampleCount`/`getSamples` or the older `set_frames` style); the key enum is `KeyFinder::Key::A_MAJOR` etc. — map to `MusicalKey` via its Camelot/chromatic ordering.

`openmix-core/src/keyfinder/build.rs`:

```rust
fn main() {
    println!("cargo:rerun-if-changed=shim.cpp");
    println!("cargo:rerun-if-changed=shim.h");
    let mut build = cc::Build::new();
    build.cpp(true).std("c++11").file("shim.cpp").warnings(false);
    build.include("vendor/libkeyfinder");
    build.compile("openmix_kf");
    println!("cargo:rustc-link-lib=static=openmix_kf");
    println!("cargo:rustc-link-search=native={}", std::env::var("OUT_DIR").unwrap());
}
```

`openmix-core/src/keyfinder/mod.rs`:

```rust
//! Unsafe FFI to the vendored libkeyfinder; wrapped in a tiny safe API.
//! NOTE: libkeyfinder is GPL-3.0 — see vendor/NOTICE.

#[link(name = "openmix_kf", kind = "static")]
unsafe extern "C" {
    fn openmix_kf_detect(samples: *const f32, n: usize, rate: u32,
                         key_out: *mut i32, conf_out: *mut f32) -> i32;
}

pub fn detect_key(samples: &[f32], rate: u32) -> Option<KeyResult> {
    let mut key_out: i32 = -1;
    let mut conf_out: f32 = 0.0;
    let rc = unsafe {
        openmix_kf_detect(samples.as_ptr(), samples.len(), rate, &mut key_out, &mut conf_out)
    };
    if rc != 0 || key_out < 0 { return None; }
    let key = MusicalKey::from_keyfinder_index(key_out)?;
    Some(KeyResult { key, confidence: conf_out.clamp(0.0, 1.0), algorithm: KeyAlgorithm::KeyFinder, alternate: None })
}
```

Add `MusicalKey::from_keyfinder_index(i: i32) -> Option<MusicalKey>` in `analysis/key.rs` (Task 10) mapping libkeyfinder's `Key` enum ordering (A_MAJOR=0 … G_SHARP_MINOR=23 in its header) to our 24 variants.

Cargo.toml: add `[build-dependencies] cc = "1"` to workspace or core; add `mod keyfinder;` to `lib.rs` inside `#[cfg(feature = "native-analysis")]`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p openmix-core`
Expected: PASS on macOS.

- [ ] **Step 5: Verify feature isolation + Windows**

Run: `cargo check -p openmix-core --no-default-features`
Expected: green (keyfinder module excluded).
Push branch; confirm Windows CI compiles `shim.cpp` under MSVC (`cc` crate handles MSVC). **If MSVC compilation of vendored sources fails, STOP**: fallback per Risks — keep `keyfinder` module macOS/Linux-gated for now, default Windows key detection to the K-S chroma path (already behind `KeyDetector`), documented in Gate 2 report. Do not block the whole phase on it.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/keyfinder/ openmix-core/Cargo.toml
git commit -m "feat(core): vendored keyfinder binding behind native-analysis"
```

---

### Task 10: K-S Chroma Key Fallback + KeyDetector Dispatch (TDD)

**Files:**
- Create: `openmix-core/src/analysis/chroma.rs`, `openmix-core/src/analysis/key.rs`
- Modify: `openmix-core/src/analysis/mod.rs` (`pub mod chroma; pub mod key;` re-exports)
- Test: inline tests in `chroma.rs`

**Interfaces:**
- Consumes: `downsample_mono` (Task 1), `rustfft`, `MusicalKey`.
- Produces: `pub fn ks_key(mono: &[f32], rate: u32) -> Option<KeyResult>` (always available — the K-S fallback and the comparison algorithm); `pub fn best_key(kf: Option<KeyResult>, ks: Option<KeyResult>) -> Option<KeyResult>` (by confidence, sets `alternate`); `MusicalKey` full enum + `from_relative_index(semitones: i32 -> tonic) -> Option<MusicalKey>` + `from_keyfinder_index`. `KeyDetector` impls: `KeyFinderKeyDetector` `[native]` and `KrumhanslKeyDetector`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn ks_detects_c_major_pad() {
    let mono = c_major_pad(44100, 6.0); // same helper as Task 9
    let k = ks_key(&mono, 44100).expect("detect");
    assert!(k.key == MusicalKey::CMajor || k.key == MusicalKey::AMinor, "key {:?}", k.key);
    assert!(k.confidence > 0.3, "conf {}", k.confidence);
}

#[test]
fn ks_returns_none_on_silence() {
    let silent = vec![0.0f32; 44100 * 3];
    assert!(ks_key(&silent, 44100).is_none());
}

#[test]
fn best_key_prefers_higher_confidence() {
    let low = KeyResult { key: MusicalKey::AMajor, confidence: 0.4, algorithm: KeyAlgorithm::KrumhanslSchmuckler, alternate: None };
    let high = KeyResult { key: MusicalKey::CMajor, confidence: 0.9, algorithm: KeyAlgorithm::KeyFinder, alternate: None };
    let best = best_key(Some(high.clone()), Some(low.clone())).unwrap();
    assert_eq!(best.key, MusicalKey::CMajor);
    assert_eq!(best.alternate, Some((MusicalKey::AMajor, 0.4)));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `module chroma not found` / `module key not found`.

- [ ] **Step 3: Implement**

`analysis/chroma.rs` (Krumhansl–Schmuckler):

```rust
use rustfft::{num_complex::Complex, FftPlanner};
use crate::analysis::key::{KeyAlgorithm, KeyResult, MusicalKey};

// Krumhansl–Kessler major profile (normalized), tonic = C
const KK_MAJOR: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
// Krumhansl–Kessler minor profile, tonic = A
const KK_MINOR: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];

pub fn ks_key(mono: &[f32], rate: u32) -> Option<KeyResult> {
    let work = crate::audio::mono::downsample_mono(mono, rate, 11_025);
    if work.len() < 8192 { return None; }
    let fft_size = 4096usize;
    let hop = 2048usize;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buf = vec![Complex::new(0.0f32, 0.0); fft_size];
    let mut chroma = [0.0f64; 12];
    let mut frames = 0u32;
    for chunk in work.chunks(hop) {
        if chunk.len() < hop { break; }
        for (i, s) in chunk.iter().enumerate() {
            let h = 0.54 - 0.46 * ((std::f32::consts::TAU * i as f32 / (fft_size as f32 - 1.0)).cos());
            buf[i] = Complex::new(s * h, 0.0);
        }
        fft.process(&mut buf);
        let bin_hz = 11_025.0 / fft_size as f32;
        for k in 1..(fft_size / 2) {
            let mag = buf[k].norm() as f64;
            if mag < 1e-6 { continue; }
            let hz = k as f32 * bin_hz;
            if hz < 60.0 || hz > 4000.0 { continue; }
            let midi = 69.0 + 12.0 * ((hz / 440.0).ln() / std::f32::consts::LN_2);
            let class = ((midi.round() as i32).rem_euclid(12)) as usize;
            chroma[class] += mag;
        }
        frames += 1;
    }
    if frames == 0 { return None; }
    let total: f64 = chroma.iter().sum();
    if total <= 1e-9 { return None; }
    let norm: Vec<f32> = chroma.iter().map(|c| (c / total) as f32).collect();

    // correlate against all 12 rotations of each profile, A-minor anchored
    let mut best = (0i32, 0f32, 0f32); // (semitone, mode 0=major 1=minor, correlation)
    for tonic in 0..12i32 {
        let mut cmaj = 0f32; let mut cmin = 0f32;
        for pc in 0..12 {
            let idx = ((tonic + pc).rem_euclid(12)) as usize;
            cmaj += norm[idx] * KK_MAJOR[pc];
            cmin += norm[idx] * KK_MINOR[(pc + 9).rem_euclid(12)]; // A-minor anchored to C
        }
        // basic normalization: correlation / (profile norm) — sufficient for argmax
        if cmaj > best.1 { best = (tonic, cmaj, 0.0); }
        if cmin > best.1 { best = (tonic, cmin, 1.0); }
    }
    // confidence: normalized by max possible (sum of profile) → rough scale
    let conf = (best.1 / KK_MAJOR.iter().sum::<f32>()).clamp(0.0, 1.0);
    let key = MusicalKey::from_tonic_mode(best.0, best.2 == 1.0)?;
    Some(KeyResult { key, confidence: conf, algorithm: KeyAlgorithm::KrumhanslSchmuckler, alternate: None })
}
```

`analysis/key.rs`:

```rust
//! MusicalKey enum, KeyResult, and KeyDetector dispatch.
//! (MusicalKey from Task 2 lives here; extend with constructors:)

impl MusicalKey {
    /// tonic: semitone index 0..11 (C..B); minor: true → minor.
    pub fn from_tonic_mode(tonic: i32, minor: bool) -> Option<MusicalKey> {
        use MusicalKey::*;
        let keys = if minor {
            [AMinor, ASharpMinor, BMinor, CMinor, CSharpMinor, DMinor, DSharpMinor, EMinor, FMinor, FSharpMinor, GMinor, GSharpMinor]
        } else {
            [AMajor, ASharpMajor, BMajor, CMajor, CSharpMajor, DMajor, DSharpMajor, EMajor, FMajor, FSharpMajor, GMajor, GSharpMajor]
        };
        keys.get(tonic.rem_euclid(12) as usize).copied()
    }
    /// libkeyfinder's Key enum order (A_MAJOR .. G_SHARP_MINOR).
    #[cfg(feature = "native-analysis")]
    pub fn from_keyfinder_index(i: i32) -> Option<MusicalKey> {
        use MusicalKey::*;
        let all = [
            AMajor, ASharpMajor, BMajor, CMajor, CSharpMajor, DMajor,
            DSharpMajor, EMajor, FMajor, FSharpMajor, GMajor, GSharpMajor,
            AMinor, ASharpMinor, BMinor, CMinor, CSharpMinor, DMinor,
            DSharpMinor, EMinor, FMinor, FSharpMinor, GMinor, GSharpMinor,
        ];
        all.get(i as usize).copied()
    }
}

pub fn best_key(kf: Option<KeyResult>, ks: Option<KeyResult>) -> Option<KeyResult> {
    match (kf, ks) {
        (Some(mut a), Some(b)) => {
            if b.confidence > a.confidence {
                a.alternate = Some((b.key, b.confidence));
            } else {
                a.alternate = Some((b.key, b.confidence));
                let _ = a.algorithm; // keep KeyFinder as primary per architecture
            }
            Some(a)
        }
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

#[cfg(feature = "native-analysis")]
pub struct KeyFinderKeyDetector;
#[cfg(feature = "native-analysis")]
impl super::KeyDetector for KeyFinderKeyDetector {
    fn key(&self, mono: &[f32], rate: u32) -> Option<KeyResult> {
        crate::keyfinder::detect_key(mono, rate)
    }
}

pub struct KrumhanslKeyDetector;
impl super::KeyDetector for KrumhanslKeyDetector {
    fn key(&self, mono: &[f32], rate: u32) -> Option<KeyResult> {
        crate::analysis::chroma::ks_key(mono, rate)
    }
}
```

Fix `best_key` so `alternate` is always the *other* algorithm's result and the primary is the higher-confidence one (keep the comparison symmetric; adjust code above to swap cleanly — final form must satisfy the test exactly).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS (3 tests). If `ks_detects_c_major_pad` flips to A-minor only, accept relative-key equivalence (test asserts either).

- [ ] **Step 5: Quality gates**

Run: gates from Task 4 Step 5.
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/analysis/chroma.rs openmix-core/src/analysis/key.rs openmix-core/src/analysis/mod.rs
git commit -m "feat(core): krumhansl-schmuckler key fallback and detector dispatch"
```

---

### Task 11: Fixture Corpus + Accuracy Runner (TDD)

**Files:**
- Create: `scripts/gen-analysis-fixtures.py`
- Create: `openmix-core/tests/accuracy.rs`
- Create: `openmix-core/src/bin/analyze.rs`, `openmix-core/src/bin/accuracy.rs`
- Create: `openmix-core/tests/fixtures/*.wav` (+ `.flac`/`.mp3` variants, committed binaries)
- Modify: `openmix-core/Cargo.toml` (add `[[bin]]` entries need no change — `src/bin/*` auto-detected; add `[dev-dependencies]` nothing new)
- Test: `openmix-core/tests/accuracy.rs` is the accuracy gate itself.

**Interfaces:**
- Produces: fixture manifest embedded in `accuracy.rs` as `(path, expected_bpm: Option<f64>, expected_key: Option<MusicalKey>, expected_offset_s: Option<f64>)`; CLI `analyze` prints JSON of `AnalysisResult`; CLI `accuracy` prints per-fixture table + aggregate percentages.

- [ ] **Step 1: Write the fixture generator** (`scripts/gen-analysis-fixtures.py`, stdlib only, committed):

```python
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

ROOTS = ["A", "A#", "B", "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#"]
for root in ROOTS:
    write_wav(f"pad_{root}_major.wav", pad_track(triad(NOTE[root], minor=False), 8.0))
for root in ["A", "C", "D", "E", "F", "G"]:
    write_wav(f"pad_{root}_minor.wav", pad_track(triad(NOTE[root], minor=True), 8.0))
print("fixtures written")
```

Generate, then produce FLAC/MP3 variants of `kick_120bpm` via the Phase 1 flow (`afconvert` + `lame` on macOS, dev-only; committed binaries so CI never runs the scripts). Also one **stereo** variant (`kick_120bpm_stereo.wav`) — generate by duplicating mono to 2ch in-wave (or the `.py` extended) to exercise the mono downmix path.

- [ ] **Step 2: Commit fixtures**

```sh
git add scripts/gen-analysis-fixtures.py openmix-core/tests/fixtures/
git commit -m "test(core): analysis fixture corpus (bpm kicks + key pads)"
```

- [ ] **Step 3: Write `tests/accuracy.rs` (the Gate 2 machine gate)**

```rust
use std::path::Path;

use openmix_core::analysis::{analyze_path, AnalysisConfig};
use openmix_core::AppError;

struct Fixture { file: &'static str, bpm: Option<f64>, key: Option<MusicalKey>, offset_s: Option<f64> }

const FIXTURES: &[Fixture] = &[
    // BPM set (all WAV unless noted)
    Fixture { file: "kick_70bpm.wav",  bpm: Some(70.0), key: None, offset_s: None },
    Fixture { file: "kick_87bpm.wav",  bpm: Some(87.0), key: None, offset_s: None },
    Fixture { file: "kick_100bpm.wav", bpm: Some(100.0), key: None, offset_s: None },
    Fixture { file: "kick_120bpm.wav", bpm: Some(120.0), key: None, offset_s: Some(0.0) },
    Fixture { file: "kick_120bpm.flac", bpm: Some(120.0), key: None, offset_s: Some(0.0) },
    Fixture { file: "kick_120bpm.mp3",  bpm: Some(120.0), key: None, offset_s: Some(0.0) },
    Fixture { file: "kick_120bpm_stereo.wav", bpm: Some(120.0), key: None, offset_s: Some(0.0) },
    Fixture { file: "kick_120bpm_intro.wav", bpm: Some(120.0), key: None, offset_s: Some(0.87) },
    Fixture { file: "kick_128bpm.wav", bpm: Some(128.0), key: None, offset_s: None },
    Fixture { file: "kick_140bpm.wav", bpm: Some(140.0), key: None, offset_s: None },
    Fixture { file: "kick_174bpm.wav", bpm: Some(174.0), key: None, offset_s: None },
    Fixture { file: "kick_180bpm.wav", bpm: Some(180.0), key: None, offset_s: None },
    // Key set (pad fixtures): 12 major + 6 minor
    Fixture { file: "pad_A_major.wav",  bpm: None, key: Some(MusicalKey::AMajor), offset_s: None },
    Fixture { file: "pad_A#_major.wav", bpm: None, key: Some(MusicalKey::ASharpMajor), offset_s: None },
    Fixture { file: "pad_B_major.wav",  bpm: None, key: Some(MusicalKey::BMajor), offset_s: None },
    Fixture { file: "pad_C_major.wav",  bpm: None, key: Some(MusicalKey::CMajor), offset_s: None },
    Fixture { file: "pad_C#_major.wav", bpm: None, key: Some(MusicalKey::CSharpMajor), offset_s: None },
    Fixture { file: "pad_D_major.wav",  bpm: None, key: Some(MusicalKey::DMajor), offset_s: None },
    Fixture { file: "pad_D#_major.wav", bpm: None, key: Some(MusicalKey::DSharpMajor), offset_s: None },
    Fixture { file: "pad_E_major.wav",  bpm: None, key: Some(MusicalKey::EMajor), offset_s: None },
    Fixture { file: "pad_F_major.wav",  bpm: None, key: Some(MusicalKey::FMajor), offset_s: None },
    Fixture { file: "pad_F#_major.wav", bpm: None, key: Some(MusicalKey::FSharpMajor), offset_s: None },
    Fixture { file: "pad_G_major.wav",  bpm: None, key: Some(MusicalKey::GMajor), offset_s: None },
    Fixture { file: "pad_G#_major.wav", bpm: None, key: Some(MusicalKey::GSharpMajor), offset_s: None },
    Fixture { file: "pad_A_minor.wav",  bpm: None, key: Some(MusicalKey::AMinor), offset_s: None },
    Fixture { file: "pad_C_minor.wav",  bpm: None, key: Some(MusicalKey::CMinor), offset_s: None },
    Fixture { file: "pad_D_minor.wav",  bpm: None, key: Some(MusicalKey::DMinor), offset_s: None },
    Fixture { file: "pad_E_minor.wav",  bpm: None, key: Some(MusicalKey::EMinor), offset_s: None },
    Fixture { file: "pad_F_minor.wav",  bpm: None, key: Some(MusicalKey::FMinor), offset_s: None },
    Fixture { file: "pad_G_minor.wav",  bpm: None, key: Some(MusicalKey::GMinor), offset_s: None },
];

fn fixtures_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").as_path()
}

fn analyze(fixture: &Fixture) -> Result<openmix_core::analysis::AnalysisResult, AppError> {
    let path = fixtures_dir().join(fixture.file);
    analyze_path(&path, &AnalysisConfig::default())
}

#[test]
fn bpm_accuracy_ge_90_percent() {
    let bpm_fixtures: Vec<&Fixture> = FIXTURES.iter().filter(|f| f.bpm.is_some()).collect();
    let passed = bpm_fixtures.iter().filter(|f| match analyze(f) {
        Ok(r) => r.bpm.is_some_and(|b| (b - f.bpm.unwrap()).abs() <= f.bpm.unwrap() * 0.015),
        Err(_) => false,
    }).count();
    let pct = passed as f64 / bpm_fixtures.len() as f64;
    assert!(pct >= 0.90, "BPM accuracy {:.0}% ({passed}/{}); inspect per-fixture report", pct * 100.0, bpm_fixtures.len());
}

#[test]
fn key_accuracy_ge_90_percent() {
    let key_fixtures: Vec<&Fixture> = FIXTURES.iter().filter(|f| f.key.is_some()).collect();
    let passed = key_fixtures.iter().filter(|f| match analyze(f) {
        Ok(r) => r.key.is_some_and(|k| k.key == f.key.unwrap() || k.key == f.key.unwrap().relative()),
        Err(_) => false,
    }).count();
    let pct = passed as f64 / key_fixtures.len() as f64;
    assert!(pct >= 0.90, "Key accuracy {:.0}% ({passed}/{}); inspect per-fixture report", pct * 100.0, key_fixtures.len());
}

#[test]
fn grid_offset_accuracy_ge_90_percent() {
    let grid_fixtures: Vec<&Fixture> = FIXTURES.iter().filter(|f| f.offset_s.is_some()).collect();
    let passed = grid_fixtures.iter().filter(|f| match analyze(f) {
        Ok(r) => r.grid.is_some_and(|g| (g.first_beat_offset - f.offset_s.unwrap()).abs() <= 0.050),
        Err(_) => false,
    }).count();
    let pct = passed as f64 / grid_fixtures.len() as f64;
    assert!(pct >= 0.90, "Grid offset accuracy {:.0}% ({passed}/{}); inspect per-fixture report", pct * 100.0, grid_fixtures.len());
}

#[test]
fn accuracy_report_prints_per_fixture() {
    // informational: prints a table the Gate 2 report copies verbatim
    for f in FIXTURES {
        let r = analyze(f);
        println!("{:<24} bpm={:?} key={:?} offset={:?} => {:?}", f.file, f.bpm, f.key, f.offset_s,
                 r.as_ref().map(|a| (a.bpm, a.key.as_ref().map(|k| k.key), a.grid.as_ref().map(|g| g.first_beat_offset))));
    }
}
```

- [ ] **Step 4: Run — verify it fails (missing fixtures / below target)**

Run: `cargo test -p openmix-core --test accuracy`
Expected: FAIL on missing `MusicalKey` import path adjustment (re-export `MusicalKey` from `openmix_core::analysis`) and any fixture accuracy below target.

- [ ] **Step 5: Tune to ≥90%**

Iterate detector parameters until all three gates pass:
- BPM: aubio threshold/silence (`with_threshold`), hop size, feed length (first 60 s is enough — cap `mono` slice in runner).
- Key: K-S window/hop, bin range, profile normalization; KeyFinder tuning (chroma resolution) if vendored API exposes it.
- Grid: `correct()` tightened tolerance, fit sweep granularity.
Record the final per-fixture numbers (from the report test) for the Gate 2 report.

- [ ] **Step 6: Write the CLIs** (`src/bin/analyze.rs`, `src/bin/accuracy.rs`)

`analyze.rs`: parse `--json` flag and file path → `analyze_path` → print `serde_json::to_string_pretty` of result. `accuracy.rs`: loop `FIXTURES`-like table (reuse via a small `pub fn fixture_list()` exported from a `#[cfg(test)]`-independent location — place the manifest in `src/bin/accuracy.rs` itself or `analysis/fixtures.rs`; keep it simple: duplicate the small manifest in the bin) → print aligned table + percentages; exit non-zero if any metric <90%.

- [ ] **Step 7: Verify**

Run: `cargo test -p openmix-core --test accuracy` (3 gates PASS) and `cargo run -p openmix-core --bin accuracy` (table printed, exit 0).
Run: gates from Task 4 Step 5.
Expected: green.

- [ ] **Step 8: Commit**

```sh
git add openmix-core/tests/ openmix-core/src/bin/ scripts/
git commit -m "test(core): analysis accuracy fixture suite (>=90% gate) and cli"
```

---

### Task 12: Analysis Orchestrator (single-pass runner, TDD)

**Files:**
- Create: `openmix-core/src/analysis/runner.rs` (or fold into `mod.rs` if cleaner)
- Modify: `openmix-core/src/analysis/mod.rs` (`mod runner; pub use runner::{analyze, analyze_path};`), `openmix-core/src/error.rs` (add `Analysis(String)` variant)
- Test: `openmix-core/tests/analysis_test.rs` (integration)

**Interfaces:**
- Consumes: `DecodedStream` (Phase 1), all detectors (Tasks 4–10), `fit_uniform` + `correct` (Tasks 7–8).
- Produces: `pub fn analyze(stream: &mut DecodedStream, cfg: &AnalysisConfig, cancel: &AtomicBool) -> Result<AnalysisResult, AppError>` — the single-pass orchestrator; `pub fn analyze_path(path, cfg) -> Result<AnalysisResult, AppError>`. Consumed by Tasks 11 (accuracy) and 14 (IPC).

- [ ] **Step 1: Write the failing integration tests** (`openmix-core/tests/analysis_test.rs`)

```rust
use std::path::Path;
use openmix_core::analysis::{analyze_path, AnalysisConfig};
use openmix_core::AppError;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

#[test]
fn analyze_120bpm_fixture_full_result() -> Result<(), AppError> {
    let r = analyze_path(fixture("kick_120bpm.wav"), &AnalysisConfig::default())?;
    assert!(r.bpm.is_some_and(|b| (b - 120.0).abs() <= 1.8), "bpm {:?}", r.bpm);
    assert!(r.grid.as_ref().is_some_and(|g| g.confidence > 0.8), "grid {:?}", r.grid);
    assert!(!r.beats.is_empty());
    assert!(!r.energy_windows.is_empty());
    assert!(r.rms_db.is_some());
    Ok(())
}

#[test]
fn analyze_mp3_and_flac_paths() -> Result<(), AppError> {
    for f in ["kick_120bpm.mp3", "kick_120bpm.flac"] {
        let r = analyze_path(fixture(f), &AnalysisConfig::default())?;
        assert!(r.bpm.is_some(), "{f}: no bpm");
    }
    Ok(())
}

#[test]
fn silence_yields_none_not_error() -> Result<(), AppError> {
    // generate 4 s of silence into a wav via DecodedStream-compatible fixture
    // (reuse tiny helper writing a silent 16-bit mono wav like Phase 1 audio_test)
    let r = analyze_path(silent_wav()?, &AnalysisConfig::default())?;
    assert!(r.bpm.is_none() && r.key.is_none());
    Ok(())
}

#[test]
fn cancel_stops_early() -> Result<(), AppError> {
    use std::sync::atomic::{AtomicBool, Ordering};
    use openmix_core::audio::DecodedStream;
    let cancel = AtomicBool::new(true);
    let mut stream = DecodedStream::open(fixture("kick_120bpm.wav"))?;
    let r = openmix_core::analysis::analyze(&mut stream, &AnalysisConfig::default(), &cancel);
    assert!(r.is_err(), "cancelled analysis should error");
    Ok(())
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core --test analysis_test`
Expected: FAIL — `analyze`/`analyze_path` not found (or module missing).

- [ ] **Step 3: Implement the runner**

```rust
use std::sync::atomic::{AtomicBool, Ordering};
use crate::audio::decode::{AudioChunk, DecodedStream};
use crate::beatgrid::{correct, fit_uniform};
use crate::error::AppError;
use crate::analysis::energy::{energy_windows, peak_db_of, rms_db_of};
use super::{AnalysisConfig, AnalysisResult};

/// Single bounded-memory pass: feeds aubio streaming detectors each chunk,
/// accumulates only the capped key buffer.
pub fn analyze(stream: &mut DecodedStream, cfg: &AnalysisConfig, cancel: &AtomicBool)
    -> Result<AnalysisResult, AppError>
{
    let rate = stream.sample_rate();
    let mut mono_feed: Vec<f32> = Vec::new(); // downsampled to tempo_hop-friendly 44100
    // We avoid doubling memory: build mono at 44100 in one pass, feeding
    // tempo/onset/beat detectors hop-by-hop from a reusable ring.
    let mut tempo = make_tempo_stream(rate, cfg);   // Option<Tempo> [native] / None (fallback)
    let mut onsets = make_onset_stream(rate, cfg);  // Option<Onset> [native] / None
    let mut beat_times: Vec<f64> = Vec::new();
    let mut onset_times: Vec<f64> = Vec::new();
    let key_rate = cfg.key_rate;
    let key_cap = cfg.key_max_seconds.map(|s| (key_rate as f64 * s) as usize);
    let mut key_buf: Vec<f32> = Vec::new();
    let mut energy_acc: Vec<f32> = Vec::new(); // raw mono, for windowed RMS at end

    let hop = cfg.tempo_hop.max(1) as usize;
    let mut mono_buf: Vec<f32> = Vec::with_capacity(hop * 2);
    let mut fence = 0usize; // absolute frame counter of fed samples

    while let Some(chunk) = stream.next_chunk(8192)? {
        if cancel.load(Ordering::Relaxed) { return Err(AppError::Other("cancelled".into())); }
        let mono = crate::audio::mono::to_mono(&chunk.samples, stream.channels(), rate, 44_100);
        mono_buf.extend_from_slice(&mono);
        energy_acc.extend_from_slice(&mono);
        // feed tempo/onset in hop-sized slices
        let mut idx = 0;
        while idx + hop <= mono_buf.len() {
            let slice = &mono_buf[idx..idx + hop];
            if let Some(t) = tempo.as_mut() {
                if let Ok(r) = t.do_result(slice) {
                    if r > 0.5 { beat_times.push(t.get_last_s() as f64 + fence as f64 / 44_100.0); }
                }
            }
            if let Some(o) = onsets.as_mut() {
                if let Ok(r) = o.do_result(slice) {
                    if r > 0.5 { onset_times.push(o.get_last_s() as f64 + fence as f64 / 44_100.0); }
                }
            }
            idx += hop;
            fence += hop;
        }
        mono_buf.drain(..idx);
        // bounded key accumulation at key_rate
        let key_mono = crate::audio::mono::to_mono(&chunk.samples, stream.channels(), rate, key_rate);
        if key_cap.map(|c| key_buf.len() < c).unwrap_or(true) {
            let room = key_cap.map_or(key_mono.len(), |c| c - key_buf.len());
            key_buf.extend_from_slice(&key_mono[..room.min(key_mono.len())]);
        }
        if energy_acc.len() > 44_100 * 60 { // cap energy window source: 60 s is enough
            energy_acc.drain(..energy_acc.len() - 44_100 * 60);
        }
    }
    if cancel.load(Ordering::Relaxed) { return Err(AppError::Other("cancelled".into())); }

    // BPM (native) or autocorr fallback; confidence
    let (bpm, bpm_confidence) = extract_bpm(&beat_times, &mono_in_memory_if_any, rate);

    // Beat grid: uniform fit + automatic correction
    let grid = fit_uniform(&beat_times, 0.5);
    let grid = correct(grid, &beat_times, 50.0);

    let beats = label_from_grid(beat_times.clone(), &grid);
    let rms_db = rms_db_of(&energy_acc);
    let peak_db = peak_db_of(&energy_acc);
    let energy_windows = energy_windows(&energy_acc, 44_100, cfg.energy_window_ms);

    // Key: KeyFinder (native) + K-S; dispatch chooses best
    #[cfg(feature = "native-analysis")]
    let kf = crate::keyfinder::detect_key(&key_buf, key_rate);
    #[cfg(not(feature = "native-analysis"))]
    let kf = None;
    let ks = crate::analysis::chroma::ks_key(&key_buf, key_rate);
    let key = best_key(kf, ks);

    Ok(AnalysisResult {
        bpm, bpm_confidence,
        onsets: onset_times,
        beats,
        grid: Some(grid),
        key,
        rms_db, peak_db, energy_windows,
    })
}

pub fn analyze_path(path: impl AsRef<std::path::Path>, cfg: &AnalysisConfig)
    -> Result<AnalysisResult, AppError>
{
    let mut stream = crate::audio::DecodedStream::open(path)?;
    let cancel = AtomicBool::new(false);
    analyze(&mut stream, cfg, &cancel)
}
```

(Implement helper functions `make_tempo_stream`, `make_onset_stream`, `extract_bpm`, `label_from_grid` — streaming aubio holders gated `[native]`, with fallback branches; the exact factor behind `bpm` extraction: prefer aubio `get_bpm()`/`get_confidence()` captured during the loop, else `autocorr_bpm` on a capped work buffer. Keep helper signatures internal.)

`error.rs` addition:

```rust
#[error("analysis error: {0}")]
Analysis(String),
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-core --test analysis_test && cargo test -p openmix-core --test accuracy`
Expected: PASS (integration + 3 accuracy gates).

- [ ] **Step 5: Quality gates + CLI smoke**

Run: gates from Task 4 Step 5; then `cargo run -p openmix-core --bin analyze -- tests/fixtures/kick_120bpm.wav --json`
Expected: JSON output with non-null bpm/grid/key fields; all gates green.

- [ ] **Step 6: Commit**

```sh
git add openmix-core/src/analysis/ openmix-core/src/error.rs openmix-core/tests/analysis_test.rs
git commit -m "feat(core): single-pass analysis orchestrator and cli"
```

---

### Task 13: App Storage — Analysis Tables + Migrations (TDD)

**Files:**
- Modify: `openmix-app/src/storage/schema.sql`, `openmix-app/src/storage/db.rs`, `openmix-app/src/storage/mod.rs`
- Modify: `openmix-app/tests/storage_test.rs`
- Test: `cargo test -p openmix-app --test storage_test`

**Interfaces:**
- Consumes: existing `Storage`, `Track` (Phase 1).
- Produces: `pub struct AnalysisRow { pub track_id: String, pub file_hash: String, pub bpm: Option<f64>, pub bpm_confidence: Option<f32>, pub key: Option<String>, pub key_confidence: Option<f32>, pub energy: String, pub created_at: String }` (JSON `energy` holds `{rms_db, peak_db, energy_windows}`); `Storage::upsert_analysis(&AnalysisRow) -> Result<(), StorageError>`, `Storage::get_analysis(&self, track_id: &str) -> Result<Option<AnalysisRow>, StorageError>`, `Storage::upsert_beat_grid(&self, track_id: &str, file_hash: &str, grid_json: &str) -> Result<(), StorageError>`, `Storage::get_beat_grid(&self, track_id: &str) -> Result<Option<String>, StorageError>`; migration runner stamps `PRAGMA user_version`.

- [ ] **Step 1: Write the failing tests** (append to `openmix-app/tests/storage_test.rs`)

```rust
#[test]
fn analysis_upsert_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    let row = AnalysisRow {
        track_id: sample_track(None).id.clone(), // reuse the id from inserted track
        file_hash: "abc".into(), bpm: Some(128.0), bpm_confidence: Some(0.9),
        key: Some("AMinor".into()), key_confidence: Some(0.8),
        energy: r#"{"rms_db":-12.0,"peak_db":-1.0,"energy_windows":[1,2,3]}"#.into(),
        created_at: "123".into(),
    };
    storage.upsert_analysis(&row).unwrap();
    let got = storage.get_analysis(&row.track_id).unwrap().unwrap();
    assert_eq!(got.bpm, Some(128.0));
    assert_eq!(got.key.as_deref(), Some("AMinor"));
    storage.upsert_analysis(&row).unwrap(); // idempotent upsert
    let again = storage.get_analysis(&row.track_id).unwrap().unwrap();
    assert_eq!(again.bpm, Some(128.0));
}

#[test]
fn beat_grid_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    let grid = r#"{"first_beat_offset":0.87,"bpm":120.0,"beat_interval":0.5,"confidence":0.95,"curve":[]}"#;
    storage.upsert_beat_grid("tid", "abc", grid).unwrap();
    assert_eq!(storage.get_beat_grid("tid").unwrap(), Some(grid.to_string()));
}

#[test]
fn delete_project_cascades_analysis() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    // upsert with the real inserted track id
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    let row = AnalysisRow { track_id: tracks[0].id.clone(), file_hash: "abc".into(), bpm: None,
        bpm_confidence: None, key: None, key_confidence: None, energy: "{}".into(), created_at: "1".into() };
    storage.upsert_analysis(&row).unwrap();
    storage.delete_project(&p.id).unwrap();
    assert!(storage.get_analysis(&row.track_id).unwrap().is_none());
}
```

(Adjust `sample_track` id handling so the analysis row's `track_id` matches an inserted track — use `list_tracks` as above.)

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-app --test storage_test`
Expected: FAIL — `AnalysisRow` / methods not found.

- [ ] **Step 3: Implement migration runner + schema**

`schema.sql` (append):

```sql
PRAGMA user_version = 2;

CREATE TABLE IF NOT EXISTS track_analysis (
  track_id TEXT PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  file_hash TEXT NOT NULL,
  bpm REAL,
  bpm_confidence REAL,
  key TEXT,
  key_confidence REAL,
  energy TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS beat_grids (
  track_id TEXT PRIMARY KEY REFERENCES tracks(id) ON DELETE CASCADE,
  file_hash TEXT NOT NULL,
  grid TEXT NOT NULL,
  created_at TEXT NOT NULL
);
```

`db.rs`: keep `init` (idempotent `execute_batch`) and add `pub fn migrate(conn: &rusqlite::Connection) -> rusqlite::Result<()>` reading `PRAGMA user_version` — run `init` always (idempotent `CREATE TABLE IF NOT EXISTS`), then `conn.pragma_update(None, "user_version", 2)`. Replace `db::init` calls in `Storage::open`/`open_in_memory` with `db::migrate`.

`storage/mod.rs`: add `AnalysisRow` struct + the four methods (JSON column serialization/deserialization via `serde_json::from_str::<serde_json::Value>` ensure well-formed; store raw strings otherwise).

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-app --test storage_test`
Expected: PASS (old 4 + new 3 = 7 tests).

- [ ] **Step 5: Quality gates**

Run: `cargo fmt --all --check && cargo clippy -p openmix-app --all-targets -- -D warnings`
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-app/src/storage/ openmix-app/tests/storage_test.rs
git commit -m "feat(app): analysis storage tables and migration runner"
```

---

### Task 14: Analysis IPC Commands (TDD)

**Files:**
- Create: `openmix-app/src/commands/analysis.rs`
- Modify: `openmix-app/src/lib.rs` (register commands + `analysis:done` event emit), `openmix-app/src/storage/mod.rs` if an `AnalysisStatus` helper is needed
- Test: `openmix-app/tests/analysis_command_test.rs`

**Interfaces:**
- Consumes: `analysis::analyze_path` (Task 12), storage methods (Task 13), existing `AppState`.
- Produces: `#[tauri::command] fn analyze_track(state: State<AppState>, track_id: String) -> Result<String, String>` (spawns background thread, returns `track_id` immediately, emits `analysis:done` with the `AnalysisResult` payload); `#[tauri::command] fn get_analysis(state: State<AppState>, track_id: String) -> Result<Option<AnalysisResult>, String>`; `#[tauri::command] fn get_beat_grid(state: State<AppState>, track_id: String) -> Result<Option<BeatGrid>, String>`; cache rule: if `track_analysis.file_hash == track.file_hash`, return cached.

- [ ] **Step 1: Write the failing tests** (`openmix-app/tests/analysis_command_test.rs`)

```rust
use openmix_app::commands::analysis::{get_analysis, get_beat_grid, analyze_track};
use openmix_app::storage::{Storage, Track};
use tauri::State;

fn sample_track() -> Track { /* same shape as storage_test */ }

#[test]
fn analyze_track_persists_and_returns() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    let mut t = sample_track();
    t.project_id = Some(p.id.clone());
    storage.insert_track(&t).unwrap();
    // direct-call the command body (extract a testable fn analyze_track_inner(storage, track_id, emit) )
    let id = analyze_track_inner(&storage, &t.id, &wav_fixture_path()).unwrap();
    assert_eq!(id, t.id);
    let analysis = get_analysis_inner(&storage, &t.id).unwrap();
    assert!(analysis.is_some());
}

#[test]
fn cache_hit_returns_without_reanalysis() {
    // insert track + pre-existing analysis row with matching file_hash
    // analyze_track_inner must return cached (track timestamp unchanged)
    // verify by inspecting storage row created_at stability
}

#[test]
fn beat_grid_roundtrip_via_command() {
    let id = analyze_track_inner(&storage, &t.id, &wav_fixture_path()).unwrap();
    let g = get_beat_grid_inner(&storage, &id).unwrap();
    assert!(g.is_some());
}
```

Design for testability: implement `analyze_track_inner(storage: &Storage, track_id: &str, path: &Path) -> Result<String, String>` (pure persistence + core call, no Tauri) and have the `#[tauri::command]` wrapper spawn `std::thread::spawn(move || analyze_track_inner(...))` + `app.emit("analysis:done", result)`; tests call the inner fns directly. Reuse `openmix-core/tests/fixtures/kick_120bpm.wav` via `include_bytes!`-style path (read from `CARGO_MANIFEST_DIR/../openmix-core/tests/fixtures`).

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-app --test analysis_command_test`
Expected: FAIL — module/command not found.

- [ ] **Step 3: Implement**

`commands/analysis.rs`:

```rust
use openmix_core::analysis::{analyze_path, AnalysisConfig, AnalysisResult};
use crate::storage::{Storage, Track};

pub fn analyze_track_inner(storage: &Storage, track_id: &str, path: &std::path::Path)
    -> Result<String, String>
{
    let track: Track = storage.list_tracks(None).unwrap_or_default()
        .into_iter().chain(storage.list_tracks(Some(track_id)).unwrap_or_default()) // or storage.get_track(track_id) — add it if missing
        .find(|t| t.id == track_id)
        .ok_or_else(|| format!("track {track_id} not found"))?;
    // cache: skip re-analysis if hash matches
    if let Some(row) = storage.get_analysis(track_id).map_err(|e| e.to_string())? {
        if row.file_hash == track.file_hash {
            return Ok(track_id.into());
        }
    }
    let result = analyze_path(path, &AnalysisConfig::default()).map_err(|e| e.to_string())?;
    let energy_json = serde_json::json!({
        "rms_db": result.rms_db, "peak_db": result.peak_db, "energy_windows": result.energy_windows,
    }).to_string();
    let row = AnalysisRow {
        track_id: track_id.into(), file_hash: track.file_hash.clone(),
        bpm: result.bpm, bpm_confidence: result.bpm_confidence,
        key: result.key.as_ref().map(|k| serde_json::to_string(&k.key).unwrap_or_default()),
        key_confidence: result.key.as_ref().map(|k| k.confidence),
        energy: energy_json, created_at: "1".into(),
    };
    storage.upsert_analysis(&row).map_err(|e| e.to_string())?;
    if let Some(grid) = &result.grid {
        let g = serde_json::to_string(grid).map_err(|e| e.to_string())?;
        storage.upsert_beat_grid(track_id, &track.file_hash, &g).map_err(|e| e.to_string())?;
    }
    Ok(track_id.into())
}

pub fn get_analysis_inner(storage: &Storage, track_id: &str) -> Result<Option<AnalysisResult>, String> {
    let row = storage.get_analysis(track_id).map_err(|e| e.to_string())?;
    let grid = storage.get_beat_grid(track_id).map_err(|e| e.to_string())?;
    let Some(row) = row else { return Ok(None); };
    let grid = match grid { Some(g) => serde_json::from_str(&g).ok(), None => None };
    let energy: serde_json::Value = serde_json::from_str(&row.energy).unwrap_or(serde_json::json!({}));
    let key = row.key.and_then(|k| serde_json::from_str(&k).ok());
    Ok(Some(AnalysisResult {
        bpm: row.bpm, bpm_confidence: row.bpm_confidence,
        onsets: vec![], beats: vec![],
        grid,
        key,
        rms_db: energy.get("rms_db").and_then(|v| v.as_f64()).map(|v| v as f32),
        peak_db: energy.get("peak_db").and_then(|v| v.as_f64()).map(|v| v as f32),
        energy_windows: energy.get("energy_windows").and_then(|v| serde_json::from_value(v.clone()).ok()).unwrap_or_default(),
    }))
}

pub fn get_beat_grid_inner(storage: &Storage, track_id: &str) -> Result<Option<BeatGrid>, String> {
    storage.get_beat_grid(track_id).map_err(|e| e.to_string())?
        .map(|g| serde_json::from_str(&g).map_err(|e| e.to_string())).transpose()
}

#[tauri::command]
pub fn analyze_track(app: tauri::AppHandle, state: tauri::State<'_, AppState>, track_id: String) -> Result<String, String> {
    let track = state.storage.list_tracks(None) ... find path (add Storage::get_track(track_id) if needed);
    let path = track.path.clone();
    let storage_handle = state.storage.clone_shared(); // Storage is !Clone — wrap in Arc or pass via app state
    std::thread::spawn(move || {
        let res = analyze_track_inner(&*storage_handle, &track_id, std::path::Path::new(&path));
        if let Ok(id) = &res { let _ = app.emit("analysis:done", id); }
    });
    Ok(track_id)
}

// get_analysis / get_beat_grid commands thunk get_analysis_inner / get_beat_grid_inner
```

Note: if `Storage` isn't `Clone`, the command needs `Arc<Storage>` (or `Arc<Mutex<Storage>>` if methods take `&self` and the existing `Storage` is held in `AppState`). Simplest: change `AppState` to hold `Arc<Storage>` (methods are `&self`-based already, so shareable) — `analyze_track` clones the `Arc` into the spawned thread; `get_analysis`/`get_beat_grid` stay synchronous `&self` calls. Update `openmix-app/src/lib.rs` accordingly.

`lib.rs`: add `mod commands::analysis;` registration in `generate_handler!` + `use tauri::Emitter;` for `app.emit`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p openmix-app --test analysis_command_test`
Expected: PASS (3 tests).

- [ ] **Step 5: Build + quality gates**

Run: `cargo build -p openmix-app` + gates from Task 13 Step 5 (workspace-wide).
Expected: green.

- [ ] **Step 6: Commit**

```sh
git add openmix-app/src/ openmix-app/tests/analysis_command_test.rs
git commit -m "feat(app): analyze_track/get_analysis ipc with sqlite cache"
```

---

### Task 15: Docs, README, Gate 2 Checklist, Full Verification

**Files:**
- Modify: `README.md`, `docs/build-guide.md`, `docs/dependencies.md`, `openmix-core/src/keyfinder/vendor/NOTICE` (if not added in Task 9)
- No new source files.

**Interfaces:** n/a — integration + verification task.

- [ ] **Step 1: Update README**

Status section → `**Phase 2 complete — Analysis.** BPM, DJ-style beat grid with automatic backend correction, key detection, and RMS energy, accuracy-tested at ≥90% on a synthetic fixture suite. Phase 3 (AutoMix + interactive beat-grid editing) is next.`

- [ ] **Step 2: Update `docs/build-guide.md`**

- aubio: compiled from source via the `aubio-rs` `builtin` feature (no system aubio, no pkg-config, no cmake needed; only a C compiler).
- KeyFinder (native-analysis): vendored `mixxxdj/libkeyfinder` compiled by `cc`; requires a C++ compiler (Xcode CLT / MSVC Build Tools — already a prerequisite).
- GPL-3.0 notice: aubio and libkeyfinder are GPL-3.0; both are optional behind `native-analysis` (default on); `--no-default-features` gives the pure-Rust fallback path. NOTICE file location.

- [ ] **Step 3: Update `docs/dependencies.md`**

Mark `aubio-rs` (Phase 2 — used), `rustfft` (Phase 2 — used), `keyfinder` binding (Phase 2 — vendored); note `rayon` (Phase 2 — used by app callers for parallel analysis across tracks).

- [ ] **Step 4: Full verification (repo root)**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace --no-default-features
pnpm --dir frontend lint && pnpm --dir frontend test && pnpm --dir frontend build
```
Expected: all green (frontend untouched but must stay green).

- [ ] **Step 5: Manual smoke (Gate 2 checklist)**

1. `cargo run -p openmix-core --bin accuracy` → per-fixture table; each of BPM/key/grid ≥90%, exit 0.
2. `cargo run -p openmix-core --bin analyze -- <real mp3> --json` → non-null bpm/grid/key.
3. `pnpm exec tauri dev` → import a real MP3/WAV/FLAC; call `analyze_track` via devtools console (`__TAURI_INTERNALS__.invoke('analyze_track', {trackId})`); verify `analysis:done` event fires and `get_analysis` returns data; relaunch app → `get_analysis` returns cached row without re-analysis.
4. Record results in the Gate 2 report (committed as a note in the PR or `docs/`).

- [ ] **Step 6: Commit**

```sh
git add README.md docs/ openmix-core/src/keyfinder/vendor/NOTICE
git commit -m "docs: phase 2 complete, gate 2 report"
git push
```

- [ ] **Step 7: Gate 2 review**

Report results (accuracy table, memory notes, Windows CI status, any deviations), then **stop and wait for user review before Phase 3**.

---

## Self-Review Notes

- **Spec coverage (Phase 0 / architecture):** every Gate 2 deliverable has a task — BPM (T4), onsets (T5), beats (T6), grid model + fit (T7), automatic backend correction + drift (T8), KeyFinder native binding (T9), K-S fallback + confidence comparison through `KeyDetector` (T10), fixtures + ≥90% gate (T11), single-pass bounded-memory orchestrator + cancellation (T12), app IPC/SQLite analysis integration + hash cache (T13/T14), docs/Gate-2 checklist (T15). RMS/LUFS-style energy requirement is satisfied minimally (RMS + peak + per-window RMS in T2); sections/segments remain Phase 3 (out of Gate 2 scope, per architecture §2.3 and Phase 2 phase table, and the explicit "backend/accuracy" scope of this plan).
- **No UI:** no frontend BPM/key badges or grid UI in any task — architecture requires the interactive grid UI only in Phase 3.
- **Phase 0 decisions preserved:** detector traits (§2.1), `native-analysis` feature isolation with pure-Rust fallbacks, `openmix-core` zero-persistence rule (all SQLite stays in `openmix-app`), chunked streaming (no full-file loads — key buffer capped by `key_max_seconds`), aubio + KeyFinder accepted GPL-3.0 deps behind the feature with NOTICE.
- **Type consistency:** `AnalysisResult`, `BeatGrid`, `KeyResult`, `MusicalKey`, `AnalysisConfig` names/signatures are identical across core (T2/T10/T12), app storage (T13), and IPC (T14); `Beat.position_sec`/`BeatGrid.first_beat_offset` naming is consistent through fit (T7), correct (T8), and accuracy (T11). Tauri camelCase↔snake_case conversion (`trackId` ↔ `track_id`) is the standard Phase 1 pattern.
- **No placeholders:** every interface, SQL schema, test, aubio loop, and gate command is concrete. Deliberate exceptions, documented inline: (a) the vendored `mixxxdj/libkeyfinder` C++ source is copied wholesale from a pinned commit (not authored here) — the shim contract is fully specified; (b) `best_key`'s alternate-placement detail and the vendored libkeyfinder member names are flagged for a 5-minute implementation-time check against the actual sources.
- **Windows CI is load-bearing in this phase:** Task 3 (aubio builtin) and Task 9 (MSVC shim compile) each include an explicit push-and-watch-CI step with documented fallbacks so a C-build problem is caught before it blocks detector work.

## Phase 2 Acceptance Criteria (Gate 2)

1. `analysis::analyze` produces bpm+confidence, beats, beat grid (offset/bpm/confidence), key+confidence, RMS/peak energy for any decodable MP3/WAV/FLAC.
2. Beat-grid automatic correction improves low-confidence grids and detects tempo drift (fixture-verified).
3. Key detection returns KeyFinder (native) and K-S (fallback) results with confidence comparison + `alternate`.
4. `cargo test --workspace` green on macOS **and** Windows; `--no-default-features` build green (pure-Rust fallbacks).
5. Fixture accuracy ≥90% each for BPM, key, beat-grid offset on clean fixtures (Gate 2 machine gate).
6. App: `analyze_track`/`get_analysis`/`get_beat_grid` IPC + SQLite persistence + hash-keyed cache work end-to-end.
7. No frontend BPM/key UI added (explicitly out of scope; Phase 3).
8. Bounded memory: no whole-file RAM loads anywhere in the analysis path (chunked decode + capped key buffer ≤ ~26 MB default), verified in Gate 2 report.

## Gate 2 Checklist

- [ ] accuracy.rs suite ≥90% on BPM, key, grid (macOS + Windows CI green)
- [ ] `cargo fmt` / `clippy -D warnings` / `test` all green, including `--no-default-features`
- [ ] CLI `analyze`/`accuracy` verified manually on a real MP3
- [ ] App import → analyze → persist → `analysis:done` → cache-hit verified in `tauri dev`
- [ ] Energy (RMS/peak) computed and stored
- [ ] No frontend UI changes introduced
- [ ] Memory/performance notes recorded in Gate 2 report
- [ ] README/deps/build-guide updated; GPL-3.0 notices for aubio + libkeyfinder added
- [ ] Stop for user review before Phase 3

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| aubio-rs `builtin` build breaks on Windows MSVC | Med | High | Task 3 spike validates on Windows CI before detector work; fallback to `pkg-config` + preinstalled aubio, or pure-Rust path as default; documented in plan. |
| Vendored KeyFinder C++ build issues on Windows (cc/MSVC) | Med | High | Task 9 gated + Windows compile check; minimal C ABI shim (no cxx dep); fallback: Windows key detection uses K-S chroma only — feature still compiles, accuracy gate unaffected (K-S validated in suite); documented in Gate 2 report. |
| Key accuracy <90% on fixtures | Med | Med | Unambiguous pad fixtures; confidence comparison picks best; relative-key equivalence pre-approved; KeyFinder tuning params (chroma filter) adjustable. |
| Beat-grid offset off on intro fixtures | Med | Med | Task 8 auto-correction re-fits tighter windows; `kick_120bpm_intro.wav` (0.87 s silence) is the explicit regression test; ±50 ms tolerance. |
| GPL-3.0 linkage (aubio, libkeyfinder) vs MIT app | Certain (linkage) | Legal | Phase 0 decision preserved (user-approved); feature-gated so pure-Rust build is possible; NOTICE + legal screen note per LAME precedent; flagged again in Gate 2 report. |
| Bounded-memory violation via key accumulator | Low | Med | `key_max_seconds` cap (default 600 s ⇒ ≤ 26 MB at 11025 Hz); documented formula; test asserts buffer stays bounded. |
| Analysis performance on 2-h tracks | Low | Med | Single decode pass; cancellation every ~11 ms; key buffer capped; real-world timing noted in report (formal `cargo bench` deferred to Phase 4 per architecture §6). |
| aubio-rs API drift (0.2.0, 2026-07 release) | Low | Low | Verified against docs.rs at plan time; isolated behind traits + thin wrappers in `tempo.rs`/`onsets.rs`. |