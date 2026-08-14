use crate::analysis::key::KeyResult;
use crate::audio::decode::DecodedStream;
use crate::audio::mono::to_mono;
use crate::beatgrid::{correct, fit_uniform, Beat};
use crate::error::AppError;
use std::path::Path;
use std::sync::atomic::AtomicBool;

pub mod beats;
pub mod chroma;
pub mod energy;
pub mod key;
pub mod onsets;
pub mod tempo;

#[cfg(feature = "native-analysis")]
pub use beats::AubioBeatTracker;
pub use beats::HistogramBeatTracker;
pub use energy::{AnalysisConfig, AnalysisResult};
pub use key::MusicalKey;
#[cfg(feature = "native-analysis")]
pub use onsets::AubioOnsetDetector;
pub use onsets::FluxOnsetDetector;
#[cfg(feature = "native-analysis")]
pub use tempo::AubioTempoDetector;
pub use tempo::AutocorrTempoDetector;

pub type AnalysisCancel = AtomicBool;

pub trait TempoDetector {
    fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64>;
}
pub trait OnsetDetector {
    fn onsets(&self, mono: &[f32], rate: u32) -> Vec<f64>;
}
pub trait BeatTracker {
    fn beats(&self, mono: &[f32], rate: u32) -> Vec<Beat>;
}
pub trait KeyDetector {
    fn key(&self, mono: &[f32], rate: u32) -> Option<KeyResult>;
}

/// Decode an audio file and run the full analysis pipeline. This is the
/// minimal orchestrator; Phase 3 (Task 12) formalizes it into the real
/// runner with cancellation and progress.
pub fn analyze_path(
    path: impl AsRef<Path>,
    config: &AnalysisConfig,
) -> Result<AnalysisResult, AppError> {
    let mut stream = DecodedStream::open(path)?;
    let mut samples = Vec::new();
    while let Some(chunk) = stream.next_chunk(65536)? {
        samples.extend_from_slice(&chunk.samples);
    }
    let rate = stream.sample_rate();
    let mono = to_mono(&samples, stream.channels(), rate, rate);

    #[cfg(feature = "native-analysis")]
    let (bpm, bpm_confidence) = match tempo::aubio_bpm(&mono, rate) {
        Some((b, c)) => (Some(b), Some(c)),
        None => (None, None),
    };
    #[cfg(not(feature = "native-analysis"))]
    let (bpm, bpm_confidence) = (tempo::autocorr_bpm(&mono, rate), None);

    #[cfg(feature = "native-analysis")]
    let onsets = onsets::aubio_onsets(&mono, rate);
    #[cfg(not(feature = "native-analysis"))]
    let onsets = onsets::flux_onsets(&mono, rate);

    #[cfg(feature = "native-analysis")]
    let beats = beats::aubio_beats(&mono, rate);
    #[cfg(not(feature = "native-analysis"))]
    let beats = beats::histogram_beats(&mono, rate);

    let beat_times: Vec<f64> = beats.iter().map(|b| b.position_sec).collect();
    let grid = if beat_times.len() >= 2 {
        let mut g = fit_uniform(&beat_times, 0.5);
        if g.confidence < 0.8 {
            g = correct(g, &beat_times, 50.0);
        }
        Some(g)
    } else {
        None
    };

    let key_input = match config.key_max_seconds {
        Some(s) => &mono[..((rate as f64 * s) as usize).min(mono.len())],
        None => &mono[..],
    };
    #[cfg(feature = "native-analysis")]
    let key = key::best_key(
        key::KeyFinderKeyDetector.key(key_input, rate),
        key::KrumhanslKeyDetector.key(key_input, rate),
    );
    #[cfg(not(feature = "native-analysis"))]
    let key = key::KrumhanslKeyDetector.key(key_input, rate);

    Ok(AnalysisResult {
        bpm,
        bpm_confidence,
        onsets,
        beats,
        grid,
        key,
        rms_db: Some(energy::rms_db_of(&mono)),
        peak_db: Some(energy::peak_db_of(&mono)),
        energy_windows: energy::energy_windows(&mono, rate, config.energy_window_ms),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_result_defaults_to_all_none() {
        let r = AnalysisResult {
            bpm: None,
            bpm_confidence: None,
            onsets: vec![],
            beats: vec![],
            grid: None,
            key: None,
            rms_db: None,
            peak_db: None,
            energy_windows: vec![],
        };
        assert!(r.bpm.is_none() && r.key.is_none() && r.grid.is_none());
    }

    #[test]
    fn musical_key_relative_and_camelot() {
        assert_eq!(MusicalKey::CMajor.relative(), MusicalKey::AMinor);
        assert_eq!(MusicalKey::AMinor.relative(), MusicalKey::CMajor); // +9 semitones, mode flips back
        assert_eq!(MusicalKey::CMajor.camelot(), (8, 'B'));
        assert_eq!(MusicalKey::AMinor.camelot(), (8, 'A'));
    }

    #[test]
    fn camelot_relative_pairs_share_number_all_keys() {
        const ALL: [MusicalKey; 24] = [
            MusicalKey::AMajor,
            MusicalKey::ASharpMajor,
            MusicalKey::BMajor,
            MusicalKey::CMajor,
            MusicalKey::CSharpMajor,
            MusicalKey::DMajor,
            MusicalKey::DSharpMajor,
            MusicalKey::EMajor,
            MusicalKey::FMajor,
            MusicalKey::FSharpMajor,
            MusicalKey::GMajor,
            MusicalKey::GSharpMajor,
            MusicalKey::AMinor,
            MusicalKey::ASharpMinor,
            MusicalKey::BMinor,
            MusicalKey::CMinor,
            MusicalKey::CSharpMinor,
            MusicalKey::DMinor,
            MusicalKey::DSharpMinor,
            MusicalKey::EMinor,
            MusicalKey::FMinor,
            MusicalKey::FSharpMinor,
            MusicalKey::GMinor,
            MusicalKey::GSharpMinor,
        ];
        for k in ALL {
            let (n, l) = k.camelot();
            let (rn, rl) = k.relative().camelot();
            assert_eq!(
                n,
                rn,
                "key {k:?} and relative {:?} share camelot number",
                k.relative()
            );
            assert!(
                l != rl,
                "key {k:?} and relative {:?} must differ in mode letter",
                k.relative()
            );
            assert!((l == 'A' || l == 'B') && (rl == 'A' || rl == 'B'));
        }
    }

    #[test]
    fn musical_key_serializes() {
        let j = serde_json::to_string(&MusicalKey::FSharpMinor).unwrap();
        let k: MusicalKey = serde_json::from_str(&j).unwrap();
        assert_eq!(k, MusicalKey::FSharpMinor);
    }
}
