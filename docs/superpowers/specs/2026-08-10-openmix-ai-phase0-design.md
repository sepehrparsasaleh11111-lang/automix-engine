# OpenMix AI — Phase 0 Design Spec

Date: 2026-08-10
Status: Approved by user (sections 1–3)

## Purpose

Free, open-source, fully-local automatic DJ mixing desktop application.
No cloud audio processing, no paid APIs, no accounts. All analysis, mixing,
and rendering happen on the user's computer.

## Scope of this cycle

Phase 0 only: technical planning documents and repository setup.
No application code.

## Decisions (user-approved)

| Topic | Decision |
|-------|----------|
| Scope | Phase 0 planning docs only (`architecture.md`, build guide, dependency list, folder structure) |
| Organization | Cargo workspace: `openmix-core` (engine, no Tauri dep, `cargo test`-able) + `openmix-app` (Tauri 2 shell) + `frontend/` (React) |
| Beat/tempo/onset | aubio C library via bindings (`aubio-rs`) |
| Key detection | KeyFinder C++ via bindings, behind replaceable `KeyDetector` trait; Krumhansl-Schmuckler chroma as fallback/future second algorithm with confidence comparison |
| Time-stretch/resample | `rubato` (pure Rust), behind replaceable trait |
| Audio decoding | `symphonia` (MP3/WAV/FLAC, streaming, chunked) |
| Storage | SQLite via `rusqlite` (bundled), behind a storage module |
| Concurrency | tokio (IPC/async glue) + rayon (analysis parallelism) |
| Frontend | Vite + React + TypeScript + Tailwind, Zustand state, `<canvas>` waveforms, Tauri IPC + dialog/fs plugins |
| Dev platform | macOS first; Windows via CI + cross-platform-safe code |
| Source control | git init + GitHub repo (gh CLI), MIT license |
| Testing | Rust unit + integration tests in `openmix-core` with golden fixtures; `cargo test`/`clippy`/`fmt` gates; Vitest + Playwright for frontend |

## Repo layout

```
automix1/
├── architecture.md          # master technical design (all phases)
├── docs/
│   ├── build-guide.md       # toolchain setup, dev workflow, packaging
│   ├── dependencies.md      # dependency list by phase with rationale
│   ├── folder-structure.md  # canonical folder layout
│   └── superpowers/specs/   # design specs (this file)
├── openmix-core/            # Rust engine crate (no Tauri dep)
├── openmix-app/             # Tauri 2 shell crate
└── frontend/                # React + TS + Tailwind
```

## Phase gates

- **Gate 0:** docs committed, GitHub repo live, CI smoke (fmt/lint/build)
- **Gate 1:** app opens, imports MP3/WAV/FLAC, waveform + project CRUD in SQLite
- **Gate 2:** BPM/beatgrid/key correct on fixture suite (≥90% accuracy on clean tracks)
- **Gate 3:** auto-mix output beat-matched; manual grid corrections work
- **Gate 4:** 2-hour render stable, background render with progress, streamed memory
- **Gate 5:** signed installers macOS + Windows, issue templates, docs polished

## Risk register

1. **aubio binding build pain on Windows** → macOS-first dev; pure-Rust fallback stub behind trait
2. **KeyFinder accuracy on electronic music** → K-S fallback + confidence scoring, replaceable trait
3. **Beat-grid drift on live/variable-BPM tracks** → manual correction UI is a priority
4. **Long-render stability** → chunked streaming from day one, no full-file buffering

## Next step

Writing the plan (writing-plans skill) → execute Phase 0 docs, then Gate 0.