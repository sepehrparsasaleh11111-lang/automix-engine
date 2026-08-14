pub mod commands;
pub mod import;
pub mod storage;

use std::sync::Arc;

use storage::Storage;
use tauri::Manager;

pub struct AppState {
    pub storage: Arc<Storage>,
}

#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let storage = Storage::open(&data_dir.join("openmix.db"))?;
            app.manage(AppState {
                storage: Arc::new(storage),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            commands::projects::list_projects,
            commands::projects::create_project,
            commands::projects::delete_project,
            commands::tracks::list_tracks,
            commands::tracks::import_tracks,
            commands::analysis::analyze_track,
            commands::analysis::get_analysis,
            commands::analysis::get_beat_grid
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenMix AI");
}
