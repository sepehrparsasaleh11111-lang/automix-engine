use std::path::{Path, PathBuf};

use openmix_core::analysis::{analyze, analyze_path, AnalysisConfig};
use openmix_core::audio::DecodedStream;
use openmix_core::AppError;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn write_silent_wav(path: &Path, seconds: u32) -> std::io::Result<()> {
    let rate = 44100u32;
    let frames = (rate * seconds) as usize;
    let data_size = frames * 2;
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
    bytes.resize(bytes.len() + data_size, 0);
    std::fs::write(path, bytes)
}

fn silent_wav() -> PathBuf {
    let path = std::env::temp_dir().join("openmix_silence_4s.wav");
    write_silent_wav(&path, 4).unwrap();
    path
}

#[test]
fn analyze_120bpm_fixture_full_result() -> Result<(), AppError> {
    let r = analyze_path(fixture("kick_120bpm.wav"), &AnalysisConfig::default())?;
    assert!(
        r.bpm.is_some_and(|b| (b - 120.0).abs() <= 1.8),
        "bpm {:?}",
        r.bpm
    );
    assert!(
        r.grid.as_ref().is_some_and(|g| g.confidence > 0.8),
        "grid {:?}",
        r.grid
    );
    assert!(!r.beats.is_empty());
    assert!(!r.energy_windows.is_empty());
    assert!(r.rms_db.is_some());
    Ok(())
}

#[test]
fn analyze_mp3_and_flac_paths() -> Result<(), AppError> {
    for f in ["kick_120bpm.mp3", "kick_120bpm.flac"] {
        let r = analyze_path(fixture(f), &AnalysisConfig::default())?;
        assert!(r.bpm.is_some(), "{f}: no bpm");
    }
    Ok(())
}

#[test]
fn silence_yields_none_not_error() -> Result<(), AppError> {
    let r = analyze_path(silent_wav(), &AnalysisConfig::default())?;
    assert!(r.bpm.is_none() && r.key.is_none());
    Ok(())
}

#[test]
fn cancel_stops_early() -> Result<(), AppError> {
    use std::sync::atomic::AtomicBool;
    let cancel = AtomicBool::new(true);
    let mut stream = DecodedStream::open(fixture("kick_120bpm.wav"))?;
    let r = analyze(&mut stream, &AnalysisConfig::default(), &cancel);
    assert!(r.is_err(), "cancelled analysis should error");
    Ok(())
}
