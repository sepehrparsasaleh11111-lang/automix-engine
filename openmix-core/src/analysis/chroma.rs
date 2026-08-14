// Krumhansl–Kessler major profile (normalized), tonic = C
const KK_MAJOR: [f32; 12] = [
    6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88,
];
// Krumhansl–Kessler minor profile (normalized), tonic = C (index 0)
const KK_MINOR: [f32; 12] = [
    6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17,
];

use rustfft::{num_complex::Complex, FftPlanner};

use crate::analysis::key::{KeyAlgorithm, KeyResult, MusicalKey};

pub fn ks_key(mono: &[f32], rate: u32) -> Option<KeyResult> {
    let work = crate::audio::mono::downsample_mono(mono, rate, 11_025);
    if work.len() < 8192 {
        return None;
    }
    let fft_size = 4096usize;
    let hop = 2048usize;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut buf = vec![Complex::new(0.0f32, 0.0); fft_size];
    let mut chroma = [0.0f64; 12];
    let mut frames = 0u32;
    let mut frame_start = 0usize;
    while frame_start + fft_size <= work.len() {
        let frame = &work[frame_start..frame_start + fft_size];
        for (i, s) in frame.iter().enumerate() {
            let h =
                0.54 - 0.46 * ((std::f32::consts::TAU * i as f32 / (fft_size as f32 - 1.0)).cos());
            buf[i] = Complex::new(s * h, 0.0);
        }
        fft.process(&mut buf);
        let bin_hz = 11_025.0 / fft_size as f32;
        for (k, mag) in buf.iter().enumerate().skip(1).take(fft_size / 2 - 1) {
            let mag = mag.norm() as f64;
            if mag < 1e-6 {
                continue;
            }
            let hz = k as f32 * bin_hz;
            if !(60.0..=4000.0).contains(&hz) {
                continue;
            }
            let midi = 69.0 + 12.0 * ((hz / 440.0).ln() / std::f32::consts::LN_2);
            let class = ((midi.round() as i32).rem_euclid(12)) as usize;
            chroma[class] += mag;
        }
        frames += 1;
        frame_start += hop;
    }
    if frames == 0 {
        return None;
    }
    let total: f64 = chroma.iter().sum();
    if total <= 1e-9 {
        return None;
    }
    let norm: Vec<f32> = chroma.iter().map(|c| (c / total) as f32).collect();
    let norm_len: f32 = norm.iter().map(|c| c * c).sum::<f32>().sqrt();
    let major_len: f32 = KK_MAJOR.iter().map(|v| v * v).sum::<f32>().sqrt();
    let minor_len: f32 = KK_MINOR.iter().map(|v| v * v).sum::<f32>().sqrt();

    let mut best = (0usize, 0f32, false);
    for tonic in 0..12usize {
        let mut cmaj = 0f32;
        let mut cmin = 0f32;
        for pc in 0..12usize {
            let idx = (tonic + pc) % 12;
            cmaj += norm[idx] * KK_MAJOR[pc];
            cmin += norm[idx] * KK_MINOR[pc];
        }
        if cmaj > best.1 {
            best = (tonic, cmaj, false);
        }
        if cmin > best.1 {
            best = (tonic, cmin, true);
        }
    }
    let profile_len = if best.2 { minor_len } else { major_len };
    let conf = (best.1 / (norm_len * profile_len)).clamp(0.0, 1.0);
    let key = MusicalKey::from_tonic_mode(best.0 as i32, best.2)?;
    Some(KeyResult {
        key,
        confidence: conf,
        algorithm: KeyAlgorithm::KrumhanslSchmuckler,
        alternate: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::key::{best_key, KeyAlgorithm, KeyResult, MusicalKey};
    use crate::analysis::KeyDetector;

    fn c_major_pad(rate: u32, seconds: f64) -> Vec<f32> {
        let n = (rate as f64 * seconds) as usize;
        let attack = (rate as f64 * 0.010) as usize;
        let freqs = [261.63f64, 329.63, 392.00];
        (0..n)
            .map(|i| {
                let t = i as f64 / rate as f64;
                let mut v = 0.0f64;
                for f in freqs {
                    v += 0.2 * (2.0 * std::f64::consts::PI * f * t).sin();
                }
                let ramp = if i < attack {
                    i as f64 / attack as f64
                } else {
                    1.0
                };
                (v * ramp) as f32
            })
            .collect()
    }

    #[test]
    fn ks_detects_c_major_pad() {
        let mono = c_major_pad(44100, 6.0);
        let k = ks_key(&mono, 44100).expect("detect");
        assert!(
            k.key == MusicalKey::CMajor || k.key == MusicalKey::AMinor,
            "key {:?}",
            k.key
        );
        assert!(k.confidence > 0.3, "conf {}", k.confidence);
    }

    fn a_minor_pad(rate: u32, seconds: f64) -> Vec<f32> {
        let n = (rate as f64 * seconds) as usize;
        let attack = (rate as f64 * 0.010) as usize;
        let freqs = [440.00f64, 523.25, 659.25];
        (0..n)
            .map(|i| {
                let t = i as f64 / rate as f64;
                let mut v = 0.0f64;
                for f in freqs {
                    v += 0.2 * (2.0 * std::f64::consts::PI * f * t).sin();
                }
                let ramp = if i < attack {
                    i as f64 / attack as f64
                } else {
                    1.0
                };
                (v * ramp) as f32
            })
            .collect()
    }

    #[test]
    fn ks_detects_a_minor_pad_not_third_low() {
        let mono = a_minor_pad(44100, 6.0);
        let k = ks_key(&mono, 44100).expect("detect");
        assert!(
            k.key == MusicalKey::AMinor || k.key == MusicalKey::CMajor,
            "key {:?} (regression: minor profiles had a 3-semitone index offset)",
            k.key
        );
    }

    #[test]
    fn ks_returns_none_on_silence() {
        let silent = vec![0.0f32; 44100 * 3];
        assert!(ks_key(&silent, 44100).is_none());
    }

    #[test]
    fn best_key_prefers_higher_confidence() {
        let low = KeyResult {
            key: MusicalKey::AMajor,
            confidence: 0.4,
            algorithm: KeyAlgorithm::KrumhanslSchmuckler,
            alternate: None,
        };
        let high = KeyResult {
            key: MusicalKey::CMajor,
            confidence: 0.9,
            algorithm: KeyAlgorithm::KeyFinder,
            alternate: None,
        };
        let best = best_key(Some(high.clone()), Some(low.clone())).unwrap();
        assert_eq!(best.key, MusicalKey::CMajor);
        assert_eq!(best.alternate, Some((MusicalKey::AMajor, 0.4)));
    }

    #[test]
    fn from_tonic_mode_anchors_at_c() {
        assert_eq!(
            MusicalKey::from_tonic_mode(0, false),
            Some(MusicalKey::CMajor)
        );
        assert_eq!(
            MusicalKey::from_tonic_mode(0, true),
            Some(MusicalKey::CMinor)
        );
        assert_eq!(
            MusicalKey::from_tonic_mode(9, true),
            Some(MusicalKey::AMinor)
        );
        assert_eq!(
            MusicalKey::from_tonic_mode(2, false),
            Some(MusicalKey::DMajor)
        );
        assert_eq!(
            MusicalKey::from_tonic_mode(11, true),
            Some(MusicalKey::BMinor)
        );
        assert_eq!(
            MusicalKey::from_tonic_mode(-1, false),
            Some(MusicalKey::BMajor)
        );
    }

    #[test]
    fn krumhansl_detector_dispatches() {
        let mono = c_major_pad(44100, 6.0);
        let k = crate::analysis::key::KrumhanslKeyDetector
            .key(&mono, 44100)
            .expect("detect");
        assert_eq!(k.algorithm, KeyAlgorithm::KrumhanslSchmuckler);
    }
}
