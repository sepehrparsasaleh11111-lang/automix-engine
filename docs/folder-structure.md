# OpenMix AI — Folder Structure

Canonical layout. Phases add directories; nothing below is removed.

```
automix1/
├── architecture.md               # master technical design (living doc)
├── LICENSE                       # MIT (added at repo init)
├── README.md                     # minimal at Gate 0 → polished Phase 5
├── CONTRIBUTING.md               # Phase 5
├── .gitignore
├── .github/
│   ├── ISSUE_TEMPLATE/           # bug report + feature request (Phase 5)
│   └── workflows/
│       ├── ci.yml                # fmt/clippy/test + frontend build, macOS+Windows matrix (Gate 0; jobs activate when Phase 1 lands)
│       └── release.yml           # installers + GitHub release (Phase 5)
│
├── docs/
│   ├── build-guide.md
│   ├── dependencies.md
│   ├── folder-structure.md       # this file
│   └── superpowers/specs/        # design specs
│
├── openmix-core/                 # engine crate — NO Tauri deps, cargo test-able alone
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── error.rs              # AppError (no storage variants — persistence is app-layer)
│   │   ├── audio/                # decoded stream adapter, chunking, resample glue
│   │   │   ├── decode.rs         #   symphonia wrapper
│   │   │   └── peaks.rs          #   zoomable waveform peaks
│   │   ├── analysis/
│   │   │   ├── mod.rs            #   traits + runner (rayon), cancellation
│   │   │   ├── tempo.rs          #   aubio BPM [native-analysis]
│   │   │   ├── onsets.rs         #   aubio onsets [native-analysis]
│   │   │   ├── beats.rs          #   aubio beat tracker [native-analysis]
│   │   │   ├── key.rs            #   KeyFinder binding [native-analysis] + K-S fallback
│   │   │   ├── energy.rs         #   RMS/LUFS-style loudness
│   │   │   └── sections.rs       #   structure/segments
│   │   ├── beatgrid/
│   │   │   ├── mod.rs            #   grid model, fit, drift correction
│   │   │   └── correct.rs        #   auto + manual correction logic
│   │   ├── automix/
│   │   │   ├── mod.rs            #   orchestrator
│   │   │   ├── matcher.rs        #   beat matching (rubato behind trait)
│   │   │   ├── transition.rs     #   scoring + selection
│   │   │   └── effects/
│   │   │       ├── mod.rs        #   effect chain trait
│   │   │       ├── crossfade.rs
│   │   │       ├── eq.rs
│   │   │       ├── bass_swap.rs
│   │   │       ├── filter.rs
│   │   │       ├── reverb.rs
│   │   │       └── delay.rs
│   │   ├── render/
│   │   │   ├── mod.rs            #   offline render orchestrator
│   │   │   ├── pipeline.rs       #   chunked streaming pipeline
│   │   │   └── encoder.rs        #   MP3/WAV/FLAC writers
│   │   ├── preview/              #   (Phase 3) live audition / cpal output
│   └── tests/
│       ├── fixtures/             # synthetic tracks with known BPM/key (golden)
│       └── analysis.rs, render.rs, ...
│
├── openmix-app/                  # Tauri 2 shell
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/             # Tauri permission capabilities
│   ├── icons/
│   └── src/
│       ├── main.rs
│       ├── lib.rs
│       ├── storage/               # SQLite (rusqlite) — app-layer persistence
│       │   ├── mod.rs             #   storage API; rusqlite connection
│       │   ├── db.rs              #   migrations, schema runner
│       │   └── schema.sql
│       └── commands/              # thin IPC commands → openmix-core
│           ├── tracks.rs
│           ├── projects.rs
│           ├── analysis.rs
│           ├── automix.rs
│           └── render.rs
│
└── frontend/                     # React UI
    ├── package.json
    ├── vite.config.ts
    ├── tailwind.config.ts
    ├── index.html
    └── src/
        ├── main.tsx
        ├── App.tsx
        ├── styles/
        ├── api/                  # typed wrappers around Tauri IPC
        ├── store/                # Zustand slices: library, project, transport, render
        ├── components/
        │   ├── library/          #   track library, import
        │   ├── waveform/         #   canvas waveform + beat grid overlay + zoom
        │   ├── mixer/            #   decks, BPM/key/energy readouts, transport
        │   ├── transitions/      #   transition config UI
        │   └── export/           #   format/quality/destination
        ├── pages/
        │   ├── Home.tsx          #   start screen: recent mixes, projects
        │   ├── Mixer.tsx
        │   └── Export.tsx
        └── tests/
```

## Design rules the layout enforces

1. `openmix-core` never imports `tauri` **and never persists** (no rusqlite;
   storage lives in `openmix-app/src/storage/`) — engine usable headless
   (tests, CI).
2. Frontend never touches SQLite or files directly — only IPC commands.
3. Detectors implement traits from `analysis/mod.rs` — replaceable per decision.
   aubio/KeyFinder code is gated by the `native-analysis` cargo feature
   (default on); the crate compiles without it using pure-Rust fallbacks.
4. Golden fixtures live in `openmix-core/tests/fixtures/` (generated by a
   checked-in generator script so they stay reproducible).