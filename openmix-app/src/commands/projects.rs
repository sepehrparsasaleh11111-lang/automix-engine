use tauri::State;

use crate::storage::Project;
use crate::AppState;

#[tauri::command]
pub fn list_projects(state: State<'_, AppState>) -> Result<Vec<Project>, String> {
    state.storage.list_projects().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_project(state: State<'_, AppState>, name: String) -> Result<Project, String> {
    state
        .storage
        .create_project(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_project(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state.storage.delete_project(&id).map_err(|e| e.to_string())
}
