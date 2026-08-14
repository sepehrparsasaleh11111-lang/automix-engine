use super::{fit_uniform, BeatGrid, BpmCurvePoint};

/// Re-fit with a tighter window when confidence is low (backend automatic
/// correction; the manual-edit re-fits in Phase 3 call the same helpers).
pub fn correct(grid: BeatGrid, beats: &[f64], tightened_tolerance_ms: f64) -> BeatGrid {
    let _ = tightened_tolerance_ms;
    if grid.confidence >= 0.8 || beats.len() < 4 {
        return grid;
    }
    let interval = grid.beat_interval.max(0.05);
    let low = interval * 0.99;
    let high = interval * 1.01;
    let mut best: Option<BeatGrid> = None;
    let mut steps = 0f64;
    while steps < 8.0 {
        let delta = interval * 0.0025 * steps;
        for candidate_interval in [interval, low, high, interval - delta, interval + delta] {
            if candidate_interval <= 0.0 {
                continue;
            }
            let g = fit_uniform(beats, candidate_interval);
            if best
                .as_ref()
                .map(|b| g.confidence > b.confidence)
                .unwrap_or(true)
            {
                best = Some(g);
            }
        }
        if best.as_ref().map(|b| b.confidence >= 0.8).unwrap_or(false) {
            break;
        }
        steps += 1.0;
    }
    best.unwrap_or(grid)
}

/// Sliding-window (8-beat) interval estimates vs the uniform grid; a monotonic
/// residual trend ⇒ tempo drift ⇒ BpmCurve. Empty vec = no drift.
pub fn detect_drift(beats: &[f64], grid: &BeatGrid) -> Vec<BpmCurvePoint> {
    if beats.len() < 16 {
        return vec![];
    }
    let window = 8usize;
    let mut points = Vec::new();
    for start in (0..beats.len() - window).step_by(window) {
        let seg = &beats[start..start + window];
        let d: f64 = seg.windows(2).map(|w| w[1] - w[0]).sum::<f64>() / (window - 1) as f64;
        let bpm = 60.0 / d;
        if (bpm - grid.bpm).abs() / grid.bpm > 0.01 {
            points.push(BpmCurvePoint {
                position_sec: seg[0],
                bpm,
            });
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::{correct, detect_drift, fit_uniform, BeatGrid};

    #[test]
    fn low_confidence_grid_is_refit() {
        let beats: Vec<f64> = (0..64).map(|i| 0.1 + i as f64 * 0.3333).collect();
        let bad = BeatGrid {
            first_beat_offset: 0.0,
            bpm: 120.0,
            beat_interval: 0.5,
            confidence: 0.3,
            curve: vec![],
        };
        let fixed = correct(bad.clone(), &beats, 50.0);
        assert!(
            fixed.confidence > bad.confidence,
            "{} -> {}",
            bad.confidence,
            fixed.confidence
        );
        assert!(
            (fixed.bpm - 180.0).abs() < 180.0 * 0.02,
            "bpm {}",
            fixed.bpm
        );
    }

    #[test]
    fn drift_is_detected_as_variable_grid() {
        let mut beats = Vec::new();
        let mut t = 0.0;
        let mut interval = 0.5;
        let n = 64usize;
        for _ in 0..n {
            beats.push(t);
            interval -= 0.000375;
            t += interval;
        }
        let g = fit_uniform(&beats, 0.5);
        let curve = detect_drift(&beats, &g);
        assert!(!curve.is_empty(), "drift not detected");
        let mid_bpm = curve[curve.len() / 2].bpm;
        assert!((mid_bpm - 123.0).abs() <= 123.0 * 0.02, "mid bpm {mid_bpm}");
        assert!((g.bpm - 123.0).abs() < 1.5, "grid bpm {}", g.bpm);
    }

    #[test]
    fn uniform_track_has_no_curve() {
        let beats: Vec<f64> = (0..64).map(|i| 0.25 + i as f64 * 0.5).collect();
        let g = fit_uniform(&beats, 0.5);
        assert!(detect_drift(&beats, &g).is_empty());
    }
}
