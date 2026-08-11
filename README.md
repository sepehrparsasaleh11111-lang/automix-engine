# OpenMix AI

Free, open-source, fully-local automatic DJ mixing for your desktop.

OpenMix AI automatically mixes your music library like a DJ — beat-matched
transitions, harmonic mixing, and rendered mix files — entirely on your own
computer. Audio never leaves your device: no cloud processing, no cloud
storage, no paid APIs, no accounts.

## Status

**Phase 0 — Planning.** Architecture and technical planning are complete;
application code begins at Phase 1.

## Features (planned)

- Import MP3 / WAV / FLAC from your library
- Automatic BPM detection, DJ-style beat grid, and musical key detection
- AutoMix engine: beat matching, transition scoring, crossfades, EQ, effects
- Export to MP3 / WAV / FLAC with background rendering
- Projects, mix presets, and recent mixes, stored locally in SQLite
- Windows 10/11 and macOS (Apple Silicon + Intel)

## Architecture

- **Frontend:** React + TypeScript + Tailwind (Vite)
- **Shell:** Tauri 2
- **Engine:** Rust (`openmix-core` — audio analysis, beat grid, AutoMix, rendering)
- **Storage:** SQLite (`rusqlite`, in the app layer)
- **Analysis:** aubio (tempo/onsets/beats), KeyFinder (key), LAME (MP3 export)

See [architecture.md](architecture.md) for the full technical design and
[docs/](docs/) for the build guide, dependency list, and folder structure.

## Building

Requirements: Rust (stable), Node.js 22+, pnpm, Tauri CLI, CMake + C/C++
compiler (Xcode CLT on macOS, MSVC Build Tools on Windows).

```sh
pnpm install --dir frontend
cargo tauri dev        # run the app
cargo test --workspace # engine tests
```

Full instructions: [docs/build-guide.md](docs/build-guide.md)

## License

MIT — see [LICENSE](LICENSE).

Third-party notices: LAME (LGPL) is used for MP3 encoding; its license
requires attribution (added to the legal/about screen at Phase 4).
