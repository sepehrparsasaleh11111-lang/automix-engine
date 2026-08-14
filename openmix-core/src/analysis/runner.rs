//! Single-pass analysis orchestrator.
//!
//! Feeds aubio streaming detectors (Tempo, Onset) hop-by-hop as the
//! `DecodedStream` is drained, accumulating only bounded buffers:
//! - `mono_acc`: last 60 s of feed-rate mono (energy + fallback detectors)
//! - `key_buf`: first `key_max_seconds` at `key_rate`
//!
//! Cancel is checked before every chunk and once after the loop.

use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "native-analysis")]
use aubio_rs::{Onset, OnsetMode, Tempo};

use crate::audio::decode::DecodedStream;
use crate::audio::mono::to_mono;
use crate::beatgrid::{correct, fit_uniform};
use crate::error::AppError;

use super::beats::label_beats;
use super::energy::{energy_windows, peak_db_of, rms_db_of, AnalysisConfig, AnalysisResult};
use super::key::best_key;
#[cfg(not(feature = "native-analysis"))]
use super::onsets;
use super::{chroma, tempo};

/// Full-rate mono retained for energy analysis and the non-aubio fallback
/// detectors (bounds single-pass memory).
const MONO_CAP_SECS: usize = 60;
/// Onset-stream hop when it must differ from the tempo hop (native only;
/// see the measured 512-vs-256 decision in the Task 12 report).
#[cfg(feature = "native-analysis")]
const ONSET_HOP: usize = 512;

/// Run the full analysis pipeline over an already-open stream.
///
/// Single bounded-memory pass: tempo/onset streams are fed hop-by-hop and
/// only the capped key buffer and the last 60 s of mono are retained.
/// `cancel` is polled before every chunk; a set flag aborts with
/// `AppError::Other("cancelled")`.
pub fn analyze(
    stream: &mut DecodedStream,
    cfg: &AnalysisConfig,
    cancel: &AtomicBool,
) -> Result<AnalysisResult, AppError> {
    if cfg.tempo_hop == 0 || cfg.key_rate == 0 || cfg.energy_window_ms == 0 {
        return Err(AppError::Analysis(
            "invalid analysis config (zero hop/rate/window)".into(),
        ));
    }
    let rate = stream.sample_rate();
    #[cfg(feature = "native-analysis")]
    let tempo_hop = cfg.tempo_hop;
    #[cfg(feature = "native-analysis")]
    let onset_hop = ONSET_HOP;
    let key_cap =
        (cfg.key_max_seconds.unwrap_or(MONO_CAP_SECS as f64) * cfg.key_rate as f64) as usize;
    let energy_cap = MONO_CAP_SECS * rate as usize;

    #[cfg(feature = "native-analysis")]
    let mut tempo_holder = Tempo::new(OnsetMode::SpecFlux, 1024, tempo_hop, rate).ok();
    #[cfg(feature = "native-analysis")]
    let mut onset_holder = Onset::new(OnsetMode::SpecFlux, 1024, onset_hop, rate).ok();

    let mut key_buf: Vec<f32> = Vec::new();
    let mut mono_acc: Vec<f32> = Vec::new();
    #[cfg(feature = "native-analysis")]
    let mut tempo_buf: Vec<f32> = Vec::new();
    #[cfg(feature = "native-analysis")]
    let mut onset_buf: Vec<f32> = Vec::new();
    #[cfg(feature = "native-analysis")]
    let mut onset_times: Vec<f64> = Vec::new();

    while let Some(chunk) = stream.next_chunk(8192)? {
        if cancel.load(Ordering::Relaxed) {
            return Err(AppError::Other("cancelled".into()));
        }
        let mono = to_mono(&chunk.samples, stream.channels(), rate, rate);
        mono_acc.extend_from_slice(&mono);
        if mono_acc.len() > energy_cap {
            let drop = mono_acc.len() - energy_cap;
            mono_acc.copy_within(drop.., 0);
            mono_acc.truncate(energy_cap);
        }
        #[cfg(feature = "native-analysis")]
        {
            tempo_buf.extend_from_slice(&mono);
            onset_buf.extend_from_slice(&mono);
            if let Some(t) = tempo_holder.as_mut() {
                feed_tempo(t, &mut tempo_buf, tempo_hop);
            } else {
                tempo_buf.clear();
            }
            if let Some(o) = onset_holder.as_mut() {
                feed_onset(o, &mut onset_buf, onset_hop, &mut onset_times);
            } else {
                onset_buf.clear();
            }
        }
        let key_mono = to_mono(&chunk.samples, stream.channels(), rate, cfg.key_rate);
        if key_buf.len() < key_cap {
            let room = key_cap - key_buf.len();
            key_buf.extend_from_slice(&key_mono[..room.min(key_mono.len())]);
        }
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(AppError::Other("cancelled".into()));
    }

    // BPM: streaming aubio Tempo (native) with an autocorrelation fallback
    // on the capped mono buffer; pure autocorrelation without aubio.
    #[cfg(feature = "native-analysis")]
    let (bpm, bpm_confidence) = extract_bpm(tempo_holder.as_ref(), &mono_acc, rate);
    #[cfg(not(feature = "native-analysis"))]
    let (bpm, bpm_confidence) = (tempo::autocorr_bpm(&mono_acc, rate), None);

    // Onset/beat times: streaming onset stream (native, matches the
    // approved `aubio_onsets` output) or the pure-Rust flux/histogram
    // detectors over the capped mono buffer.
    #[cfg(not(feature = "native-analysis"))]
    let onset_times = onsets::flux_onsets(&mono_acc, rate);
    #[cfg(not(feature = "native-analysis"))]
    let beat_times: Vec<f64> = super::beats::histogram_beats(&mono_acc, rate)
        .into_iter()
        .map(|b| b.position_sec)
        .collect();
    #[cfg(feature = "native-analysis")]
    let beat_times = onset_times.clone();

    let grid = if beat_times.len() >= 2 {
        let g = fit_uniform(&beat_times, 0.5);
        Some(correct(g, &beat_times, 50.0))
    } else {
        None
    };
    let beats = label_beats(beat_times);

    // Key: KeyFinder (native) + Krumhansl–Schmuckler, best-of dispatch.
    #[cfg(feature = "native-analysis")]
    let kf = crate::keyfinder::detect_key(&key_buf, cfg.key_rate);
    #[cfg(not(feature = "native-analysis"))]
    let kf = None;
    let ks = chroma::ks_key(&key_buf, cfg.key_rate);
    let key = best_key(kf, ks);

    Ok(AnalysisResult {
        bpm,
        bpm_confidence,
        onsets: onset_times,
        beats,
        grid,
        key,
        rms_db: Some(rms_db_of(&mono_acc)),
        peak_db: Some(peak_db_of(&mono_acc)),
        energy_windows: energy_windows(&mono_acc, rate, cfg.energy_window_ms),
    })
}

