use super::decode::DecodedStream;
use crate::error::AppError;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Peak {
    pub min: f32,
    pub max: f32,
}

pub fn compute_peaks(stream: &mut DecodedStream, points: usize) -> Result<Vec<Peak>, AppError> {
    let mut peaks: Vec<Peak> = Vec::with_capacity(points);
    let mut bucket: Vec<f32> = Vec::with_capacity(8192);
    let mut last_bucket = false;
    while let Some(chunk) = stream.next_chunk(8192)? {
        bucket.extend_from_slice(&chunk.samples);
        if bucket.len() >= points {
            let stride = bucket.len() / points;
            let mut p: Vec<Peak> = Vec::with_capacity(points);
            for i in 0..points {
                let slice = &bucket[i * stride..(i + 1) * stride];
                let min = slice.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = slice.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                p.push(Peak { min, max });
            }
            peaks = p;
            last_bucket = true;
        }
    }
    if !last_bucket && !bucket.is_empty() {
        peaks.push(Peak {
            min: bucket.iter().cloned().fold(f32::INFINITY, f32::min),
            max: bucket.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
        });
    }
    Ok(peaks)
}
