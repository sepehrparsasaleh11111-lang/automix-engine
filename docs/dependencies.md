# OpenMix AI — Dependency List

All dependencies are open-source and local-only. Nothing phones home.
Dependency phase: when the crate enters the codebase and becomes load-bearing.

## Rust — openmix-core

| Crate | Phase | Purpose | Rationale / alternative |
|-------|-------|---------|--------------------------|
| `symphonia` | 1 | MP3/WAV/FLAC decoding, streaming chunk reads | Pure Rust, no system deps; the reason we don't need ffmpeg |
| `aubio-rs` (aubio C lib) | 2 | Tempo/onset/beat detection | Industry-standard beat tracking; accepted C dep (user decision); behind traits + `native-analysis` feature |
| `keyfinder` bindings (KeyFinder C++) | 2 | Musical key detection | Mature algorithm; behind `KeyDetector` trait + `native-analysis` feature |
| `rubato` | 3 | Time-stretch + resampling | High-quality TDHS, pure Rust; behind replaceable trait |
| `rayon` | 2 | Parallel analysis | Data-parallel CPU work |
| `tokio` | 1 | Async IPC/event runtime (core side shared with app) | Standard async runtime |
| `cpal` | 3 (preview) | Native audio output for live preview | Cross-platform output |
| MP3 encoder: `lame-rs` (LAME C) | 4 | MP3 export — **decided**: LAME is the reference encoder; audio quality is priority #1 | LGPL; attribution in README + legal screen. Pure-Rust encoders not used at this quality bar |
| `serde` / `serde_json` | 1 | IPC + config serialization | Standard |
| `thiserror` | 1 | Typed `AppError` (core: decode/analysis/grid/render/io — no storage variants) | Ergonomic errors |
| `uuid` | 1 | Track/project/renders IDs | Standard |
| `log` + `tracing` | 1 | Structured logging | Layer per phase |

**Feature isolation (`openmix-core` Cargo.toml):**

- `native-analysis` (default on): enables `aubio-rs` + `keyfinder` bindings.
- Without it the crate compiles pure-Rust fallback stubs behind the same
  detector traits (autocorrelation tempo, spectral-flux onsets, K-S chroma).
- Windows/macOS CI can build both configurations; users and packagers can
  disable native analysis per-platform if a C build misbehaves.

## Rust — openmix-app (Tauri 2)

| Crate | Purpose |
|-------|---------|
| `tauri` 2.x | App shell, IPC, windows |
| `tauri-plugin-dialog` | Native open-file dialogs (import) |
| `tauri-plugin-fs` | Safe file access paths |
| `tauri-plugin-shell` *(optional)* | Open export folder in Finder/Explorer |
| `rusqlite` (bundled) | SQLite storage — lives here, not in core. Tables: projects, tracks, track_analysis, beat_grids, mix_presets, preferences, renders |

## Frontend (frontend/)

| Package | Purpose |
|---------|---------|
| Vite 6 | Dev server + bundler |
| React 18/19 | UI |
| TypeScript 5 | Types |
| Tailwind CSS 4 | Styling |
| Zustand | Client state (projects, tracks, transport) |
| `@tauri-apps/api` | IPC, events |
| Vitest + Testing Library | Unit/component tests |
| Playwright | E2E against the packaged app |

**Waveform rendering:** custom `<canvas>` renderer (min/max peak tables from
core, beat-grid/bar/phrase overlays, zoom). No third-party waveform library —
a dedicated renderer is required for beat-grid overlay precision anyway.

## System deps (build time only)

| Dep | Why |
|-----|-----|
| CMake | Builds aubio + KeyFinder + LAME (only when `native-analysis` / MP3 encoding enabled) |
| C/C++ compiler (clang/MSVC) | Compiles C/C++ deps |
| Xcode CLT (macOS) | Apple toolchain, clang, cmake |
| WebView2 (Windows) | Tauri webview runtime (preinstalled on Win11) |
| WiX/NSIS (Windows, packaging) | .msi/.exe installers (bundled by Tauri) |

## CI (GitHub Actions)

| Action | Purpose |
|--------|---------|
| `actions/checkout` | Clone |
| `dtolnay/rust-toolchain` | Rust stable + clippy/rustfmt components |
| `actions/setup-node` | Node 22 |
| `pnpm/action-setup` | pnpm for frontend |

## Explicitly excluded

| Thing | Why |
|-------|-----|
| ffmpeg | Heavy system dep; symphonia covers the three formats |
| Python/librosa | Runtime dependency + slow startup; analysis is Rust/aubio |
| Cloud services, paid APIs, accounts | Hard requirement |
| GPU/neural deps for base app | Future AI features only, still local |

## Version pinning policy

- Rust crates: semver-compatible ranges; lockfile committed.
- JS: lockfile committed (`pnpm-lock.yaml`).
- Long-lived C vendoring decision deferred to Phase 2 when bindings are added
  (aubio can be fetched via build script; revisit if builds are flaky).