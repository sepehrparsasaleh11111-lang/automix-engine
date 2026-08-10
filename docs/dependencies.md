# OpenMix AI — Dependency List

All dependencies are open-source and local-only. Nothing phones home.
Dependency phase: when the crate enters the codebase and becomes load-bearing.

## Rust — openmix-core

| Crate | Phase | Purpose | Rationale / alternative |
|-------|-------|---------|--------------------------|
| `symphonia` | 1 | MP3/WAV/FLAC decoding, streaming chunk reads | Pure Rust, no system deps; the reason we don't need ffmpeg |
| `aubio-rs` (aubio C lib) | 2 | Tempo/onset/beat detection | Industry-standard beat tracking; accepted C dep (user decision) |
| `keyfinder` bindings (KeyFinder C++) | 2 | Musical key detection | Mature algorithm; behind `KeyDetector` trait |
| `rubato` | 3 | Time-stretch + resampling | High-quality TDHS, pure Rust; behind replaceable trait |
| `rusqlite` (bundled) | 1 | SQLite storage | Simple, sync, compile-safe; no async complexity needed |
| `rayon` | 2 | Parallel analysis | Data-parallel CPU work |
| `tokio` | 1 | Async IPC/event runtime (core side shared with app) | Standard async runtime |
| `cpal` | 3 (preview) | Native audio output for live preview | Cross-platform output |
| MP3 encoder (e.g. `lame` bindings or pure-Rust `mp3-encoder`) | 4 | MP3 export | Chosen at render phase; WAV/FLAC via symphonia |
| `serde` / `serde_json` | 1 | IPC + config serialization | Standard |
| `thiserror` | 1 | Typed `AppError` | Ergonomic errors |
| `uuid` | 1 | Track/project/renders IDs | Standard |
| `log` + `tracing` | 1 | Structured logging | Layer per phase |

## Rust — openmix-app (Tauri 2)

| Crate | Purpose |
|-------|---------|
| `tauri` 2.x | App shell, IPC, windows |
| `tauri-plugin-dialog` | Native open-file dialogs (import) |
| `tauri-plugin-fs` | Safe file access paths |
| `tauri-plugin-sql` *(optional)* | If frontend needs direct DB access (avoid; prefer core storage API) |
| `tauri-plugin-shell` *(optional)* | Open export folder in Finder/Explorer |

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
| `wavesurfer` *(evaluate)* | Waveform rendering — likely replaced by custom canvas renderer for beat-grid overlay + zoom |

## System deps (build time only)

| Dep | Why |
|-----|-----|
| CMake | Builds aubio + KeyFinder |
| C/C++ compiler (clang/MSVC) | Compiles C/C++ deps |
| Xcode CLT (macOS) | Apple toolchain, clang, cmake |
| WebView2 (Windows) | Tauri webview runtime (preinstalled on Win11) |
| WiX/NSIS (Windows, packaging) | .msi/.exe installers (bundled by Tauri) |

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