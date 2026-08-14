use std::sync::Arc;

use openmix_core::analysis::key::{KeyAlgorithm, KeyResult};
use openmix_core::analysis::{analyze_path, AnalysisConfig, AnalysisResult, MusicalKey};
use openmix_core::beatgrid::BeatGrid;
use tauri::{Emitter, State};

use crate::storage::{now_stamp, AnalysisRow, Storage};
use crate::AppState;

pub fn analyze_track_inner(
    storage: &Storage,
    track_id: &str,
    path: &std::path::Path,
) -> Result<String, String> {
    let track = storage
        .get_track(track_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("track {track_id} not found"))?;
    // cache: skip re-analysis if the stored file hash still matches
    if let Some(row) = storage.get_analysis(track_id).map_err(|e| e.to_string())? {
        if row.file_hash == track.file_hash {
            return Ok(track_id.into());
        }
    }
    let result = analyze_path(path, &AnalysisConfig::default()).map_err(|e| e.to_string())?;
    let energy_json = serde_json::json!({
        "rms_db": result.rms_db, "peak_db": result.peak_db, "energy_windows": result.energy_windows,
    })
    .to_string();
    let row = AnalysisRow {
        track_id: track_id.into(),
        file_hash: track.file_hash.clone(),
        bpm: result.bpm,
        bpm_confidence: result.bpm_confidence,
        key: result
            .key
            .as_ref()
            .map(|k| serde_json::to_string(&k.key).unwrap_or_default()),
        key_confidence: result.key.as_ref().map(|k| k.confidence),
        energy: energy_json,
        created_at: now_stamp(),
    };
    storage.upsert_analysis(&row).map_err(|e| e.to_string())?;
    if let Some(grid) = &result.grid {
        let g = serde_json::to_string(grid).map_err(|e| e.to_string())?;
        storage
            .upsert_beat_grid(track_id, &track.file_hash, &g)
            .map_err(|e| e.to_string())?;
    }
    Ok(track_id.into())
}

pub fn get_analysis_inner(
    storage: &Storage,
    track_id: &str,
) -> Result<Option<AnalysisResult>, String> {
    let Some(row) = storage.get_analysis(track_id).map_err(|e| e.to_string())? else {
        return Ok(None);
    };
    let grid = storage
        .get_beat_grid(track_id)
        .map_err(|e| e.to_string())?
        .and_then(|g| serde_json::from_str(&g).ok());
    let energy: serde_json::Value =
        serde_json::from_str(&row.energy).unwrap_or_else(|_| serde_json::json!({}));
    // The schema stores only the MusicalKey (no algorithm column, Task 13), so the
    // reconstructed KeyResult falls back to KeyFinder and loses the alternate.
    let key = row
        .key
        .and_then(|k| serde_json::from_str::<MusicalKey>(&k).ok())
        .map(|k| KeyResult {
            key: k,
            confidence: row.key_confidence.unwrap_or(0.0),
            algorithm: KeyAlgorithm::KeyFinder,
            alternate: None,
        });
    Ok(Some(AnalysisResult {
        bpm: row.bpm,
        bpm_confidence: row.bpm_confidence,
        onsets: vec![],
        beats: vec![],
        grid,
        key,
        rms_db: energy
            .get("rms_db")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        peak_db: energy
            .get("peak_db")
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        energy_windows: energy
            .get("energy_windows")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
    }))
}

pub fn get_beat_grid_inner(storage: &Storage, track_id: &str) -> Result<Option<BeatGrid>, String> {
    storage
        .get_beat_grid(track_id)
        .map_err(|e| e.to_string())?
        .map(|g| serde_json::from_str(&g).map_err(|e| e.to_string()))
        .transpose()
}

#[tauri::command]
pub fn analyze_track(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    track_id: String,
) -> Result<String, String> {
    let track = state
        .storage
        .get_track(&track_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("track {track_id} not found"))?;
    let path = track.path;
    let storage_handle: Arc<Storage> = Arc::clone(&state.storage);
    let emit_id = track_id.clone();
    std::thread::spawn(move || {
        let res = analyze_track_inner(&storage_handle, &emit_id, std::path::Path::new(&path));
        if let Ok(id) = &res {
            let _ = app.emit("analysis:done", id);
        }
    });
    Ok(track_id)
}

#[tauri::command]
pub fn get_analysis(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Option<AnalysisResult>, String> {
    get_analysis_inner(&state.storage, &track_id)
}

#[tauri::command]
pub fn get_beat_grid(
    state: State<'_, AppState>,
    track_id: String,
) -> Result<Option<BeatGrid>, String> {
    get_beat_grid_inner(&state.storage, &track_id)
}
