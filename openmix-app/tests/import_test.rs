use openmix_app_lib::import::import_file;
use openmix_app_lib::storage::Storage;

#[test]
fn import_wav_persists_track_with_peaks() {
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("import_me.wav");
    std::fs::write(&wav, SINE_WAV_BYTES).unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();

    let summary = import_file(&storage, &wav, Some(&p.id)).unwrap();

    assert_eq!(summary.title, "import_me");
    assert_eq!(summary.format, "wav");
    assert_eq!(summary.sample_rate, 44100);
    assert_eq!(summary.peaks.len(), 2000);
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].file_hash.len(), 64);
    assert!(tracks[0].file_hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn import_streamed_hash_matches_known_digest() {
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let wav = dir.path().join("hash_me.wav");
    std::fs::write(&wav, SINE_WAV_BYTES).unwrap();
    let storage = Storage::open_in_memory().unwrap();

    let _summary = import_file(&storage, &wav, None).unwrap();

    let mut h = Sha256::new();
    h.update(SINE_WAV_BYTES);
    let expected: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    let stored = storage.list_tracks(None).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].file_hash, expected,
        "streamed hash diverged from reference digest"
    );
}

#[test]
fn import_hash_changes_when_content_changes() {
    let dir = tempfile::tempdir().unwrap();
    let wav_a = dir.path().join("a.wav");
    let wav_b = dir.path().join("b.wav");
    std::fs::write(&wav_a, SINE_WAV_BYTES).unwrap();
    let mut tampered = SINE_WAV_BYTES.to_vec();
    let last = tampered.len() - 1;
    tampered[last] ^= 0x01;
    std::fs::write(&wav_b, &tampered).unwrap();
    let storage = Storage::open_in_memory().unwrap();

    let _a = import_file(&storage, &wav_a, None).unwrap();
    let _b = import_file(&storage, &wav_b, None).unwrap();

    let stored = storage.list_tracks(None).unwrap();
    assert_eq!(stored.len(), 2);
    let hash_a = stored
        .iter()
        .find(|t| t.path.ends_with("a.wav"))
        .unwrap()
        .file_hash
        .clone();
    let hash_b = stored
        .iter()
        .find(|t| t.path.ends_with("b.wav"))
        .unwrap()
        .file_hash
        .clone();
    assert_ne!(hash_a, hash_b, "different content produced the same hash");
    assert_eq!(hash_a.len(), 64);
}

#[test]
fn import_unsupported_format_errors() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.txt");
    std::fs::write(&bad, "not audio").unwrap();
    let storage = Storage::open_in_memory().unwrap();
    let err = import_file(&storage, &bad, None).unwrap_err();
    assert!(
        err.contains("unsupported") || err.contains("decode"),
        "got: {err}"
    );
}

const SINE_WAV_BYTES: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../openmix-core/tests/fixtures/sine1k_1s.wav"
));
