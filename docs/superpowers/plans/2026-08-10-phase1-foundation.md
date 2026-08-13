# Phase 1: Application Foundation — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A runnable Tauri 2 desktop app that opens, imports MP3/WAV/FLAC files, generates waveforms, and manages projects/tracks in SQLite — Gate 1.

**Architecture:** Cargo workspace (`openmix-core` engine crate with zero Tauri/persistence deps + `openmix-app` Tauri shell owning SQLite storage) and a Vite+React frontend communicating only via typed IPC commands. Decoding, waveform peaks, and file hashing are chunked/streaming from day one.

**Tech Stack:** Rust stable, Tauri 2 (npm CLI `@tauri-apps/cli`), symphonia (decode), rusqlite bundled (storage), React 19, Vite 6, Tailwind 4, Zustand 5, Vitest 3 (component tests), Playwright (E2E with mocked IPC — `tauri-driver` does not support macOS).

## Global Constraints

- macOS-first dev machine; Windows/macOS CI matrix validates every change.
- All processing local. No cloud, no accounts, no paid services, no telemetry.
- Audio files referenced by path on user's device — never copied.
- `openmix-core`: zero `tauri` deps, zero persistence (no rusqlite). Storage only in `openmix-app/src/storage/`.
- Never load whole audio files into RAM — chunked streaming only (decode chunks, peaks buckets, SHA-256 over a 64 KiB bounded buffer).
- Formats in Phase 1: MP3, WAV, FLAC.
- TDD (failing test → implement → passing test), `cargo fmt/clippy -D warnings` green, frequent commits.
- No code comments unless required for non-obvious logic.
- Versions: tauri 2.x, symphonia 0.5, rusqlite 0.32 (bundled), react 19, vite 6, tailwind 4, vitest 3, pnpm 9, node 22.
- Icons: generated from a Python-generated PNG (`scripts/gen-icon.py`) via `pnpm tauri icon`.

---

### Task 1: Toolchain Bootstrap

**Files:** none (system setup)

**Interfaces:** n/a — provides `rustc`, `cargo`, `pnpm` to all later tasks.

- [ ] **Step 1: Install Rust via rustup (stable)**

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"
```

- [ ] **Step 2: Install pnpm 9 (corepack absent on this machine)**

```sh
npm install -g pnpm@9
```

- [ ] **Step 3: Verify every installation actually succeeded**

Run: `rustc --version && cargo --version && pnpm --version`
Expected: `rustc 1.8x.x`, `cargo 1.8x.x`, `pnpm 9.x.y` (any stable Rust >= 1.80).
If a system-level install fails (permissions/environment), STOP and report the exact blocker.

No commit (no repo changes).

---

### Task 2: Cargo Workspace + openmix-core Skeleton

**Files:**
- Create: `Cargo.toml` (root workspace)
- Create: `openmix-core/Cargo.toml`
- Create: `openmix-core/src/lib.rs`
- Create: `openmix-core/src/error.rs`
- Test: `openmix-core/src/lib.rs` (unit tests inline)

**Interfaces:**
- Produces: crate `openmix-core` with `pub struct AppError` (thiserror) — `#[derive(Debug, thiserror::Error)]`, `Send + Sync`; and `pub fn engine_name() -> &'static str` returning `"openmix-core"`.

- [ ] **Step 1: Write the failing test** (in `openmix-core/src/lib.rs`)

```rust
use crate::error::AppError;

#[test]
fn engine_name_is_openmix_core() {
    assert_eq!(engine_name(), "openmix-core");
}

#[test]
fn app_error_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AppError>();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core`
Expected: FAIL — `cannot find function engine_name`, `cannot find module error`.

- [ ] **Step 3: Write root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["openmix-core", "openmix-app"]

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"
authors = ["OpenMix AI contributors"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
uuid = { version = "1", features = ["v4"] }
symphonia = { version = "0.5", features = ["mp3", "flac", "wav", "pcm"] }
```

- [ ] **Step 4: Write `openmix-core/Cargo.toml`**

```toml
[package]
name = "openmix-core"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[features]
default = ["native-analysis"]
native-analysis = []

[dependencies]
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
symphonia = { workspace = true }
```

- [ ] **Step 5: Write `openmix-core/src/error.rs`**

```rust
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("failed to open audio file {0}: {1}")]
    OpenFile(PathBuf, String),
    #[error("unsupported or corrupt audio format in {0}")]
    UnsupportedFormat(PathBuf),
    #[error("audio decode error: {0}")]
    Decode(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}
```

- [ ] **Step 6: Write `openmix-core/src/lib.rs`** with `mod error; pub use error::AppError; pub fn engine_name() -> &'static str { "openmix-core" }` plus the two tests from Step 1.

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test -p openmix-core`
Expected: PASS (2 tests).

- [ ] **Step 8: Verify quality gates**

Run: `cargo fmt --all --check && cargo clippy -p openmix-core --all-targets -- -D warnings && cargo check -p openmix-core --no-default-features`
Expected: all green. (CI `core` job activates now — `hashFiles('openmix-core/Cargo.toml')` exists.)

- [ ] **Step 9: Commit**

```sh
git add Cargo.toml openmix-core/
git commit -m "feat: cargo workspace with openmix-core skeleton"
```

---

### Task 3: Frontend Scaffold

**Files:**
- Create: `frontend/package.json`
- Create: `frontend/vite.config.ts`
- Create: `frontend/tsconfig.json`, `frontend/tsconfig.node.json`
- Create: `frontend/eslint.config.js`
- Create: `frontend/index.html`
- Create: `frontend/src/main.tsx`, `frontend/src/App.tsx`, `frontend/src/styles/index.css`
- Create: `frontend/src/test-setup.ts`
- Test: `frontend/src/App.test.tsx`

**Interfaces:**
- Produces: `frontend/` with scripts `dev` (port 1420), `build`, `lint`, `test`, `e2e`; React 19 root at `src/main.tsx`; `App` renders heading `OpenMix AI`.

- [ ] **Step 1: Write the failing test** (`frontend/src/App.test.tsx`)

```tsx
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import App from './App';

describe('App', () => {
  it('renders the app title', () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: 'OpenMix AI' })).toBeInTheDocument();
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `pnpm --dir frontend test`
Expected: FAIL — `Cannot find module './App'` / missing package.json.

- [ ] **Step 3: Write `frontend/package.json`**

```json
{
  "name": "openmix-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "lint": "eslint src",
    "test": "vitest run",
    "e2e": "playwright test"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2",
    "react": "^19.1.0",
    "react-dom": "^19.1.0",
    "zustand": "^5.0.0"
  },
  "devDependencies": {
    "@eslint/js": "^9",
    "@playwright/test": "^1.50",
    "@tauri-apps/cli": "^2",
    "@testing-library/jest-dom": "^6",
    "@testing-library/react": "^16",
    "@testing-library/user-event": "^14",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "@vitejs/plugin-react": "^4",
    "eslint": "^9",
    "jsdom": "^26",
    "tailwindcss": "^4",
    "@tailwindcss/vite": "^4",
    "typescript": "~5.7.0",
    "typescript-eslint": "^8",
    "vite": "^6",
    "vitest": "^3"
  }
}
```

- [ ] **Step 4: Write `frontend/vite.config.ts`**

```ts
/// <reference types="vitest/config" />
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
  },
});
```

- [ ] **Step 5: Write `frontend/src/test-setup.ts`** with `import '@testing-library/jest-dom/vitest';`

