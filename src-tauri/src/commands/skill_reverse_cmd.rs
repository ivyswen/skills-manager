use crate::core::skill_reverse_service;
use crate::db::Database;
use crate::models::{
    AppError, MountResult, ReversePushRequest, ReversePushResult, SkillDiffResult,
};
use std::path::Path;
use tauri::State;

#[tauri::command]
pub async fn skill_inspect_reverse_diff(
    project_id: String,
    skill_name: String,
    db: State<'_, Database>,
) -> Result<SkillDiffResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    let project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    skill_reverse_service::inspect_skill_diff(
        Path::new(&project.path),
        Path::new(&repo.path),
        &skill_name,
        &db,
    )
}

#[tauri::command]
pub async fn skill_execute_reverse_push(
    request: ReversePushRequest,
    db: State<'_, Database>,
) -> Result<ReversePushResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    let project = db
        .get_project_by_id(&request.project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    skill_reverse_service::execute_reverse_push(
        Path::new(&project.path),
        Path::new(&repo.path),
        &request,
        &db,
    )
}

#[tauri::command]
pub async fn skill_import_unmanaged(
    project_id: String,
    dir_name: String,
    preferred_mode: Option<String>,
    db: State<'_, Database>,
) -> Result<MountResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    let project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    skill_reverse_service::execute_import_unmanaged(
        Path::new(&project.path),
        Path::new(&repo.path),
        &project_id,
        &dir_name,
        preferred_mode.as_deref(),
        &db,
    )
}
