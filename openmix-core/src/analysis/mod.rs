use crate::analysis::key::KeyResult;
use crate::beatgrid::Beat;
use std::sync::atomic::AtomicBool;

pub mod beats;
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
