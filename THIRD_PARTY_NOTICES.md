# OpenMix AI — Third-Party Notices

OpenMix AI (MIT) links against the following open-source libraries. Their
licenses require attribution; the full texts are available at the linked
URLs.

## Analysis (GPL-3.0)

- **aubio** — https://aubio.org — audio analysis (tempo/onsets/beats),
  compiled from source via the `aubio-rs` `builtin` feature. GPL-3.0:
  https://www.gnu.org/licenses/gpl-3.0.txt
- **libkeyfinder** (mixxxdj/libkeyfinder) — musical key detection, vendored
  into `openmix-core/src/keyfinder/vendor/` at tag v2.2.6
  (commit `a409c7447e9f440a12627ff4a540a43e41b48a55`). GPL-3.0-or-later;
  see `openmix-core/src/keyfinder/vendor/NOTICE` for details, provenance,
  and the single modification made (FFTW3 replacement).

Both are optional: they are enabled only by the `native-analysis` Cargo
feature (default on). `cargo check --workspace --no-default-features`
builds the pure-Rust fallback path (autocorrelation tempo, spectral-flux
onsets, histogram beats, Krumhansl–Schmuckler key) with no GPL code.

## Encoding (LGPL)

- **LAME** — https://lame.sourceforge.io — MP3 encoding (Phase 4). LGPL-2.0:
  https://www.gnu.org/licenses/old-licenses/lgpl-2.0.html
