use crate::core::config_io::{apply_import_config, export_config, preview_import_config};
use crate::db::Database;
use crate::models::{AppError, ImportPreviewResult};
use std::collections::HashMap;
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn config_export(
    output_path: String,
    db: State<'_, Database>,
) -> Result<String, AppError> {
    export_config(&db, Path::new(&output_path))
}

#[tauri::command]
pub async fn config_import_preview(file_path: String) -> Result<ImportPreviewResult, AppError> {
    preview_import_config(Path::new(&file_path))
}

#[tauri::command]
pub async fn config_import_apply(
    file_path: String,
    path_mappings: HashMap<String, String>,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    apply_import_config(&db, Path::new(&file_path), path_mappings)
}
