#![cfg_attr(not(feature = "native-analysis"), allow(dead_code))]

use crate::beatgrid::{fit_uniform, Beat, BeatLabel};

#[cfg(feature = "native-analysis")]
pub fn aubio_beats(mono: &[f32], rate: u32) -> Vec<Beat> {
    label_beats(crate::analysis::onsets::aubio_onsets(mono, rate))
}

pub fn histogram_beats(mono: &[f32], rate: u32) -> Vec<Beat> {
    let onsets = crate::analysis::onsets::flux_onsets(mono, rate);
    if onsets.len() < 4 {
        return vec![];
    }
    let mut diffs: Vec<f64> = onsets
        .windows(2)
        .map(|w| w[1] - w[0])
        .filter(|d| *d > 0.2)
        .collect();
    if diffs.is_empty() {
        return vec![];
    }
    diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = diffs[diffs.len() / 2];
    let times = onsets;
    let grid = fit_uniform(&times, median);
    let step = grid.beat_interval;
    let mut beats = Vec::new();
    let mut t = grid.first_beat_offset;
    let mut idx = 0usize;
    while t <= times.last().unwrap() + step {
        beats.push(Beat {
            position_sec: t,
            label: if idx.is_multiple_of(4) {
                BeatLabel::Downbeat
            } else {
                BeatLabel::Beat
            },
        });
        idx += 1;
        t += step;
    }
    beats
}

pub(crate) fn label_beats(mut times: Vec<f64>) -> Vec<Beat> {
    if times.is_empty() {
        return vec![];
    }
    times.dedup();
    let grid = fit_uniform(&times, 0.5);
    // fit_uniform's offset maximizes grid-line hits within tolerance and can
    // sit off the true beat phase; snap it so the first time is grid line 0.
    let phase = ((times[0] - grid.first_beat_offset) / grid.beat_interval).round();
    let offset = grid.first_beat_offset + phase * grid.beat_interval;
    times.retain(|t| (t - offset) >= -grid.beat_interval / 2.0);
    times
        .into_iter()
        .map(|t| {
            let idx = ((t - offset) / grid.beat_interval).round() as usize;
            Beat {
                position_sec: t,
                label: if idx.is_multiple_of(4) {
                    BeatLabel::Downbeat
                } else {
                    BeatLabel::Beat
                },
            }
        })
        .collect()
}

#[cfg(feature = "native-analysis")]
pub struct AubioBeatTracker;
#[cfg(feature = "native-analysis")]
impl super::BeatTracker for AubioBeatTracker {
    fn beats(&self, mono: &[f32], rate: u32) -> Vec<Beat> {
        aubio_beats(mono, rate)
    }
}

pub struct HistogramBeatTracker;
impl super::BeatTracker for HistogramBeatTracker {
    fn beats(&self, mono: &[f32], rate: u32) -> Vec<Beat> {
        histogram_beats(mono, rate)
    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "native-analysis")]
    use super::aubio_beats;
    use super::histogram_beats;
    #[cfg(feature = "native-analysis")]
    use crate::beatgrid::BeatLabel;

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
    fn aubio_beats_match_128bpm_grid() {
        let mono = synthetic_kick(44100, 128.0, 20.0);
        let beats = aubio_beats(&mono, 44100);
        assert!(beats.len() >= 40, "few beats: {}", beats.len());
        let interval = 60.0 / 128.0;
        for w in beats.windows(2) {
            assert!((w[1].position_sec - w[0].position_sec - interval).abs() < interval * 0.06);
        }
        assert_eq!(beats[0].label, BeatLabel::Downbeat);
        assert_eq!(beats[3].label, BeatLabel::Beat);
    }

    #[test]
    fn histogram_beats_detect_grid() {
        let mono = synthetic_kick(44100, 120.0, 20.0);
        let beats = histogram_beats(&mono, 44100);
        assert!(!beats.is_empty());
        for w in beats.windows(2) {
            assert!((w[1].position_sec - w[0].position_sec - 0.5).abs() < 0.05);
        }
    }
}
