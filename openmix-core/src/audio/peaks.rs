use super::decode::DecodedStream;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Peak {
    pub min: f32,
    pub max: f32,
}

/// Reduce an audio stream to `points` min/max peak buckets in one pass with
/// bounded memory.
///
/// Each bucket covers an equal share of the stream's total duration (from
/// `DecodedStream::duration`), so every sample touches its bucket exactly once
/// — O(n) total, unlike the previous never-drained accumulator which rescanned
/// the whole buffer on every chunk (O(n^2)) and hung ~30 min on a 5-minute
/// MP3 in the `tauri dev` debug build. Files shorter than `points` frames, and
/// length-less sources (duration unknown), fall back to a single or in-memory
/// reduction respectively.
pub fn compute_peaks(stream: &mut DecodedStream, points: usize) -> Result<Vec<Peak>, AppError> {
    if points == 0 {
        return Ok(vec![]);
    }
    let total_frames = stream.duration().as_secs_f64() * stream.sample_rate() as f64;
    if total_frames < 1.0 {
        // Unknown length (e.g. no frame count in the container): accumulate the
        // mono stream, then reduce once. Same O(n) work as the fast path.
        return compute_peaks_from_mono(stream, points);
    }
    if (total_frames as usize) < points {
        return single_peak(stream);
    }
    let seg_frames = total_frames / points as f64;
    let mut peaks = vec![
        Peak {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY
        };
        points
    ];
    let channels = stream.channels() as usize;
    let mut frame_index: u64 = 0;
    while let Some(chunk) = stream.next_chunk(8192)? {
        let samples = &chunk.samples;
        for (i, s) in samples.iter().enumerate() {
            let f = frame_index + (i / channels) as u64;
            let seg = ((f as f64 / seg_frames) as usize).min(points - 1);
            let p = &mut peaks[seg];
            p.min = p.min.min(*s);
            p.max = p.max.max(*s);
        }
        frame_index += chunk.frames as u64;
    }
    Ok(peaks)
}

/// Single min/max bucket over the whole stream (used for very short files).
fn single_peak(stream: &mut DecodedStream) -> Result<Vec<Peak>, AppError> {
    let mut peak = Peak {
        min: f32::INFINITY,
        max: f32::NEG_INFINITY,
    };
    while let Some(chunk) = stream.next_chunk(8192)? {
        for s in &chunk.samples {
            peak.min = peak.min.min(*s);
            peak.max = peak.max.max(*s);
        }
    }
    if peak.min.is_finite() {
        Ok(vec![peak])
    } else {
        Ok(vec![])
    }
}

/// In-memory reduction for length-less sources: mono-accumulate then bucket.
fn compute_peaks_from_mono(
    stream: &mut DecodedStream,
    points: usize,
) -> Result<Vec<Peak>, AppError> {
    let channels = stream.channels() as usize;
    let mut mono: Vec<f32> = Vec::new();
    while let Some(chunk) = stream.next_chunk(8192)? {
        let n = chunk.samples.len() / channels;
        for f in 0..n {
            let mut acc = 0.0f32;
            for c in 0..channels {
                acc += chunk.samples[f * channels + c];
            }
            mono.push(acc / channels as f32);
        }
    }
    if mono.is_empty() {
        return Ok(vec![]);
    }
    if mono.len() < points {
        let mut peak = Peak {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        };
        for s in &mono {
            peak.min = peak.min.min(*s);
            peak.max = peak.max.max(*s);
        }
        return Ok(vec![peak]);
    }
    let seg = mono.len() / points;
    let mut peaks: Vec<Peak> = Vec::with_capacity(points);
    for i in 0..points {
        let slice = &mono[i * seg..(i + 1) * seg];
        let mut min = f32::INFINITY;
        let mut max = f32::NEG_INFINITY;
        for s in slice {
            min = min.min(*s);
            max = max.max(*s);
        }
        peaks.push(Peak { min, max });
    }
    Ok(peaks)
}
