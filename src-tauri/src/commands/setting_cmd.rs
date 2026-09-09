use crate::db::Database;
use crate::models::{AppError, AppSetting};
use tauri::State;

#[tauri::command]
pub async fn app_setting_get(
    key: String,
    db: State<'_, Database>,
) -> Result<Option<String>, AppError> {
    db.get_setting(&key)
}

#[tauri::command]
pub async fn app_setting_set(
    key: String,
    value: String,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    db.set_setting(&key, &value)
}

#[tauri::command]
pub async fn app_setting_get_all(db: State<'_, Database>) -> Result<Vec<AppSetting>, AppError> {
    db.get_all_settings()
}
