use std::path::Path;

use openmix_app_lib::commands::analysis::{
    analyze_track_inner, get_analysis_inner, get_beat_grid_inner,
};
use openmix_app_lib::storage::{AnalysisRow, Storage, Track};

fn sample_track(project_id: Option<&str>) -> Track {
    Track {
        id: uuid::Uuid::new_v4().to_string(),
        project_id: project_id.map(|s| s.to_string()),
        path: "/tmp/test.wav".into(),
        title: "Test Track".into(),
        artist: Some("Artist".into()),
        album: None,
        duration_ms: 1000,
        sample_rate: 44100,
        channels: 1,
        format: "wav".into(),
        file_hash: "abc".into(),
        peaks: vec![openmix_core::audio::Peak {
            min: -0.5,
            max: 0.5,
        }],
        created_at: "".into(),
    }
}

fn wav_fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../openmix-core/tests/fixtures/kick_120bpm.wav")
}

#[test]
fn analyze_track_persists_and_returns() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    let mut t = sample_track(Some(&p.id));
    t.file_hash = "fixture-hash".into();
    storage.insert_track(&t).unwrap();
    let id = analyze_track_inner(&storage, &t.id, &wav_fixture_path()).unwrap();
    assert_eq!(id, t.id);
    let analysis = get_analysis_inner(&storage, &t.id).unwrap();
    assert!(analysis.is_some());
    let row = storage.get_analysis(&t.id).unwrap().unwrap();
    assert_eq!(row.file_hash, "fixture-hash");
}

#[test]
fn cache_hit_returns_without_reanalysis() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    let mut t = sample_track(Some(&p.id));
    t.file_hash = "fixture-hash".into();
    storage.insert_track(&t).unwrap();
    let row = AnalysisRow {
        track_id: t.id.clone(),
        file_hash: "fixture-hash".into(),
        bpm: Some(120.0),
        bpm_confidence: Some(0.9),
        key: Some("AMinor".into()),
        key_confidence: Some(0.8),
        energy: r#"{"rms_db":-12.0,"peak_db":-1.0,"energy_windows":[1,2,3]}"#.into(),
        created_at: "cached-stamp".into(),
    };
    storage.upsert_analysis(&row).unwrap();
    let id = analyze_track_inner(&storage, &t.id, &wav_fixture_path()).unwrap();
    assert_eq!(id, t.id);
    let cached = storage.get_analysis(&t.id).unwrap().unwrap();
    assert_eq!(cached.created_at, "cached-stamp");
    assert_eq!(cached.bpm, Some(120.0));
}

#[test]
fn beat_grid_roundtrip_via_command() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    let mut t = sample_track(Some(&p.id));
    t.file_hash = "fixture-hash".into();
    storage.insert_track(&t).unwrap();
    let id = analyze_track_inner(&storage, &t.id, &wav_fixture_path()).unwrap();
    let g = get_beat_grid_inner(&storage, &id).unwrap();
    let grid = g.expect("beat grid should be stored for the kick fixture");
    assert!((grid.bpm - 120.0).abs() <= 2.0, "bpm = {}", grid.bpm);
}
