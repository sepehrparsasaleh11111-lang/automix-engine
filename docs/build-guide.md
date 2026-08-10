# OpenMix AI — Build Guide

How to set up the toolchain, develop, test, and package OpenMix AI on macOS and Windows.

## Prerequisites (all platforms)

| Tool | Version | Purpose |
|------|---------|---------|
| Rust (rustup) | stable | Core engine + Tauri shell |
| Node.js | 22 LTS+ | Frontend toolchain (Vite) |
| pnpm (or npm) | latest | Frontend package manager |
| Tauri CLI | 2.x | Build/run/package the app |

## macOS

```sh
# 1. Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Xcode Command Line Tools (already present if you have Xcode)
xcode-select --install

# 3. Tauri CLI (via cargo)
cargo install tauri-cli --version "^2"

# 4. Node + pnpm (Node 22 already installed here)
npm install -g pnpm

# 5. C dependencies (aubio, KeyFinder) — built via CMake by Rust build scripts;
#    needs `cmake` and a C++ compiler (comes with Xcode CLT)
brew install cmake
```

## Windows 10/11

```sh
# 1. Rust via rustup (x64; ARM64 cross-compile: add aarch64-pc-windows-msvc target)
winget install Rustlang.Rustup

# 2. WebView2 (usually preinstalled on Win11)
# 3. Visual Studio Build Tools 2022 with "Desktop development with C++"
#    (MSVC + CMake)
# 4. Node.js LTS 22
winget install OpenJS.NodeJS.LTS

# 5. Tauri CLI
cargo install tauri-cli --version "^2"

# 6. WiX v3 (bundled automatically by Tauri for .msi) — .exe via NSIS: see Tauri docs
```

## Dev workflow

```sh
# One-time: install JS deps
pnpm install --dir frontend

# Run the app (dev mode: Vite HMR + cargo run)
cargo tauri dev

# Headless engine work (analysis/render) without GUI:
cargo test --workspace            # all Rust tests
cargo test -p openmix-core        # engine tests only
```

### Frontend-only dev

```sh
pnpm --dir frontend dev           # Vite dev server (UI without backend features)
```

## Quality gates (run before every commit / PR)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --dir frontend lint
pnpm --dir frontend test          # Vitest
pnpm --dir frontend build         # production frontend build
```

## Packaging

```sh
# macOS → .dmg  (Apple Silicon + Intel via universal builds in CI)
cargo tauri build

# Windows → .exe (NSIS installer)
cargo tauri build

# CI does both OSes; local packaging on Windows requires Windows runner
```

## Notes

- `openmix-core` has **zero Tauri dependency** — `cargo test -p openmix-core`
  works on a bare machine with only Rust, enabling headless CI and quick iteration.
- aubio/KeyFinder are C/C++ deps compiled at build time via CMake;
  first build will take a few minutes.
- Keep Node ≥ 20 (Vite 6 requirement); Node 22 is the pinned baseline.