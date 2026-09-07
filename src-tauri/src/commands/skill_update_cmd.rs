use crate::core::skill_update_service;
use crate::db::Database;
use crate::models::{
    AppError, BatchUpdateResult, SkillBackupItem, SkillUpdateInfo, SkillUpdateResult,
};
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn skill_check_updates(
    skill_name: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<SkillUpdateInfo>, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    skill_update_service::check_skill_updates(Path::new(&repo.path), skill_name.as_deref(), &db)
        .await
}

#[tauri::command]
pub async fn skill_update_single(
    skill_name: String,
    db: State<'_, Database>,
) -> Result<SkillUpdateResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    skill_update_service::update_single_skill(Path::new(&repo.path), &skill_name, &db).await
}

#[tauri::command]
pub async fn skill_update_batch(
    skill_names: Vec<String>,
    db: State<'_, Database>,
) -> Result<BatchUpdateResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    skill_update_service::batch_update_skills(Path::new(&repo.path), &skill_names, &db).await
}

#[tauri::command]
pub async fn skill_backup_list(
    skill_name: Option<String>,
    db: State<'_, Database>,
) -> Result<Vec<SkillBackupItem>, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    skill_update_service::list_skill_backups(Path::new(&repo.path), skill_name.as_deref())
}

#[tauri::command]
pub async fn skill_backup_restore(
    backup_id: String,
    db: State<'_, Database>,
) -> Result<SkillUpdateResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    skill_update_service::restore_skill_backup(Path::new(&repo.path), &backup_id, &db)
}

#[tauri::command]
pub async fn skill_backup_delete(
    backup_id: String,
    db: State<'_, Database>,
) -> Result<bool, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    skill_update_service::delete_skill_backup(Path::new(&repo.path), &backup_id)
}
