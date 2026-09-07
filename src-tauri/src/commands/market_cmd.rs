use crate::core::market_service;
use crate::db::Database;
use crate::models::{
    AppError, MarketInstallRequest, MarketInstallResult, MarketSearchResponse, MarketSkillDetail,
    MarketUninstallCheckResult, MarketUninstallRequest, MarketUninstallResult,
};
use std::path::{Path, PathBuf};
use tauri::State;

#[tauri::command]
pub async fn market_search(
    query: String,
    db: State<'_, Database>,
) -> Result<MarketSearchResponse, AppError> {
    let installed_names = if let Ok(Some(repo)) = db.get_repository() {
        db.get_skills(&repo.id)
            .map(|list| list.into_iter().map(|s| s.name).collect())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    market_service::search_skills(&query, &installed_names).await
}

#[tauri::command]
pub async fn market_install(
    req: MarketInstallRequest,
    db: State<'_, Database>,
) -> Result<MarketInstallResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库，请先在主界面添加中央仓库"))?;

    market_service::install_skill(Path::new(&repo.path), req, &db)
}

#[tauri::command]
pub async fn market_uninstall_check(
    skill_name: String,
    db: State<'_, Database>,
) -> Result<MarketUninstallCheckResult, AppError> {
    market_service::check_skill_uninstall(&skill_name, &db)
}

#[tauri::command]
pub async fn market_uninstall(
    req: MarketUninstallRequest,
    db: State<'_, Database>,
) -> Result<MarketUninstallResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    market_service::uninstall_skill(Path::new(&repo.path), req, &db)
}

#[tauri::command]
pub async fn market_skill_detail(
    skill_id: String,
    skill_name: String,
    source: String,
    installs: u64,
    db: State<'_, Database>,
) -> Result<MarketSkillDetail, AppError> {
    let repo_opt = db.get_repository()?;
    let repo_path = repo_opt
        .map(|r| PathBuf::from(r.path))
        .unwrap_or_else(|| PathBuf::from(""));

    market_service::get_skill_detail(&repo_path, &skill_id, &skill_name, &source, installs, &db)
        .await
}
