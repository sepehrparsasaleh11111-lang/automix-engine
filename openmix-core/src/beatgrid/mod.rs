use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BeatLabel {
    Downbeat,
    Beat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Beat {
    pub position_sec: f64,
    pub label: BeatLabel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct BpmCurvePoint {
    pub position_sec: f64,
    pub bpm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeatGrid {
    pub first_beat_offset: f64,
    pub bpm: f64,
    pub beat_interval: f64,
    pub confidence: f32,
    pub curve: Vec<BpmCurvePoint>,
}

pub mod correct;

pub fn fit_uniform(beats: &[f64], beat_interval_guess: f64) -> BeatGrid {
    if beats.len() < 2 || beat_interval_guess <= 0.0 {
        return BeatGrid {
            first_beat_offset: 0.0,
            bpm: 0.0,
            beat_interval: 0.0,
            confidence: 0.0,
            curve: vec![],
        };
    }
    // robust interval: median of positive diffs
    let mut diffs: Vec<f64> = beats
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > 0.0)
        .collect();
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let interval = if diffs.is_empty() {
        beat_interval_guess
    } else {
        diffs[diffs.len() / 2]
    };
    let tol = interval * 0.10;
    // sweep offset within one interval, maximizing grid-line match
    let mut best_offset = beats[0];
    let mut best_score = 0usize;
    let steps = 200usize;
    for s in 0..steps {
        let offset = beats[0] + interval * s as f64 / steps as f64;
        let mut score = 0usize;
        for b in beats {
            let pos = (b - offset) / interval;
            let nearest = pos.round();
            if (pos - nearest).abs() * interval <= tol {
                score += 1;
            }
        }
        if score > best_score {
            best_score = score;
            best_offset = offset;
        }
    }
    let confidence = best_score as f32 / beats.len() as f32;
    BeatGrid {
        first_beat_offset: best_offset,
        bpm: 60.0 / interval,
        beat_interval: interval,
        confidence,
        curve: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::fit_uniform;

    #[test]
    fn ideal_grid_recovers_offset_and_bpm() {
        let mut beats = Vec::new();
        for i in 0..64 {
            beats.push(0.25 + i as f64 * 0.5);
        }
        let g = fit_uniform(&beats, 0.5);
        assert!(
            (g.first_beat_offset - 0.25).abs() < 1e-3,
            "offset {}",
            g.first_beat_offset
        );
        assert!((g.bpm - 120.0).abs() < 0.1, "bpm {}", g.bpm);
        assert!(g.confidence > 0.99);
        assert!(g.curve.is_empty());
    }

    #[test]
    fn jittered_grid_keeps_high_confidence() {
        let mut beats = Vec::new();
        for i in 0..64 {
            let jitter = (i as f64 * 13.7).sin() * 0.02;
            beats.push(0.25 + i as f64 * 0.5 + jitter);
        }
        let g = fit_uniform(&beats, 0.5);
        assert!(g.confidence > 0.8, "conf {}", g.confidence);
        assert!((g.first_beat_offset - 0.25).abs() < 0.02);
    }

    #[test]
    fn short_input_low_confidence_no_panic() {
        let g = fit_uniform(&[], 0.5);
        assert_eq!(g.confidence, 0.0);
        let g2 = fit_uniform(&[1.0], 0.5);
        assert!(g2.confidence < 0.5);
    }
}
