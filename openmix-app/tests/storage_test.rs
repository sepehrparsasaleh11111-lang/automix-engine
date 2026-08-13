use openmix_app_lib::storage::{Storage, Track};

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
