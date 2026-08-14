#[cfg(feature = "native-analysis")]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "native-analysis")]
use super::AnalysisCancel;

#[cfg(feature = "native-analysis")]
pub fn aubio_bpm(mono: &[f32], rate: u32) -> Option<(f64, f32)> {
    let cancel = AtomicBool::new(false);
    aubio_bpm_cancellable(mono, rate, &cancel)
}

#[cfg(feature = "native-analysis")]
pub(crate) fn aubio_bpm_cancellable(
    mono: &[f32],
    rate: u32,
    cancel: &AnalysisCancel,
) -> Option<(f64, f32)> {
    use aubio_rs::{OnsetMode, Tempo};
    let hop = 512usize;
    let mut tempo = Tempo::new(OnsetMode::SpecFlux, 1024, hop, rate).ok()?;
    let mut padded = mono.to_vec();
    let rem = padded.len() % hop;
    if rem != 0 {
        padded.resize(padded.len() + hop - rem, 0.0); // aubio reads a full hop from input
    }
    for chunk in padded.chunks(hop) {
        if cancel.load(Ordering::Relaxed) {
            return None;
        }
        let out = tempo.do_result(chunk).ok()?;
        let _ = out; // 1.0 → beat at get_last_s()
    }
    let bpm = tempo.get_bpm();
    let conf = tempo.get_confidence();
    if bpm <= 0.0 || conf < 0.05 {
        return None;
    }
    Some((bpm as f64, conf))
}

pub fn autocorr_bpm(mono: &[f32], rate: u32) -> Option<f64> {
    let work = crate::audio::mono::downsample_mono(mono, rate, 11_025);
    let min_lag = (11_025.0 * 60.0 / 180.0) as usize; // 180 BPM
    let max_lag = (11_025.0 * 60.0 / 60.0) as usize; // 60 BPM
    if work.len() <= max_lag + 1 {
        return None;
    }
    let mean: f64 = work.iter().map(|s| *s as f64).sum::<f64>() / work.len() as f64;
    let x: Vec<f64> = work.iter().map(|s| *s as f64 - mean).collect();
    let mut best_lag = 0usize;
    let mut best_score = f64::MIN;
    for lag in min_lag..=max_lag {
        let score: f64 = x[..x.len() - lag]
            .iter()
            .zip(&x[lag..])
            .map(|(a, b)| a * b)
            .sum::<f64>()
            / (x.len() - lag) as f64;
        if score > best_score {
            best_score = score;
            best_lag = lag;
        }
    }
    if best_score <= 0.0 {
        return None;
    }
    Some(60.0 * 11_025.0 / best_lag as f64)
}

#[cfg(feature = "native-analysis")]
pub struct AubioTempoDetector;
#[cfg(feature = "native-analysis")]
impl super::TempoDetector for AubioTempoDetector {
    fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64> {
        aubio_bpm(mono, rate).map(|(b, _)| b)
    }
}

pub struct AutocorrTempoDetector;
impl super::TempoDetector for AutocorrTempoDetector {
    fn bpm(&self, mono: &[f32], rate: u32) -> Option<f64> {
        autocorr_bpm(mono, rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_kick(rate: u32, bpm: f64, seconds: f64) -> Vec<f32> {
        let interval = 60.0 / bpm;
        let n = (rate as f64 * seconds) as usize;
        let mut out = vec![0.0f32; n];
        let mut t = 0.0;
        while t < seconds - interval {
            let start = (t * rate as f64) as usize;
            let len = (rate as f64 * 0.03) as usize; // 30 ms kick
            for i in 0..len {
                if start + i < n {
                    let env = 1.0 - i as f32 / len as f32;
                    out[start + i] =
                        0.9 * env * (std::f32::consts::TAU * 55.0 * i as f32 / rate as f32).sin();
                }
            }
            t += interval;
        }
        out
    }

    #[cfg(feature = "native-analysis")]
    #[test]
    fn aubio_detects_120bpm_kick() {
        let mono = synthetic_kick(44100, 120.0, 20.0);
        let (bpm, conf) = aubio_bpm(&mono, 44100).expect("detect");
        assert!((bpm - 120.0).abs() <= 120.0 * 0.02, "bpm = {bpm}"); // aubio period grid: ±1.6% at hop 512
        assert!(conf > 0.0);
    }

    #[test]
    fn autocorr_detects_120bpm_kick() {
        let mono = synthetic_kick(44100, 120.0, 20.0);
        let bpm = autocorr_bpm(&mono, 44100).expect("detect");
        assert!((bpm - 120.0).abs() <= 120.0 * 0.015, "bpm = {bpm}");
    }

    #[test]
    fn silence_returns_none() {
        let silent = vec![0.0f32; 44100 * 5];
        assert!(autocorr_bpm(&silent, 44100).is_none());
    }
}
