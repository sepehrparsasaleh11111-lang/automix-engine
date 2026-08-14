use openmix_core::analysis::{analyze_path, AnalysisConfig};
use openmix_core::error::AppError;
use std::path::PathBuf;

fn main() -> Result<(), AppError> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let path = args
        .iter()
        .find(|a| a.as_str() != "--json")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            eprintln!("usage: analyze <path-to-audio> [--json]");
            std::process::exit(2);
        });
    let result = analyze_path(path, &AnalysisConfig::default())?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&result).map_err(|e| AppError::Other(e.to_string()))?
        );
    } else {
        println!(
            "bpm: {:?} (confidence {:?})",
            result.bpm, result.bpm_confidence
        );
        println!(
            "key: {:?}",
            result.key.as_ref().map(|k| (k.key, k.confidence))
        );
        println!(
            "grid: {:?}",
            result
                .grid
                .as_ref()
                .map(|g| (g.first_beat_offset, g.bpm, g.confidence))
        );
        println!(
            "onsets: {} beats: {} energy windows: {} rms_db: {:?} peak_db: {:?}",
            result.onsets.len(),
            result.beats.len(),
            result.energy_windows.len(),
            result.rms_db,
            result.peak_db
        );
    }
    Ok(())
}
