use super::{fit_uniform, BeatGrid};

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

pub fn detect_drift(beats: &[f64], grid: &BeatGrid) -> Vec<super::BpmCurvePoint> {
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
            points.push(super::BpmCurvePoint {
                position_sec: seg[0],
                bpm,
            });
        }
    }
    points
}
