/// Average interleaved channels to mono, then linearly resample.
pub fn to_mono(interleaved: &[f32], channels: u16, sample_rate: u32, target_rate: u32) -> Vec<f32> {
    if channels <= 1 {
        return resample_linear(interleaved, sample_rate, target_rate);
    }
    let ch = channels as usize;
    let frames = interleaved.len() / ch;
    let mut mono = Vec::with_capacity(frames);
    for f in 0..frames {
        let sum: f32 = interleaved[f * ch..f * ch + ch].iter().sum();
        mono.push(sum / ch as f32);
    }
    resample_linear(&mono, sample_rate, target_rate)
}

fn resample_linear(src: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return src.to_vec();
    }
    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (src.len() as f64 / ratio).floor() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f64 * ratio;
        let lo = pos.floor() as usize;
        let hi = (lo + 1).min(src.len() - 1);
        let frac = (pos - lo as f64) as f32;
        out.push(src[lo] * (1.0 - frac) + src[hi] * frac);
    }
    out
}

/// Decimate with box averaging — used for the key-analysis buffer.
pub fn downsample_mono(mono: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate <= to_rate {
        return mono.to_vec();
    }
    let factor = (from_rate / to_rate) as usize;
    let mut out = Vec::with_capacity(mono.len() / factor + 1);
    for chunk in mono.chunks(factor) {
        out.push(chunk.iter().sum::<f32>() / chunk.len() as f32);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_downmix_averages_channels() {
        let inter = [0.2f32, 0.6, 0.4, 0.8, -0.2, -0.8];
        let mono = to_mono(&inter, 2, 44100, 44100);
        assert_eq!(mono.len(), 3);
        assert!((mono[0] - 0.4).abs() < 1e-6);
        assert!((mono[1] - 0.6).abs() < 1e-6);
        assert!((mono[2] + 0.5).abs() < 1e-6);
    }

    #[test]
    fn resample_halves_length() {
        let mono: Vec<f32> = (0..4410).map(|i| (i as f32 / 4410.0).sin()).collect();
        let out = to_mono(&mono, 1, 44100, 22050);
        assert!((out.len() as i64 - 2205).abs() <= 2);
    }

    #[test]
    fn resample_identity_is_close() {
        let mono: Vec<f32> = (0..1000).map(|i| (i as f32).sin()).collect();
        let out = to_mono(&mono, 1, 44100, 44100);
        assert_eq!(out.len(), 1000);
        for (a, b) in mono.iter().zip(out.iter()) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn downsample_to_quarter_length() {
        let mono: Vec<f32> = (0..4410).map(|i| (i as f32).sin()).collect();
        let out = downsample_mono(&mono, 44100, 11025);
        assert_eq!(out.len(), 1103); // 4410 / 4 = 1102 r 2 → 1103 box chunks
    }
}
