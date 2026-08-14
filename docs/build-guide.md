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

# 3. Tauri CLI (npm devDependency in frontend — no global install)
#    Installed automatically by `pnpm install` via `@tauri-apps/cli`

# 4. Node + pnpm (Node 22 already installed here)
npm install -g pnpm@9

# 5. C dependencies (aubio, KeyFinder) — compiled by Rust build scripts
#    (cc crate) with no CMake, no pkg-config, no system installs.
#    Needs a C++ compiler (comes with Xcode CLT) and libclang for bindgen
#    (provided by Xcode CLT on macOS; preinstalled on GitHub windows runners).
brew install cmake   # only needed for packaging (Tauri); analysis builds don't use it
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

# 5. Tauri CLI (npm devDependency in frontend — no global install)
#    Installed automatically by `pnpm install` via `@tauri-apps/cli`

# 6. WiX v3 (bundled automatically by Tauri for .msi) — .exe via NSIS: see Tauri docs
```

## Dev workflow

```sh
# One-time: install JS deps
pnpm install --dir frontend

# Run the app (dev mode: Vite HMR + cargo run)
# The Tauri CLI must run from openmix-app/ (where tauri.conf.json lives);
# the binary comes from the frontend node_modules.
cd openmix-app && ../frontend/node_modules/.bin/tauri dev

# Headless engine work (analysis/render) without GUI:
cargo test --workspace            # all Rust tests
cargo test -p openmix-core        # engine tests only
```

### Frontend-only dev

```sh
pnpm --dir frontend dev           # Vite dev server (UI without backend features)
```

### End-to-end tests (mocked IPC)

Phase 1 uses Playwright against the Vite dev server with Tauri's `mockIPC`
(mocks all Rust commands with in-memory data). This runs in a normal browser,
so it works on every OS. Real-webview E2E via `tauri-driver` is
Windows/Linux-only (no macOS support) and lands in Phase 5.

```sh
pnpm --dir frontend exec playwright install chromium   # one-time
pnpm --dir frontend e2e                                # 2 tests, VITE_E2E=true
```

> CI runs the same suite on Ubuntu (`pnpm e2e`), including the browser install.

## Quality gates (run before every commit / PR)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check --workspace --no-default-features   # native-analysis feature isolation
pnpm --dir frontend lint
pnpm --dir frontend test          # Vitest
pnpm --dir frontend build         # production frontend build
pnpm --dir frontend e2e           # Playwright (mocked IPC)
```

## Packaging

```sh
# From openmix-app/ (cd openmix-app first)
../frontend/node_modules/.bin/tauri build

# macOS → .dmg  (Apple Silicon + Intel via universal builds in CI)
# Windows → .exe (NSIS installer)

# CI does both OSes; local packaging on Windows requires Windows runner
```

## Notes

- `openmix-core` has **zero Tauri dependency** — `cargo test -p openmix-core`
  works on a bare machine with only Rust, enabling headless CI and quick iteration.
- Analysis C/C++ deps are compiled at build time by Rust build scripts:
  - **aubio**: compiled from source via the `aubio-rs` `builtin` feature
    (no system aubio, no pkg-config, no CMake — only a C compiler and
    libclang for bindgen). GPL-3.0.
  - **KeyFinder** (`native-analysis`): vendored `mixxxdj/libkeyfinder` C++
    compiled by the `cc` crate; requires a C++ compiler (Xcode CLT /
    MSVC Build Tools — already a prerequisite). GPL-3.0.
  - Both are optional behind `native-analysis` (default on);
    `cargo check --workspace --no-default-features` builds the pure-Rust
    fallback path. See [THIRD_PARTY_NOTICES.md](../THIRD_PARTY_NOTICES.md).
  - First build will take a few minutes (aubio + KeyFinder + bindgen).
- Keep Node ≥ 20 (Vite 6 requirement); Node 22 is the pinned baseline.