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

#[test]
fn project_crud_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("Mix 1").unwrap();
    assert!(!p.id.is_empty());
    let projects = storage.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].name, "Mix 1");
    storage.delete_project(&p.id).unwrap();
    assert!(storage.list_projects().unwrap().is_empty());
}

#[test]
fn track_insert_and_list() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("Mix 1").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Test Track");
    assert_eq!(tracks[0].peaks.len(), 1);
    assert!(storage.list_tracks(None).unwrap().is_empty());
}

#[test]
fn prefs_upsert() {
    let storage = Storage::open_in_memory().unwrap();
    storage.set_pref("theme", "dark").unwrap();
    assert_eq!(storage.get_pref("theme").unwrap(), Some("dark".into()));
    storage.set_pref("theme", "light").unwrap();
    assert_eq!(storage.get_pref("theme").unwrap(), Some("light".into()));
}

#[test]
fn migrations_are_idempotent() {
    let storage = Storage::open_in_memory().unwrap();
    let storage2 = Storage::open_in_memory().unwrap();
    storage.create_project("A").unwrap();
    storage2.create_project("B").unwrap();
    assert_eq!(storage.list_projects().unwrap().len(), 1);
    assert_eq!(storage2.list_projects().unwrap().len(), 1);
}

#[test]
fn analysis_upsert_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    let row = AnalysisRow {
        track_id: tracks[0].id.clone(),
        file_hash: "abc".into(),
        bpm: Some(128.0),
        bpm_confidence: Some(0.9),
        key: Some("AMinor".into()),
        key_confidence: Some(0.8),
        energy: r#"{"rms_db":-12.0,"peak_db":-1.0,"energy_windows":[1,2,3]}"#.into(),
        created_at: "123".into(),
    };
    storage.upsert_analysis(&row).unwrap();
    let got = storage.get_analysis(&row.track_id).unwrap().unwrap();
    assert_eq!(got.bpm, Some(128.0));
    assert_eq!(got.key.as_deref(), Some("AMinor"));
    storage.upsert_analysis(&row).unwrap(); // idempotent upsert
    let again = storage.get_analysis(&row.track_id).unwrap().unwrap();
    assert_eq!(again.bpm, Some(128.0));
}

#[test]
fn beat_grid_roundtrip() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    let grid = r#"{"first_beat_offset":0.87,"bpm":120.0,"beat_interval":0.5,"confidence":0.95,"curve":[]}"#;
    storage
        .upsert_beat_grid(&tracks[0].id, "abc", grid)
        .unwrap();
    assert_eq!(
        storage.get_beat_grid(&tracks[0].id).unwrap(),
        Some(grid.to_string())
    );
}

#[test]
fn delete_project_cascades_analysis() {
    let storage = Storage::open_in_memory().unwrap();
    let p = storage.create_project("P").unwrap();
    storage.insert_track(&sample_track(Some(&p.id))).unwrap();
    // upsert with the real inserted track id
    let tracks = storage.list_tracks(Some(&p.id)).unwrap();
    let row = AnalysisRow {
        track_id: tracks[0].id.clone(),
        file_hash: "abc".into(),
        bpm: None,
        bpm_confidence: None,
        key: None,
        key_confidence: None,
        energy: "{}".into(),
        created_at: "1".into(),
    };
    storage.upsert_analysis(&row).unwrap();
    storage.delete_project(&p.id).unwrap();
    assert!(storage.get_analysis(&row.track_id).unwrap().is_none());
}
