#[cfg(feature = "native-analysis")]
pub fn aubio_onsets(mono: &[f32], rate: u32) -> Vec<f64> {
    use aubio_rs::{Onset, OnsetMode};
    let hop = 512usize;
    let mut out = Vec::new();
    let mut onset = match Onset::new(OnsetMode::SpecFlux, 1024, hop, rate) {
        Ok(o) => o,
        Err(_) => return out,
    };
    for chunk in mono.chunks(hop) {
        if let Ok(r) = onset.do_result(chunk) {
            if r > 0.5 {
                out.push(onset.get_last_s() as f64);
            }
        }
    }
    out
}

pub fn flux_onsets(mono: &[f32], rate: u32) -> Vec<f64> {
    use rustfft::{num_complex::Complex, FftPlanner};
    let fft_size = 1024usize;
    let hop = 256usize;
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_size);
    let mut flux: Vec<f32> = Vec::new();
    let mut prev: Vec<f32> = vec![0.0; fft_size];
    let mut buf = vec![Complex::new(0.0f32, 0.0); fft_size];
    let mut spectrum = vec![0.0f32; fft_size];

    for chunk in mono.chunks(hop) {
        if chunk.len() < hop {
            break;
        }
        for (i, s) in chunk.iter().enumerate() {
            let w =
                0.5 - 0.5 * ((std::f32::consts::TAU * i as f32 / (fft_size as f32 - 1.0)).cos());
            buf[i] = Complex::new(s * w, 0.0);
        }
        for b in buf.iter_mut().skip(hop) {
            b.re = 0.0;
            b.im = 0.0;
        }
        fft.process(&mut buf);
        for k in 0..fft_size {
            spectrum[k] = buf[k].norm();
        }
        let mut f = 0.0f32;
        for k in 0..fft_size {
            f += (spectrum[k] - prev[k]).max(0.0);
        }
        prev.copy_from_slice(&spectrum);
        flux.push(f);
    }

    let mean = flux.iter().sum::<f32>() / flux.len().max(1) as f32;
    let var = flux.iter().map(|v| (v - mean) * (v - mean)).sum::<f32>() / flux.len().max(1) as f32;
    let thresh = mean + 2.0 * var.sqrt();
    let min_ioi = ((rate as f64 * 0.03) / hop as f64).ceil() as usize; // 30 ms, in frames
    let hop_s = hop as f64 / rate as f64;

    let mut out = Vec::new();
    let mut last: Option<usize> = None;
    for (i, f) in flux.iter().enumerate() {
        if *f > thresh && last.is_none_or(|l| i >= l + min_ioi) {
            out.push(i as f64 * hop_s);
            last = Some(i);
        }
    }
    out
}

#[cfg(feature = "native-analysis")]
pub struct AubioOnsetDetector;
#[cfg(feature = "native-analysis")]
impl super::OnsetDetector for AubioOnsetDetector {
    fn onsets(&self, mono: &[f32], rate: u32) -> Vec<f64> {
        aubio_onsets(mono, rate)
    }
}

pub struct FluxOnsetDetector;
impl super::OnsetDetector for FluxOnsetDetector {
    fn onsets(&self, mono: &[f32], rate: u32) -> Vec<f64> {
        flux_onsets(mono, rate)
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
    fn aubio_onsets_match_kick_grid() {
        let mono = synthetic_kick(44100, 120.0, 20.0);
        let onsets = aubio_onsets(&mono, 44100);
        assert!(!onsets.is_empty());
        for o in &onsets {
            let nearest = (o * 2.0).round() / 2.0; // nearest beat at 0.5 s
            assert!((o - nearest).abs() < 0.03, "onset {o} not on grid");
        }
    }

    #[test]
    fn flux_onsets_detect_kicks() {
        let mono = synthetic_kick(44100, 128.0, 20.0);
        let onsets = flux_onsets(&mono, 44100);
        let expected = (20.0 * 128.0 / 60.0) as usize;
        assert!(
            (onsets.len() as i64 - expected as i64).abs() <= 3,
            "n={}",
            onsets.len()
        );
    }

    #[test]
    fn silent_input_no_onsets() {
        let silent = vec![0.0f32; 44100 * 3];
        assert!(flux_onsets(&silent, 44100).is_empty());
    }
}
