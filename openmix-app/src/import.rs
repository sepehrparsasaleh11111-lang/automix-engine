use std::io::Read;
use std::path::Path;

use openmix_core::audio::{compute_peaks, DecodedStream};
use openmix_core::AppError;

use crate::storage::{Storage, Track};

const PEAK_POINTS: usize = 2000;
const HASH_BUF_SIZE: usize = 64 * 1024;

#[derive(Debug, serde::Serialize)]
pub struct TrackSummary {
    pub id: String,
    pub path: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: i64,
    pub sample_rate: u32,
    pub channels: u16,
    pub format: String,
    pub peaks: Vec<openmix_core::audio::Peak>,
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    let mut file =
        std::fs::File::open(path).map_err(|e| format!("failed to open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; HASH_BUF_SIZE];
    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(format!("failed to hash {}: {e}", path.display())),
        }
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

pub fn import_file(
    storage: &Storage,
    path: &Path,
    project_id: Option<&str>,
) -> Result<TrackSummary, String> {
    let file_hash = hash_file(path)?;
    let format = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let title = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string();

    let mut stream = DecodedStream::open(path).map_err(|e: AppError| e.to_string())?;
    let peaks = compute_peaks(&mut stream, PEAK_POINTS).map_err(|e: AppError| e.to_string())?;
    let meta = stream.metadata().clone();

    let track = Track {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.map(|s| s.to_string()),
        path: path.display().to_string(),
        title: meta.title.unwrap_or(title),
        artist: meta.artist,
        album: meta.album,
        duration_ms: stream.duration().as_millis() as i64,
        sample_rate: stream.sample_rate(),
        channels: stream.channels(),
        format,
        file_hash,
        peaks: peaks.clone(),
        created_at: "".into(),
    };
    storage.insert_track(&track).map_err(|e| e.to_string())?;
    Ok(TrackSummary {
        id: track.id,
        path: track.path,
        title: track.title,
        artist: track.artist,
        album: track.album,
        duration_ms: track.duration_ms,
        sample_rate: track.sample_rate,
        channels: track.channels,
        format: track.format,
        peaks,
    })
}

#[cfg(test)]
mod tests {
    use super::{hash_file, HASH_BUF_SIZE};
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn hash_buf_size_is_bounded() {
        assert_eq!(HASH_BUF_SIZE, 64 * 1024);
    }

    #[test]
    fn hash_file_missing_path_errors() {
        let err = hash_file(Path::new("/nonexistent/nope.wav")).unwrap_err();
        assert!(err.contains("failed to open"), "got: {err}");
    }

    #[test]
    fn hash_file_matches_in_memory_digest() {
        use sha2::{Digest, Sha256};
        let bytes = b"openmix hash probe bytes";
        let dir = std::env::temp_dir();
        let path = dir.join("openmix_hash_probe.bin");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(bytes).unwrap();

        let mut h = Sha256::new();
        h.update(bytes);
        let expected: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();

        assert_eq!(hash_file(&path).unwrap(), expected);
        std::fs::remove_file(&path).ok();
    }
}
