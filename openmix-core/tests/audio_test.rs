use openmix_core::audio::{compute_peaks, DecodedStream};
use openmix_core::AppError;

fn write_sine_wav(path: &std::path::Path) -> std::io::Result<()> {
    let rate = 44100u32;
    let frames = rate as usize;
    let amp = 0.8f32;
    let mut pcm: Vec<i16> = Vec::with_capacity(frames);
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let s = (std::f32::consts::TAU * 1000.0 * t).sin() * amp;
        pcm.push((s * i16::MAX as f32) as i16);
    }
    let data_size = pcm.len() * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 2).to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_size as u32).to_le_bytes());
    for s in &pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

#[test]
fn wav_decode_reports_correct_duration() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_sine_duration.wav");
    write_sine_wav(&path).unwrap();
    let stream = DecodedStream::open(&path)?;
    assert_eq!(stream.sample_rate(), 44100);
    assert_eq!(stream.channels(), 1);
    assert!((stream.duration().as_secs_f32() - 1.0).abs() < 0.05);
    Ok(())
}

#[test]
fn wav_chunks_sum_to_full_length() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_sine_chunks.wav");
    write_sine_wav(&path).unwrap();
    let mut stream = DecodedStream::open(&path)?;
    let mut frames = 0usize;
    while let Some(chunk) = stream.next_chunk(4096)? {
        assert_eq!(chunk.samples.len() % chunk.frames, 0);
        assert!((1..=4096).contains(&chunk.frames));
        frames += chunk.frames;
    }
    assert!((44100..=44130).contains(&frames));
    Ok(())
}

#[test]
fn peaks_are_min_max_decimated() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_sine_peaks.wav");
    write_sine_wav(&path).unwrap();
    let mut stream = DecodedStream::open(&path)?;
    let peaks = compute_peaks(&mut stream, 100)?;
    assert_eq!(peaks.len(), 100);
    for p in &peaks {
        assert!(p.min >= -1.0 && p.min <= 0.0, "min {}", p.min);
        assert!(p.max >= 0.0 && p.max <= 1.0, "max {}", p.max);
    }
    assert!(peaks.iter().any(|p| p.max > 0.7), "sine peaks reach ~0.8");
    Ok(())
}

#[test]
fn flac_and_mp3_fixtures_decode() -> Result<(), AppError> {
    for name in ["sine1k_1s.flac", "sine1k_1s.mp3"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let mut stream = DecodedStream::open(&path)?;
        let mut frames = 0usize;
        while let Some(chunk) = stream.next_chunk(8192)? {
            frames += chunk.frames;
        }
        assert!(frames > 40000, "{name}: decoded {frames} frames");
    }
    Ok(())
}

fn write_long_stereo_wav(path: &std::path::Path, seconds: usize) -> std::io::Result<()> {
    let rate = 44100u32;
    let frames = rate as usize * seconds;
    let amp = 0.6f32;
    let mut pcm: Vec<i16> = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        let t = i as f32 / rate as f32;
        let s = (std::f32::consts::TAU * 1000.0 * t).sin() * amp;
        let v = (s * i16::MAX as f32) as i16;
        pcm.push(v);
        pcm.push(v);
    }
    let data_size = pcm.len() * 2;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_size as u32).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4u16.to_le_bytes());
    bytes.extend_from_slice(&16u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&(data_size as u32).to_le_bytes());
    for s in &pcm {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, bytes)
}

// Regression (Gate 2 smoke): compute_peaks used to re-scan the entire
// (never-drained) accumulator on every chunk, making import O(n^2) — a 5-minute
// MP3 hung ~30 min at 100% CPU in the `tauri dev` (debug) build. Peaks must be
// single-pass with bounded memory: this long-file run must stay well under a
// debug-build budget that the old implementation blew through by orders of
// magnitude.
#[test]
fn long_file_peaks_complete_in_single_pass() -> Result<(), AppError> {
    let dir = std::env::temp_dir();
    let path = dir.join("openmix_long_peaks.wav");
    write_long_stereo_wav(&path, 240).unwrap();

    let mut stream = DecodedStream::open(&path)?;
    let start = std::time::Instant::now();
    let peaks = compute_peaks(&mut stream, 2000)?;
    let elapsed = start.elapsed();

    std::fs::remove_file(&path).ok();

    assert_eq!(peaks.len(), 2000, "one bucket per requested point");
    for p in &peaks {
        assert!(p.min <= 0.0 && p.min >= -0.6, "min {}", p.min);
        assert!(p.max >= 0.0 && p.max <= 0.6, "max {}", p.max);
    }
    assert!(
        peaks.iter().any(|p| p.max > 0.4),
        "sine peaks should reach their amplitude"
    );
    assert!(
        elapsed.as_secs() < 45,
        "compute_peaks took {elapsed:?} on 240s of audio (O(n^2) bucket regression?)"
    );
    Ok(())
}
