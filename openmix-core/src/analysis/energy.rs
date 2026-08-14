use serde::{Deserialize, Serialize};

use crate::analysis::key::KeyResult;
use crate::beatgrid::{Beat, BeatGrid};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub tempo_hop: usize,
    pub key_rate: u32,
    pub key_max_seconds: Option<f64>,
    pub energy_window_ms: u32,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            // 256 (not 512): aubio Tempo at hop 512 reads half-tempo on
            // 174/180 BPM fixtures and misses beats; 128 re-halves.
            tempo_hop: 256,
            key_rate: 11_025,
            key_max_seconds: Some(600.0),
            energy_window_ms: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub bpm: Option<f64>,
    pub bpm_confidence: Option<f32>,
    pub onsets: Vec<f64>,
    pub beats: Vec<Beat>,
    pub grid: Option<BeatGrid>,
    pub key: Option<KeyResult>,
    pub rms_db: Option<f32>,
    pub peak_db: Option<f32>,
    pub energy_windows: Vec<f32>,
}

pub fn rms_db_of(samples: &[f32]) -> f32 {
    let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    let rms = (sum_sq / samples.len().max(1) as f64).sqrt();
    if rms <= f64::EPSILON {
        -120.0
    } else {
        20.0 * rms.log10() as f32
    }
}

pub fn peak_db_of(samples: &[f32]) -> f32 {
    let peak = samples.iter().fold(0.0f32, |a, s| a.max(s.abs()));
    if peak <= f32::EPSILON {
        -120.0
    } else {
        20.0 * peak.log10()
    }
}

pub fn energy_windows(mono: &[f32], rate: u32, window_ms: u32) -> Vec<f32> {
    let win = (rate as usize * window_ms as usize / 1000).max(1);
    mono.chunks(win).map(rms_db_of).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rms_of_full_scale_sine_is_minus_3db() {
        // 1 s of unit sine at 44100 Hz (sum != 0; peak = 1.0)
        let n = 44100usize;
        let mut s = Vec::with_capacity(n);
        for i in 0..n {
            s.push((std::f32::consts::TAU * 440.0 * i as f32 / 44100.0).sin());
        }
        let rms = rms_db_of(&s);
        assert!((rms + 3.0103).abs() < 0.5, "rms_db = {rms}");
        assert!(
            (peak_db_of(&s)).abs() < 0.01,
            "peak_db = {}",
            peak_db_of(&s)
        );
    }

    #[test]
    fn energy_windows_count_matches_duration() {
        let n = 44100usize * 2; // 2 s at 44.1 kHz
        let mono = vec![0.0f32; n];
        let w = energy_windows(&mono, 44100, 100);
        assert_eq!(w.len(), 20);
    }

    #[test]
    fn energy_of_silence_is_quiet() {
        let mono = vec![0.0f32; 44100];
        assert!(rms_db_of(&mono) < -60.0);
    }
}
