use std::path::Path;

use tauri::State;

use crate::import::{import_file, TrackSummary};
use crate::AppState;

#[tauri::command]
pub fn list_tracks(
    state: State<'_, AppState>,
    project_id: Option<String>,
) -> Result<Vec<TrackSummary>, String> {
    let tracks = state
        .storage
        .list_tracks(project_id.as_deref())
        .map_err(|e| e.to_string())?;
    Ok(tracks
        .into_iter()
        .map(|t| TrackSummary {
            id: t.id,
            path: t.path,
            title: t.title,
            artist: t.artist,
            album: t.album,
            duration_ms: t.duration_ms,
            sample_rate: t.sample_rate,
            channels: t.channels,
            format: t.format,
            peaks: t.peaks,
        })
        .collect())
}

#[tauri::command]
pub fn import_tracks(
    state: State<'_, AppState>,
    paths: Vec<String>,
    project_id: Option<String>,
) -> Result<Vec<TrackSummary>, String> {
    let mut summaries = Vec::new();
    for p in &paths {
        summaries.push(import_file(
            &state.storage,
            Path::new(p),
            project_id.as_deref(),
        )?);
    }
    Ok(summaries)
}