- [ ] **Step 6: Write `frontend/tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "moduleResolution": "bundler",
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "skipLibCheck": true,
    "isolatedModules": true,
    "noEmit": true
  },
  "include": ["src"]
}
```

- [ ] **Step 7: Write `frontend/tsconfig.node.json`** (`composite: true`, `module: ESNext`, `moduleResolution: bundler`, `allowImportingTsExtensions: true`, `include: ["vite.config.ts"]`) and `frontend/eslint.config.js`:

```js
import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist', 'playwright-report', 'test-results', 'coverage'] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
);
```

- [ ] **Step 8: Write `frontend/index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenMix AI</title>
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

- [ ] **Step 9: Write `frontend/src/styles/index.css`** with `@import "tailwindcss";`

- [ ] **Step 10: Write `frontend/src/main.tsx`** and `frontend/src/App.tsx` (heading `OpenMix AI`, root div styled with Tailwind classes) so the test passes.

- [ ] **Step 11: Run test to verify it passes**

Run: `pnpm --dir frontend test`
Expected: PASS.

- [ ] **Step 12: Verify build + lint**

Run: `pnpm --dir frontend build && pnpm --dir frontend lint`
Expected: both green. (CI `frontend` job activates now — `package.json` exists.)

- [ ] **Step 13: Commit**

```sh
git add frontend/
git commit -m "feat: scaffold React frontend with vite, tailwind, vitest"
```

---

### Task 4: Tauri Shell (openmix-app)

**Files:**
- Create: `openmix-app/Cargo.toml`, `openmix-app/build.rs`, `openmix-app/tauri.conf.json`
- Create: `openmix-app/capabilities/default.json`
- Create: `openmix-app/src/main.rs`, `openmix-app/src/lib.rs`
- Create: `scripts/gen-icon.py`
- Generated: `openmix-app/icons/*`

**Interfaces:**
- Produces: crate `openmix-app` (bin + lib). `lib.rs` exports `pub fn run()`. Registers command `ping -> String` returning `"pong"`. `tauri.conf.json` productName `OpenMix AI`, identifier `ai.openmix.desktop`, devUrl `http://localhost:1420`.

- [ ] **Step 1: Write `openmix-app/Cargo.toml`**

```toml
[package]
name = "openmix-app"
version.workspace = true
edition.workspace = true
license.workspace = true
authors.workspace = true

[lib]
name = "openmix_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
openmix-core = { path = "../openmix-core" }
```

- [ ] **Step 2: Write `openmix-app/build.rs`** — `fn main() { tauri_build::build() }`

- [ ] **Step 3: Write `openmix-app/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OpenMix AI",
  "version": "0.1.0",
  "identifier": "ai.openmix.desktop",
  "build": {
    "beforeDevCommand": "pnpm --dir ../frontend dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "pnpm --dir ../frontend build",
    "frontendDist": "../frontend/dist"
  },
  "app": {
    "windows": [{ "title": "OpenMix AI", "width": 1280, "height": 800 }],
    "security": { "csp": null }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 4: Write `openmix-app/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "dialog:default"]
}
```

- [ ] **Step 5: Generate icons** — `scripts/gen-icon.py` (stdlib-only 1024x1024 solid PNG):

```python
import struct, zlib

W = H = 1024
color = (30, 41, 59)  # slate-800
raw = b"".join(b"\x00" + bytes(color) * W for _ in range(H))

def chunk(tag, data):
    c = tag + data
    return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xFFFFFFFF)

png = b"\x89PNG\r\n\x1a\n"
png += chunk(b"IHDR", struct.pack(">IIBBBBB", W, H, 8, 2, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(raw, 9))
png += chunk(b"IEND", b"")

with open("scripts/app-icon.png", "wb") as f:
    f.write(png)
print("wrote scripts/app-icon.png")
```

Run: `python3 scripts/gen-icon.py && pnpm --dir frontend exec tauri icon scripts/app-icon.png --output openmix-app/icons`
Expected: `openmix-app/icons/` populated (png set + icns + ico).

- [ ] **Step 6: Write `openmix-app/src/lib.rs`**

```rust
#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![ping])
        .run(tauri::generate_context!())
        .expect("error while running OpenMix AI");
}
```

- [ ] **Step 7: Write `openmix-app/src/main.rs`** — `fn main() { openmix_app_lib::run() }`

- [ ] **Step 8: Build**

Run: `cargo build -p openmix-app`
Expected: compiles (first build downloads ~400 crates; several minutes).

- [ ] **Step 9: Manual smoke run**

Run: `pnpm --dir frontend exec tauri dev` (window opens with "OpenMix AI" heading). Verify with the visible window, then quit.

- [ ] **Step 10: Commit**

```sh
git add openmix-app/ scripts/
git commit -m "feat: tauri 2 shell with ping command and icons"
```

---

### Task 5: Core — Chunked Decode + Waveform Peaks (TDD)

**Files:**
- Create: `openmix-core/src/audio/mod.rs`, `openmix-core/src/audio/decode.rs`, `openmix-core/src/audio/peaks.rs`
- Create: `scripts/gen-fixtures.py`, `scripts/gen-fixtures.sh`, `scripts/gen-fixtures.swift`
- Add: `openmix-core/tests/fixtures/sine1k_1s.wav`, `sine1k_1s.flac`, `sine1k_1s.mp3` (committed binaries)
- Test: `openmix-core/tests/audio_test.rs`

**Interfaces:**
- Produces (used by Task 7):
  - `pub struct AudioChunk { pub samples: Vec<f32>, pub frames: usize }` (interleaved)
  - `pub struct TrackMeta { pub title: Option<String>, pub artist: Option<String>, pub album: Option<String> }`
  - `pub struct DecodedStream` with `pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError>`, `pub fn next_chunk(&mut self, frames: usize) -> Result<Option<AudioChunk>, AppError>`, `pub fn sample_rate(&self) -> u32`, `pub fn channels(&self) -> u16`, `pub fn duration(&self) -> Duration`, `pub fn metadata(&self) -> &TrackMeta` — chunked iterator over a symphonia decoder, never materializing whole file.
  - `pub struct Peak { pub min: f32, pub max: f32 }` and `pub fn compute_peaks(stream: &mut DecodedStream, points: usize) -> Result<Vec<Peak>, AppError>` — min/max decimation; `Vec::with_capacity(points)`; chunked consumption.

- [x] **Step 1: Write the failing integration test** (`openmix-core/tests/audio_test.rs`)

```rust
use openmix_core::audio::{compute_peaks, DecodedStream};
use openmix_core::AppError;

fn write_sine_wav(path: &std::path::Path) -> std::io::Result<()> {
    let rate = 44100u32;
    let frames = rate as usize;
    let amp = 0.8f32;
    let mut pcm: Vec<i16> = Vec::with_capacity(frames);
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let s = (std::f32::consts::TAU * 1000.0 * t).sin() * amp;
        pcm.push((s * i16::MAX as f32) as i16);
    }
    let data_size = pcm.len() * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_size as u32).to_le_bytes());
    for s in &pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

#[test]
fn wav_decode_reports_correct_duration() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_sine_1s.wav");
    write_sine_wav(&path).unwrap();
    let stream = DecodedStream::open(&path)?;
    assert_eq!(stream.sample_rate(), 44100);
    assert_eq!(stream.channels(), 1);
    assert!((stream.duration().as_secs_f32() - 1.0).abs() < 0.05);
    Ok(())
}

#[test]
fn wav_chunks_sum_to_full_length() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_sine_1s.wav");
    write_sine_wav(&path).unwrap();
    let mut stream = DecodedStream::open(&path)?;
    let mut frames = 0usize;
    while let Some(chunk) = stream.next_chunk(4096)? {
        assert_eq!(chunk.samples.len() % chunk.frames, 0);
        assert!((1..=4096).contains(&chunk.frames));
        frames += chunk.frames;
    }
    assert!((44100..=44130).contains(&frames));
    Ok(())
}

#[test]
fn peaks_are_min_max_decimated() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_sine_1s.wav");
    write_sine_wav(&path).unwrap();
    let mut stream = DecodedStream::open(&path)?;
    let peaks = compute_peaks(&mut stream, 100)?;
    assert_eq!(peaks.len(), 100);
    for p in &peaks {
        assert!(p.min >= -1.0 && p.min <= 0.0, "min {}", p.min);
        assert!(p.max >= 0.0 && p.max <= 1.0, "max {}", p.max);
    }
    assert!(peaks.iter().any(|p| p.max > 0.7), "sine peaks reach ~0.8");
    Ok(())
}

#[test]
fn flac_and_mp3_fixtures_decode() -> Result<(), AppError> {
    for name in ["sine1k_1s.flac", "sine1k_1s.mp3"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures").join(name);
        let mut stream = DecodedStream::open(&path)?;
        let mut frames = 0usize;
        while let Some(chunk) = stream.next_chunk(8192)? {
            frames += chunk.frames;
        }
        assert!(frames > 40000, "{name}: decoded {frames} frames");
    }
    Ok(())
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-core --test audio_test`
Expected: FAIL — `module audio not found`.

- [x] **Step 3: Implement `openmix-core/src/audio/decode.rs`**

```rust
use std::path::{Path, PathBuf};
use std::time::Duration;
use openmix_core::error::AppError;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioChunk {
    pub samples: Vec<f32>,
    pub frames: usize,
}

#[derive(Debug, Default, Clone)]
pub struct TrackMeta {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
}

pub struct DecodedStream {
    path: PathBuf,
    sample_rate: u32,
    channels: u16,
    duration: Duration,
    meta: TrackMeta,
    decoder: Option<Box<dyn symphonia::core::codecs::Decoder>>,
    track_id: u32,
    sample_buf: SampleBuffer<f32>,
    format: Option<Box<dyn symphonia::core::formats::FormatReader>>,
}

impl DecodedStream {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, AppError> {
        let path = path.as_ref().to_path_buf();
        let file = std::fs::File::open(&path)
            .map_err(|e| AppError::OpenFile(path.clone(), e.to_string()))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|_| AppError::UnsupportedFormat(path.clone()))?;

        let track = probed
            .format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| AppError::UnsupportedFormat(path.clone()))?;
        let track_id = track.id;
        let params = &track.codec_params;
        let sample_rate = params.sample_rate.unwrap_or(44100);
        let channels = params.channels.map(|c| c.count() as u16).unwrap_or(2);
        let n_frames = params.n_frames.unwrap_or(0) as u64;
        let duration = if n_frames > 0 {
            Duration::from_secs_f64(n_frames as f64 / sample_rate as f64)
        } else {
            Duration::ZERO
        };

        let mut meta = TrackMeta::default();
        if let Some(tags) = probed.metadata.get().and_then(|m| m.tags()) {
            for tag in tags {
                let key = tag.key.to_ascii_lowercase();
                let val = tag.value.to_string();
                match key.as_str() {
                    "title" => meta.title = Some(val),
                    "artist" => meta.artist = Some(val),
                    "album" => meta.album = Some(val),
                    _ => {}
                }
            }
        }

        let mut format = Some(probed.format);
        let decoder = {
            let codec_params = format.as_ref().unwrap().tracks()[track_id as usize]
                .codec_params.clone();
            symphonia::default::get_codecs().make(&codec_params, &DecoderOptions::default()).ok()
        };
        let sample_buf = SampleBuffer::<f32>::new(0, params.sample_rate.unwrap_or(44100), params.channels.map(|c| c.count()).unwrap_or(2));

        Ok(Self {
            path,
            sample_rate,
            channels,
            duration,
            meta,
            decoder: decoder.map(Box::new),
            track_id,
            sample_buf,
            format,
        })
    }

    pub fn next_chunk(&mut self, frames: usize) -> Result<Option<AudioChunk>, AppError> {
        let format = self.format.as_mut().ok_or_else(|| AppError::Decode("stream exhausted".into()))?;
        loop {
            let packet = match format.next_packet() {
                Ok(p) => p,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    return Ok(None);
                }
                Err(e) => return Err(AppError::Decode(e.to_string())),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoder = self.decoder.as_mut()
                .ok_or_else(|| AppError::Decode("no decoder".into()))?;
            let decoded = decoder.decode(&packet)
                .map_err(|e| AppError::Decode(e.to_string()))?;
            let spec = *decoded.spec();
            self.sample_buf = SampleBuffer::new(frames, spec.rate, spec.channels.count());
            self.sample_buf.copy_interleaved_ref(decoded);
            let samples = self.sample_buf.samples().to_vec();
            let frame_count = samples.len() / spec.channels.count();
            return Ok(Some(AudioChunk { samples, frames: frame_count }));
        }
    }

    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn channels(&self) -> u16 { self.channels }
    pub fn duration(&self) -> Duration { self.duration }
    pub fn metadata(&self) -> &TrackMeta { &self.meta }
}
```

- [x] **Step 4: Implement `openmix-core/src/audio/peaks.rs`**

```rust
use openmix_core::error::AppError;
use super::decode::DecodedStream;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Peak {
    pub min: f32,
    pub max: f32,
}

pub fn compute_peaks(
    stream: &mut DecodedStream,
    points: usize,
) -> Result<Vec<Peak>, AppError> {
    let mut peaks: Vec<Peak> = Vec::with_capacity(points);
    let mut bucket: Vec<f32> = Vec::with_capacity(8192);
    let mut last_bucket = false;
    while let Some(chunk) = stream.next_chunk(8192)? {
        bucket.extend_from_slice(&chunk.samples);
        if bucket.len() >= points {
            let stride = bucket.len() / points;
            let mut p: Vec<Peak> = Vec::with_capacity(points);
            for i in 0..points {
                let slice = &bucket[i * stride..(i + 1) * stride];
                let min = slice.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                p.push(Peak { min, max });
            }
            peaks = p;
            last_bucket = true;
        }
    }
    if !last_bucket && !bucket.is_empty() {
        peaks.push(Peak { min: bucket.iter().cloned().fold(f32::INFINITY, f32::min), max: bucket.iter().cloned().fold(f32::NEG_INFINITY, f32::max) });
    }
    Ok(peaks)
}
```

- [x] **Step 5: Write `openmix-core/src/audio/mod.rs`** — `pub mod decode; pub mod peaks; pub use decode::{AudioChunk, DecodedStream, TrackMeta}; pub use peaks::{compute_peaks, Peak};` and add `pub mod audio;` to `lib.rs`.

- [x] **Step 6: Run tests to verify they pass**

Run: `cargo test -p openmix-core --test audio_test`
Expected: PASS (wav tests pass; fixture tests may fail until Step 7).

- [x] **Step 7: Generate and commit fixtures**

`scripts/gen-fixtures.py` (440 Hz sine, 44.1 kHz, 1 s, 16-bit mono → `sine1k_1s.wav`, stdlib only):

```python
import math, struct, wave

RATE = 44100
DUR = 1.0
FRAMES = int(RATE * DUR)
FREQ = 1000.0
AMP = 0.8

with wave.open("sine1k_1s.wav", "wb") as w:
    w.setnchannels(1)
    w.setsampwidth(2)
    w.setframerate(RATE)
    frames = bytearray()
    for i in range(FRAMES):
        s = AMP * math.sin(2 * math.pi * FREQ * i / RATE)
        frames += struct.pack("<h", int(s * 32767))
    w.writeframes(bytes(frames))
print("wrote sine1k_1s.wav")
```

Then `scripts/gen-fixtures.sh` (macOS-native, one-time):

```sh
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
python3 gen-fixtures.py
afconvert -f flac -d LEI16@44100 sine1k_1s.wav sine1k_1s.flac
swift gen-fixtures.swift   # writes sine1k_1s.mp3 via AudioToolbox (kAudioFormatMPEGLayer3, 192 kbps)
ls -la sine1k_1s.*
```

`scripts/gen-fixtures.swift` (AudioToolbox MP3 export):

```swift
import Foundation
import AudioToolbox

let src = URL(fileURLWithPath: "sine1k_1s.wav")
let dst = URL(fileURLWithPath: "sine1k_1s.mp3")
let clientFormat = AudioStreamBasicDescription(
    mSampleRate: 44100, mFormatID: kAudioFormatLinearPCM,
    mFormatFlags: kAudioFormatFlagIsSignedInteger,
    mBytesPerPacket: 2, mFramesPerPacket: 1, mBytesPerFrame: 2,
    mChannelsPerFrame: 1, mBitsPerChannel: 16, mReserved: 0)

var inF: ExtAudioFileRef?
guard ExtAudioFileOpenURL(src as CFURL, &inF) == noErr else { fatalError("open in") }
ExtAudioFileSetProperty(inF!, kExtAudioFileProperty_ClientDataFormat,
                        MemoryLayout<AudioStreamBasicDescription>.size, &clientFormat)

var outF: ExtAudioFileRef?
let fileDesc: [CFString: Any] = [
    kAudioFormatProperty_FormatID: kAudioFormatMPEGLayer3,
    kAudioFormatProperty_FormatFlags: kMPEGAudioFlagVBR,
    kAudioFilePropertyBitRateKey: 192_000,
]
let descData = try! JSONSerialization.data(withJSONObject: fileDesc)
guard ExtAudioFileCreateWithURL(
    dst as CFURL, kAudioFileMP3Type, descData as CFData, nil,
    AudioFileFlags.eraseFile.rawValue, &outF) == noErr else { fatalError("create out") }
ExtAudioFileSetProperty(outF!, kExtAudioFileProperty_ClientDataFormat,
                        MemoryLayout<AudioStreamBasicDescription>.size, &clientFormat)

var pcm = [UInt8](repeating: 0, count: Int(Double(RATE) * DUR * 2))
var buf = AudioBufferList(
    mNumberBuffers: 1,
    mBuffers: AudioBuffer(mNumberChannels: 1, mDataByteSize: UInt32(pcm.count), mData: &pcm))
var frames: UInt32 = UInt32(Double(RATE) * DUR)
while frames > 0 {
    var n = frames
    guard ExtAudioFileRead(inF!, &n, &buf) == noErr else { fatalError("read") }
    if n == 0 { break }
    ExtAudioFileWrite(outF!, n, &buf)
    frames -= n
}
ExtAudioFileDispose(inF!); ExtAudioFileDispose(outF!)
print("wrote sine1k_1s.mp3")
```

Run: `bash scripts/gen-fixtures.sh`, then move the three outputs to `openmix-core/tests/fixtures/`. **Fallback if the Swift MP3 encode is finicky:** `brew install lame && lame --preset standard sine1k_1s.wav sine1k_1s.mp3` (dev-only tool; fixtures are committed, so CI never runs this). Verify fixture decode tests pass, then commit.

- [x] **Step 8: Quality gates + commit**

Run: `cargo fmt --all --check && cargo clippy -p openmix-core --all-targets -- -D warnings && cargo test -p openmix-core`
Expected: all green.

```sh
git add openmix-core/ scripts/ openmix-core/tests/fixtures/
git commit -m "feat: chunked audio decode (symphonia) and waveform peaks with fixtures"
```

---

### Task 6: Storage Module in openmix-app (TDD)

**Files:**
- Create: `openmix-app/src/storage/mod.rs`, `openmix-app/src/storage/db.rs`, `openmix-app/src/storage/schema.sql`
- Test: `openmix-app/tests/storage_test.rs`

**Interfaces:**
- Produces (used by Task 7):
  - `pub struct Project { pub id: String, pub name: String, pub created_at: String, pub updated_at: String }`
  - `pub struct Track { pub id: String, pub project_id: Option<String>, pub path: String, pub title: String, pub artist: Option<String>, pub album: Option<String>, pub duration_ms: i64, pub sample_rate: u32, pub channels: u16, pub format: String, pub file_hash: String, pub peaks: Vec<openmix_core::audio::Peak>, pub created_at: String }`
  - `pub struct Storage { conn: std::sync::Mutex<rusqlite::Connection> }` with `pub fn open(path: &Path) -> Result<Self, StorageError>`, `pub fn open_in_memory() -> Result<Self, StorageError>`, `pub fn create_project(&self, name: &str) -> Result<Project, StorageError>`, `pub fn list_projects(&self) -> Result<Vec<Project>, StorageError>`, `pub fn delete_project(&self, id: &str) -> Result<(), StorageError>`, `pub fn insert_track(&self, t: &Track) -> Result<(), StorageError>`, `pub fn list_tracks(&self, project_id: Option<&str>) -> Result<Vec<Track>, StorageError>`, `pub fn get_pref(&self, key: &str) -> Result<Option<String>, StorageError>`, `pub fn set_pref(&self, key: &str, value: &str) -> Result<(), StorageError>`
  - `#[derive(Debug, thiserror::Error)] pub enum StorageError` (rusqlite + io variants)
  - Storage stamps `created_at`/`updated_at` on insert (ISO-8601-like via `SystemTime`).

- [x] **Step 1: Write the failing test** (`openmix-app/tests/storage_test.rs`)

```rust
use openmix_app::storage::{Storage, Track};

fn sample_track(project_id: Option<&str>) -> Track {
    Track {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.map(|s| s.to_string()),
        path: "/tmp/test.wav".into(),
        title: "Test Track".into(),
        artist: Some("Artist".into()),
        album: None,
        duration_ms: 1000,
        sample_rate: 44100,
        channels: 1,
        format: "wav".into(),
        file_hash: "abc".into(),
        peaks: vec![openmix_core::audio::Peak { min: -0.5, max: 0.5 }],
        created_at: "".into(),
    }
}

#[test]
fn project_crud_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("Mix 1").unwrap();
    assert!(!p.id.is_empty());
    let projects = storage.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Mix 1");
    storage.delete_project(&p.id).unwrap();
    assert!(storage.list_projects().unwrap().is_empty());
}

#[test]
fn track_insert_and_list() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("Mix 1").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Test Track");
    assert_eq!(tracks[0].peaks.len(), 1);
    assert!(storage.list_tracks(None).unwrap().is_empty());
}

#[test]
fn prefs_upsert() {
    let storage = Storage::open_in_memory().unwrap();
    storage.set_pref("theme", "dark").unwrap();
    assert_eq!(storage.get_pref("theme").unwrap(), Some("dark".into()));
    storage.set_pref("theme", "light").unwrap();
    assert_eq!(storage.get_pref("theme").unwrap(), Some("light".into()));
}

#[test]
fn migrations_are_idempotent() {
    let storage = Storage::open_in_memory().unwrap();
    let storage2 = Storage::open_in_memory().unwrap();
    storage.create_project("A").unwrap();
    storage2.create_project("B").unwrap();
    assert_eq!(storage.list_projects().unwrap().len(), 1);
    assert_eq!(storage2.list_projects().unwrap().len(), 1);
}
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-app --test storage_test`
Expected: FAIL — module `storage` not found.

- [x] **Step 3: Add deps to `openmix-app/Cargo.toml`** — `rusqlite = { version = "0.32", features = ["bundled"] }`, `uuid = { workspace = true }`, and `[dev-dependencies] tempfile = "3"`.

- [x] **Step 4: Write `openmix-app/src/storage/schema.sql`**

```sql
PRAGMA journal_mode = WAL;

CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tracks (
  id TEXT PRIMARY KEY,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  path TEXT NOT NULL UNIQUE,
  title TEXT NOT NULL,
  artist TEXT,
  album TEXT,
  duration_ms INTEGER NOT NULL,
  sample_rate INTEGER NOT NULL,
  channels INTEGER NOT NULL,
  format TEXT NOT NULL,
  file_hash TEXT NOT NULL,
  peaks TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS preferences (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
```

- [x] **Step 5: Write `openmix-app/src/storage/db.rs`** — `pub fn init(conn: &rusqlite::Connection) -> rusqlite::Result<()>` executing `include_str!("schema.sql")` via `conn.execute_batch`.

- [x] **Step 6: Write `openmix-app/src/storage/mod.rs`** implementing `StorageError`, `Project`, `Track` (derive `Serialize`/`Deserialize` on both), `Storage` (fields above) with all methods. Implementation notes: peaks stored as `serde_json::to_string(&t.peaks)`; rows read back with `serde_json::from_str`. `open_in_memory` uses `Connection::open_in_memory`. `open` uses `Connection::open(path)` + `db::init`. `created_at`/`updated_at` stamped on create/insert; `delete_project` cascades via FK.

- [x] **Step 7: Run tests to verify they pass**

Run: `cargo test -p openmix-app --test storage_test`
Expected: PASS (4 tests).

- [x] **Step 8: Quality gates + commit**

Run: `cargo fmt --all --check && cargo clippy -p openmix-app --all-targets -- -D warnings`
Expected: green.

```sh
git add openmix-app/
git commit -m "feat: sqlite storage module (projects, tracks, preferences) with migrations"
```

---

### Task 7: Import Pipeline + IPC Commands (TDD)

**Files:**
- Create: `openmix-app/src/commands/mod.rs`, `openmix-app/src/commands/tracks.rs`, `openmix-app/src/commands/projects.rs`
- Create: `openmix-app/src/import.rs`
- Modify: `openmix-app/src/lib.rs` (state + handlers)
- Modify: `openmix-app/Cargo.toml` (add `sha2`)
- Test: `openmix-app/tests/import_test.rs` + unit tests in `openmix-app/src/import.rs`

**Interfaces:**
- `pub fn hash_file(path: &Path) -> Result<String, String>` — **streamed** SHA-256, fixed 64 KiB buffer (`const HASH_BUF_SIZE: usize = 64 * 1024`), never loads whole file. **RAM bound: O(64 KiB), independent of file size (2-hour tracks included).**
- `pub fn import_file(storage: &Storage, path: &Path, project_id: Option<&str>) -> Result<TrackSummary, String>` — two sequential bounded passes: (1) streaming hash, (2) chunked decode + peaks.
- `TrackSummary { id, path, title, artist, album, duration_ms, sample_rate, channels, format, peaks }` (derive `serde::Serialize`).
- Tauri commands: `list_projects() -> Vec<Project>`, `create_project(name: String) -> Project`, `delete_project(id: String)`, `list_tracks(project_id: Option<String>) -> Vec<TrackSummary>`, `import_tracks(paths: Vec<String>, project_id: Option<String>) -> Vec<TrackSummary>` (all `Result<_, String>`).
- `pub struct AppState { pub storage: Mutex<Storage> }` managed in lib.rs; DB path resolved from `app.path().app_data_dir()` in `setup`; storage opened at startup.

- [x] **Step 1: Write the failing integration test** (`openmix-app/tests/import_test.rs`)

```rust
use openmix_app::import::import_file;
use openmix_app::storage::Storage;

#[test]
fn import_wav_persists_track_with_peaks() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("import_me.wav");
    std::fs::write(&wav, SINE_WAV_BYTES).unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();

    let summary = import_file(&storage, &wav, Some(&p.id)).unwrap();

    assert_eq!(summary.title, "import_me");
    assert_eq!(summary.format, "wav");
    assert_eq!(summary.sample_rate, 44100);
    assert_eq!(summary.peaks.len(), 2000);
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].file_hash.len(), 64);
    assert!(tracks[0].file_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn import_streamed_hash_matches_known_digest() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("hash_me.wav");
    std::fs::write(&wav, SINE_WAV_BYTES).unwrap();
    let storage = Storage::open_in_memory().unwrap();

    let summary = import_file(&storage, &wav, None).unwrap();

    let mut h = Sha256::new();
    h.update(SINE_WAV_BYTES);
    let expected: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(summary.file_hash, expected, "streamed hash diverged from reference digest");
}

#[test]
fn import_hash_changes_when_content_changes() {
    let dir = tempfile::tempdir().unwrap();
    let wav_a = dir.path().join("a.wav");
    let wav_b = dir.path().join("b.wav");
    std::fs::write(&wav_a, SINE_WAV_BYTES).unwrap();
    let mut tampered = SINE_WAV_BYTES.to_vec();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    std::fs::write(&wav_b, &tampered).unwrap();
    let storage = Storage::open_in_memory().unwrap();

    let summary_a = import_file(&storage, &wav_a, None).unwrap();
    let summary_b = import_file(&storage, &wav_b, None).unwrap();

    assert_eq!(summary_a.file_hash, summary_b.file_hash, "different content produced the same hash");
    assert_eq!(summary_a.file_hash.len(), 64);
}

#[test]
fn import_unsupported_format_errors() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.txt");
    std::fs::write(&bad, "not audio").unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let err = import_file(&storage, &bad, None).unwrap_err();
    assert!(err.contains("unsupported") || err.contains("decode"), "got: {err}");
}

const SINE_WAV_BYTES: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../openmix-core/tests/fixtures/sine1k_1s.wav"));
```

- [x] **Step 2: Run test to verify it fails**

Run: `cargo test -p openmix-app --test import_test`
Expected: FAIL — module `import` not found.

- [x] **Step 3: Write `openmix-app/src/import.rs`** — streaming hash, bounded buffer:

```rust
use std::io::Read;
use std::path::Path;
use openmix_core::audio::{compute_peaks, DecodedStream};
use openmix_core::AppError;
use crate::storage::{Storage, Track};

const PEAK_POINTS: usize = 2000;
const HASH_BUF_SIZE: usize = 64 * 1024;

#[derive(serde::Serialize)]
pub struct TrackSummary {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: String,
    pub peaks: Vec<openmix_core::audio::Peak>,
}

/// Streaming SHA-256 over the raw file bytes using a fixed-size buffer.
/// Memory stays O(HASH_BUF_SIZE) regardless of file length, so 2-hour
/// tracks are never loaded into RAM for fingerprinting.
pub fn hash_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; HASH_BUF_SIZE];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("failed to hash {}: {e}", path.display())),
        }
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn import_file(
    storage: &Storage,
    path: &Path,
    project_id: Option<&str>,
) -> Result<TrackSummary, String> {
    let file_hash = hash_file(path)?;
    let format = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let title = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let mut stream = DecodedStream::open(path).map_err(|e: AppError| e.to_string())?;
    let peaks = compute_peaks(&mut stream, PEAK_POINTS).map_err(|e: AppError| e.to_string())?;
    let meta = stream.metadata().clone();

    let track = Track {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.map(|s| s.to_string()),
        path: path.display().to_string(),
        title: meta.title.unwrap_or(title),
        artist: meta.artist,
        album: meta.album,
        duration_ms: stream.duration().as_millis() as i64,
        sample_rate: stream.sample_rate(),
        channels: stream.channels(),
        format,
        file_hash,
        peaks: peaks.clone(),
        created_at: "".into(),
    };
    storage.insert_track(&track).map_err(|e| e.to_string())?;
    Ok(TrackSummary {
        id: track.id,
        path: track.path,
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration_ms: track.duration_ms,
        sample_rate: track.sample_rate,
        channels: track.channels,
        format: track.format,
        peaks,
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_file, HASH_BUF_SIZE};
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn hash_buf_size_is_bounded() {
        assert_eq!(HASH_BUF_SIZE, 64 * 1024);
    }

    #[test]
    fn hash_file_missing_path_errors() {
        let err = hash_file(Path::new("/nonexistent/nope.wav")).unwrap_err();
        assert!(err.contains("failed to open"), "got: {err}");
    }

    #[test]
    fn hash_file_matches_in_memory_digest() {
        use sha2::{Digest, Sha256};
        let bytes = b"openmix hash probe bytes";
        let dir = std::env::temp_dir();
        let path = dir.join("openmix_hash_probe.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(bytes).unwrap();

        let mut h = Sha256::new();
        h.update(bytes);
        let expected: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();

        assert_eq!(hash_file(&path).unwrap(), expected);
        std::fs::remove_file(&path).ok();
    }
}
```

- [x] **Step 4: Run unit tests** — `cargo test -p openmix-app --lib` — Expected: PASS (3 unit tests).

- [x] **Step 5: Run integration test to verify it passes**

Run: `cargo test -p openmix-app --test import_test`
Expected: PASS (4 tests: persistence, known digest, tamper, unsupported).

- [x] **Step 6: Write command modules** — `openmix-app/src/commands/mod.rs` (`pub mod projects; pub mod tracks;`), `commands/projects.rs` with `list_projects`/`create_project`/`delete_project`, `commands/tracks.rs` with `list_tracks`/`import_tracks`, all delegating to storage/import via `State<'_, AppState>`.

- [x] **Step 7: Update `openmix-app/src/lib.rs`** — `mod storage; mod import; mod commands;`; `pub struct AppState { pub storage: Mutex<crate::storage::Storage> }`; in `run()`: open storage in `setup` from `app.path().app_data_dir()` (`std::fs::create_dir_all` the dir, then `Storage::open`), `manage(AppState { storage: Mutex::new(storage) })`, register all five commands; add `sha2 = "0.10"` and `uuid = { workspace = true }` to `openmix-app/Cargo.toml`.

- [x] **Step 8: Verify** — `cargo test -p openmix-app` (lib + integration: all pass) and `cargo build -p openmix-app` compiles.

- [x] **Step 9: Quality gates + commit**

Run: `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: green.

```sh
git add openmix-app/
git commit -m "feat: import pipeline with streamed sha-256 hashing and IPC commands"
```

---

### Task 8: Frontend Data Layer + Home Screen (TDD)

**Files:**
- Create: `frontend/src/types.ts`, `frontend/src/api/ipc.ts`
- Create: `frontend/src/store/projects.ts`, `frontend/src/store/tracks.ts`
- Create: `frontend/src/pages/Home.tsx`
- Modify: `frontend/src/App.tsx` (render `Home`)
- Test: `frontend/src/store/projects.test.ts`, `frontend/src/pages/Home.test.tsx`

**Interfaces:**
- Consumes: Task 7 command names and `TrackSummary`/`Project` shapes.
- Produces: `src/types.ts` (TS mirrors of `Project`, `TrackSummary`, `Peak`); `src/api/ipc.ts` (`listProjects()`, `createProject(name)`, `deleteProject(id)`, `listTracks(projectId)`, `importTracks(paths, projectId)`); Zustand stores `useProjectsStore` (`projects`, `load()`, `create(name)`, `remove(id)`, `selectedId`, `select(id)`) and `useTracksStore` (`tracks`, `load(projectId)`, `addFromPaths(paths, projectId)`).

- [x] **Step 1: Write the failing store test** (`frontend/src/store/projects.test.ts`)

```ts
import { describe, expect, it, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { useProjectsStore } from './projects';

describe('useProjectsStore', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    useProjectsStore.setState({ projects: [], selectedId: null, loading: false });
  });

  it('loads projects from IPC', async () => {
    vi.mocked(invoke).mockResolvedValue([{ id: 'p1', name: 'Mix 1', created_at: '', updated_at: '' }]);
    await useProjectsStore.getState().load();
    expect(invoke).toHaveBeenCalledWith('list_projects');
    expect(useProjectsStore.getState().projects).toHaveLength(1);
  });

  it('creates a project via IPC', async () => {
    vi.mocked(invoke).mockResolvedValue({ id: 'p2', name: 'New', created_at: '', updated_at: '' });
    await useProjectsStore.getState().create('New');
    expect(invoke).toHaveBeenCalledWith('create_project', { name: 'New' });
    expect(useProjectsStore.getState().projects[0].name).toBe('New');
  });
});
```

- [x] **Step 2: Run test to verify it fails**

Run: `pnpm --dir frontend test`
Expected: FAIL — module `./projects` missing.

- [x] **Step 3: Write `frontend/src/types.ts`**

```ts
export interface Peak { min: number; max: number }
export interface Project { id: string; name: string; created_at: string; updated_at: string }
export interface TrackSummary {
  id: string; path: string; title: string; artist: string | null; album: string | null;
  duration_ms: number; sample_rate: number; channels: number; format: string; peaks: Peak[];
}
```

- [x] **Step 4: Write `frontend/src/api/ipc.ts`**

```ts
import { invoke } from '@tauri-apps/api/core';
import type { Project, TrackSummary } from '../types';

export const listProjects = () => invoke<Project[]>('list_projects');
export const createProject = (name: string) => invoke<Project>('create_project', { name });
export const deleteProject = (id: string) => invoke<void>('delete_project', { id });
export const listTracks = (projectId: string | null) =>
  invoke<TrackSummary[]>('list_tracks', { projectId });
export const importTracks = (paths: string[], projectId: string | null) =>
  invoke<TrackSummary[]>('import_tracks', { paths, projectId });
```

(Note: Tauri converts camelCase JS args to snake_case Rust params automatically; `projectId` ↔ `project_id`.)

- [x] **Step 5: Write `frontend/src/store/projects.ts` and `frontend/src/store/tracks.ts`** (zustand; `create` appends server result, `remove` filters, `load` fetches; tracks store `addFromPaths` calls `importTracks` and appends results).

- [x] **Step 6: Run test to verify it passes**

Run: `pnpm --dir frontend test`
Expected: PASS.

- [x] **Step 7: Write the failing Home test** (`frontend/src/pages/Home.test.tsx`)

```tsx
import { describe, expect, it, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import Home from './Home';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn() }));

import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { useProjectsStore } from '../store/projects';

describe('Home', () => {
  beforeEach(() => {
    vi.mocked(invoke).mockReset();
    vi.mocked(open).mockReset();
    useProjectsStore.setState({ projects: [], selectedId: null, loading: false });
  });

  it('renders projects from the store', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects') return Promise.resolve([{ id: 'p1', name: 'Mix 1', created_at: '', updated_at: '' }]);
      if (cmd === 'list_tracks') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<Home />);
    expect(await screen.findByText('Mix 1')).toBeInTheDocument();
  });

  it('creates a project on submit', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects') return Promise.resolve([]);
      if (cmd === 'create_project') return Promise.resolve({ id: 'p2', name: 'New Mix', created_at: '', updated_at: '' });
      if (cmd === 'list_tracks') return Promise.resolve([]);
      return Promise.resolve([]);
    });
    render(<Home />);
    const input = await screen.findByPlaceholderText('Project name');
    fireEvent.change(input, { target: { value: 'New Mix' } });
    fireEvent.click(screen.getByRole('button', { name: /create/i }));
    expect(await screen.findByText('New Mix')).toBeInTheDocument();
  });

  it('imports tracks via file dialog', async () => {
    vi.mocked(invoke).mockImplementation((cmd: string) => {
      if (cmd === 'list_projects') return Promise.resolve([{ id: 'p1', name: 'Mix 1', created_at: '', updated_at: '' }]);
      if (cmd === 'list_tracks') return Promise.resolve([]);
      if (cmd === 'import_tracks') return Promise.resolve([{
        id: 't1', path: '/a.wav', title: 'Imported', artist: null, album: null,
        duration_ms: 1000, sample_rate: 44100, channels: 1, format: 'wav',
        peaks: [{ min: -0.5, max: 0.5 }],
      }]);
      return Promise.resolve([]);
    });
    vi.mocked(open).mockResolvedValue(['/a.wav'] as never);
    render(<Home />);
    fireEvent.click(await screen.findByRole('button', { name: /import/i }));
    expect(await screen.findByText('Imported')).toBeInTheDocument();
  });
});
```

- [x] **Step 8: Run test to verify it fails**

Run: `pnpm --dir frontend test`
Expected: FAIL — `Home` not found.

- [x] **Step 9: Write `frontend/src/pages/Home.tsx`**

Layout: header "OpenMix AI"; left column = project list (create input + button, delete per row, click to select); right column = tracks pane: "Import" button → `open({ multiple: true, filters: [{ name: 'Audio', extensions: ['mp3', 'wav', 'flac'] }] })` → `addFromPaths`; track list rows (title, artist, duration). Select first project on load and `load()` tracks via `useEffect`. Tailwind for minimal styling.

- [x] **Step 10: Run test to verify it passes**

Run: `pnpm --dir frontend test`
Expected: PASS (3 tests).

- [x] **Step 11: Verify build + lint**

Run: `pnpm --dir frontend build && pnpm --dir frontend lint`
Expected: green.

- [x] **Step 12: Commit**

```sh
git add frontend/src/
git commit -m "feat: home screen with project CRUD and track import"
```

---

### Task 9: Waveform Canvas + Track Card (TDD)

**Files:**
- Create: `frontend/src/components/waveform/drawPeaks.ts`, `frontend/src/components/waveform/WaveformCanvas.tsx`
- Create: `frontend/src/components/library/TrackCard.tsx`
- Modify: `frontend/src/pages/Home.tsx` (render `TrackCard` with `WaveformCanvas`)
- Test: `frontend/src/components/waveform/drawPeaks.test.ts`, `frontend/src/components/library/TrackCard.test.tsx`

**Interfaces:**
- Produces: `drawPeaks(ctx: CanvasRenderingContext2D, peaks: Peak[], width: number, height: number, color: string)` — pure, testable with a stub context; `WaveformCanvas({ peaks, className })` — canvas element, `useEffect` redraw on peaks change, `devicePixelRatio` scaling; `TrackCard({ track })` — renders title/artist/duration/format + waveform.

- [x] **Step 1: Write the failing drawPeaks test**

```ts
import { describe, expect, it, vi } from 'vitest';
import { drawPeaks } from './drawPeaks';
import type { Peak } from '../../types';

function stubCtx() {
  const calls: string[] = [];
  return {
    calls,
    beginPath: vi.fn(() => calls.push('beginPath')),
    moveTo: vi.fn(() => calls.push('moveTo')),
    lineTo: vi.fn(() => calls.push('lineTo')),
    stroke: vi.fn(() => calls.push('stroke')),
  } as unknown as CanvasRenderingContext2D;
}

describe('drawPeaks', () => {
  it('draws one vertical line per peak', () => {
    const ctx = stubCtx();
    const peaks: Peak[] = [
      { min: -0.5, max: 0.5 },
      { min: -1, max: 1 },
    ];
    drawPeaks(ctx, peaks, 100, 100, '#fff');
    const lineToCalls = (ctx as unknown as { calls: string[] }).calls.filter((c) => c === 'lineTo');
    expect(lineToCalls).toHaveLength(2);
  });

  it('is a no-op for empty peaks', () => {
    const ctx = stubCtx();
    drawPeaks(ctx, [], 100, 100, '#fff');
    expect((ctx as unknown as { calls: string[] }).calls).toEqual([]);
  });
});
```

- [x] **Step 2: Run test to verify it fails**

Run: `pnpm --dir frontend test`
Expected: FAIL — `drawPeaks` not found.

- [x] **Step 3: Write `frontend/src/components/waveform/drawPeaks.ts`**

```ts
import type { Peak } from '../../types';

export function drawPeaks(
  ctx: CanvasRenderingContext2D,
  peaks: Peak[],
  width: number,
  height: number,
  color: string,
): void {
  if (peaks.length === 0) return;
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.beginPath();
  const mid = height / 2;
  const step = width / peaks.length;
  peaks.forEach((p, i) => {
    const x = i * step;
    const yMin = mid + p.min * mid;
    const yMax = mid + p.max * mid;
    ctx.moveTo(x, yMin);
    ctx.lineTo(x, yMax);
  });
  ctx.stroke();
}
```

- [x] **Step 4: Run test to verify it passes**

Run: `pnpm --dir frontend test`
Expected: PASS.

- [x] **Step 5: Write the failing TrackCard test**

```tsx
import { describe, expect, it } from 'vitest';
import { render, screen } from '@testing-library/react';
import { TrackCard } from './TrackCard';
import type { TrackSummary } from '../../types';

const track: TrackSummary = {
  id: 't1', path: '/a.mp3', title: 'Neon Nights', artist: 'DJ Test', album: null,
  duration_ms: 210_000, sample_rate: 44100, channels: 2, format: 'mp3',
  peaks: [{ min: -0.5, max: 0.5 }],
};

describe('TrackCard', () => {
  it('renders title, artist and duration', () => {
    render(<TrackCard track={track} />);
    expect(screen.getByText('Neon Nights')).toBeInTheDocument();
    expect(screen.getByText('DJ Test')).toBeInTheDocument();
    expect(screen.getByText('3:30')).toBeInTheDocument();
    expect(screen.getByText('mp3')).toBeInTheDocument();
  });
});
```

- [x] **Step 6: Run test to verify it fails**

Run: `pnpm --dir frontend test`
Expected: FAIL — `TrackCard` not found.

- [x] **Step 7: Implement `TrackCard.tsx`** (title, artist fallback "Unknown artist", `formatDuration(ms)` → `m:ss`, format badge) and `WaveformCanvas.tsx` (canvas with `data-testid="waveform"`, redraw via `drawPeaks`, scaled by `devicePixelRatio`).

- [x] **Step 8: Run test to verify it passes** — `pnpm --dir frontend test` → PASS.

- [x] **Step 9: Wire into `Home.tsx`** — replace the plain track row with `<TrackCard>` including `<WaveformCanvas peaks={track.peaks} />`.

- [x] **Step 10: Quality gates + commit**

Run: `pnpm --dir frontend test && pnpm --dir frontend lint && pnpm --dir frontend build`
Expected: green.

```sh
git add frontend/src/
git commit -m "feat: canvas waveform and track cards"
```

---

### Task 10: E2E (Mocked IPC), Docs, Gate 1

**Files:**
- Create: `frontend/playwright.config.ts`, `frontend/e2e/home.spec.ts`, `frontend/src/e2e/mocks.ts`
- Modify: `frontend/src/main.tsx` (conditional mock install), `frontend/package.json` (e2e script exists already), `.github/workflows/ci.yml` (e2e job), `.gitignore`, `README.md`, `docs/build-guide.md`

**Interfaces:** n/a — integration task.

- [x] **Step 1: Write `frontend/src/e2e/mocks.ts`** — installs `mockIPC` handler backed by in-memory fake data:

```ts
import { mockIPC } from '@tauri-apps/api/mocks';
import type { Project, TrackSummary } from '../types';

export function installMocks() {
  const projects: Project[] = [{ id: 'p1', name: 'Demo Mix', created_at: '', updated_at: '' }];
  const tracks: TrackSummary[] = [{
    id: 't1', path: '/demo.mp3', title: 'E2E Track', artist: 'E2E Artist', album: null,
    duration_ms: 90_000, sample_rate: 44100, channels: 2, format: 'mp3',
    peaks: Array.from({ length: 200 }, (_, i) => ({
      min: -Math.abs(Math.sin(i / 10)), max: Math.abs(Math.sin(i / 10)),
    })),
  }];

  mockIPC((cmd, args) => {
    switch (cmd) {
      case 'list_projects': return Promise.resolve(projects);
      case 'create_project':
        return Promise.resolve({ id: 'p2', name: (args as { name: string }).name, created_at: '', updated_at: '' });
      case 'delete_project': return Promise.resolve(undefined);
      case 'list_tracks': return Promise.resolve((args as { projectId: string | null }).projectId === null ? [] : tracks);
      case 'import_tracks': return Promise.resolve(tracks);
      default: return Promise.resolve(undefined);
    }
  });
}
```

- [x] **Step 2: Modify `frontend/src/main.tsx`** — before rendering: `if (import.meta.env.VITE_E2E === 'true') { const { installMocks } = await import('./e2e/mocks'); installMocks(); }`

- [x] **Step 3: Write `frontend/playwright.config.ts`**

```ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './e2e',
  timeout: 30_000,
  use: { baseURL: 'http://localhost:1420' },
  webServer: {
    command: 'pnpm dev',
    url: 'http://localhost:1420',
    reuseExistingServer: !process.env.CI,
    env: { VITE_E2E: 'true' },
  },
  projects: [{ name: 'chromium', use: { browserName: 'chromium' } }],
});
```

- [x] **Step 4: Write `frontend/e2e/home.spec.ts`**

```ts
import { expect, test } from '@playwright/test';

test('create a project and see it listed', async ({ page }) => {
  await page.goto('/');
  await page.getByPlaceholder('Project name').fill('CI Mix');
  await page.getByRole('button', { name: /create/i }).click();
  await expect(page.getByText('CI Mix')).toBeVisible();
});

test('import shows a track with waveform', async ({ page }) => {
  await page.goto('/');
  await page.getByRole('button', { name: /import/i }).click();
  await expect(page.getByText('E2E Track')).toBeVisible();
  await expect(page.getByTestId('waveform')).toBeVisible();
});
```

- [x] **Step 5: Run E2E locally**

Run: `pnpm --dir frontend exec playwright install chromium && pnpm --dir frontend e2e`
Expected: 2 passing tests.

- [x] **Step 6: Add CI e2e job to `.github/workflows/ci.yml`**

```yaml
  e2e:
    name: E2E (mocked IPC)
    runs-on: ubuntu-latest
    if: ${{ hashFiles('frontend/package.json') != '' }}
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 22 }
      - uses: pnpm/action-setup@v4
        with: { version: 9 }
      - run: pnpm install --frozen-lockfile
        working-directory: frontend
      - run: pnpm exec playwright install --with-deps chromium
        working-directory: frontend
      - run: pnpm e2e
        working-directory: frontend
```

- [x] **Step 7: Update `.gitignore`** — add `frontend/.vite/`, `frontend/coverage/`, `frontend/playwright-report/`, `frontend/test-results/`.

- [x] **Step 8: Update docs** — `README.md`: status → "Phase 1 complete — foundation: imports MP3/WAV/FLAC, waveforms, project management (local SQLite)"; `docs/build-guide.md`: pnpm install via `npm install -g pnpm@9`, Tauri CLI via `@tauri-apps/cli` devDependency (`pnpm exec tauri dev`), E2E section (mocked-IPC Playwright; note `tauri-driver` is Windows/Linux-only, real-webview E2E lands in Phase 5).

- [x] **Step 9: Full verification**

Run (repo root):
`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo check --workspace --no-default-features`
`pnpm --dir frontend lint && pnpm --dir frontend test && pnpm --dir frontend build && pnpm --dir frontend e2e`
Expected: all green.

- [x] **Step 10: Manual smoke test (Gate 1 checklist)** — `pnpm exec tauri dev`:
  1. Window opens titled "OpenMix AI"
  2. Create project "Test Mix" → appears in list; restart app → persists
  3. Import a real MP3, WAV, and FLAC via dialog → all show title/artist/duration + waveform
  4. Delete project → tracks gone; preferences empty (no rows)
  5. Quit; relaunch; data intact
  Record results in the Gate 1 report.

- [x] **Step 11: Commit**

```sh
git add frontend/ .github/ .gitignore README.md docs/build-guide.md
git commit -m "feat: e2e with mocked IPC, ci e2e job, docs for phase 1"
git push
```

- [x] **Step 12: Gate 1 review** — report results (all tests, smoke checklist, memory note: decode is chunked; peaks bounded at 2000 points). **Stop and wait for user review before Phase 2.**

---

## Self-Review Notes

- **Spec coverage:** Phase 1 deliverables all mapped — Tauri setup (T4), React UI (T3/T8/T9), Rust integration (T7), SQLite storage (T6), audio import MP3/WAV/FLAC + metadata + waveform (T5/T7), project CRUD (T6/T8). Gate 1 = T10.
- **Storage in app crate only:** `rusqlite` appears only in `openmix-app/Cargo.toml`; core has no persistence.
- **RAM safety:** decode chunks (T5), peaks buckets (T5), SHA-256 over 64 KiB buffer (T7) — no whole-file reads anywhere. Two sequential bounded passes in `import_file` (hash, then decode).
- **Type consistency:** `TrackSummary` shape identical across Rust (T7), TS (T8), E2E mock (T10); camelCase↔snake_case IPC conversion noted; `project_id`/`projectId` consistent; `peaks: Vec<Peak>`/`Peak[]` consistent.
- **CI:** `core`/`frontend` jobs activate at T2/T3; e2e job added T10; `--no-default-features` gate active from T2.
- **Known deviation:** architecture's "Playwright E2E against packaged app" is deferred to Phase 5 (tauri-driver unsupported on macOS); Phase 1 uses the officially documented mocked-IPC pattern, explicitly documented in build-guide.
- **No placeholders:** every code step is complete; MP3 fixture generation has a documented fallback (lame via brew, dev-only).