/// Open the file and analyze it with a non-cancelling cancel flag.
pub fn analyze_path(
    path: impl AsRef<std::path::Path>,
    cfg: &AnalysisConfig,
) -> Result<AnalysisResult, AppError> {
    let mut stream = DecodedStream::open(path)?;
    let cancel = AtomicBool::new(false);
    analyze(&mut stream, cfg, &cancel)
}

#[cfg(feature = "native-analysis")]
fn feed_tempo(t: &mut Tempo, buf: &mut Vec<f32>, hop: usize) {
    let mut idx = 0usize;
    while idx + hop <= buf.len() {
        // Output is the fractional part of the predicted tactus, not onset
        // strength — beats come from the onset stream instead.
        let _ = t.do_result(&buf[idx..idx + hop]);
        idx += hop;
    }
    buf.drain(..idx);
}

#[cfg(feature = "native-analysis")]
fn feed_onset(o: &mut Onset, buf: &mut Vec<f32>, hop: usize, out: &mut Vec<f64>) {
    let mut idx = 0usize;
    while idx + hop <= buf.len() {
        if let Ok(r) = o.do_result(&buf[idx..idx + hop]) {
            if r > 0.5 {
                // `get_last_s()` is absolute from the start of the stream
                // (aubio accumulates total_frames internally).
                out.push(o.get_last_s() as f64);
            }
        }
        idx += hop;
    }
    buf.drain(..idx);
}

/// Native: streaming `Tempo::get_bpm()`/`get_confidence()` (Task 4's
/// gate-proven approach), falling back to autocorrelation on the capped
/// mono buffer when aubio reports no tempo (e.g. silence). Fallback:
/// autocorrelation only.
#[cfg(feature = "native-analysis")]
fn extract_bpm(
    tempo: Option<&Tempo>,
    fallback_mono: &[f32],
    rate: u32,
) -> (Option<f64>, Option<f32>) {
    if let Some(t) = tempo {
        let bpm = t.get_bpm();
        let conf = t.get_confidence();
        if bpm > 0.0 && conf >= 0.05 {
            return (Some(bpm as f64), Some(conf));
        }
    }
    (tempo::autocorr_bpm(fallback_mono, rate), None)
}
