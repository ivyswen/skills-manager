use crate::core::git_service::{
    check_is_git_repo, execute_git_pull, get_git_remote, get_git_status,
};
use crate::core::scanner::scan_repository;
use crate::db::Database;
use crate::models::{
    AppError, GitPullResult, GitStatusResult, OperationLog, Repository, ScanResult,
};
use chrono::Utc;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn repository_get(db: State<'_, Database>) -> Result<Option<Repository>, AppError> {
    db.get_repository()
}

#[tauri::command]
pub async fn repository_add(
    path: String,
    name: Option<String>,
    db: State<'_, Database>,
) -> Result<Repository, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::path_not_found("指定的目录不存在或不是有效目录"));
    }

    let repo_name = name.unwrap_or_else(|| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Central Repository".to_string())
    });

    let canonical_path = dunce::canonicalize(p)
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.clone());

    // PRD 6.1.1: 已登记路径不能重复添加
    if let Some(existing) = db.get_repository()? {
        #[cfg(windows)]
        let is_same = existing.path.eq_ignore_ascii_case(&canonical_path);
        #[cfg(not(windows))]
        let is_same = existing.path == canonical_path;

        if is_same {
            return Err(AppError::new(
                "ALREADY_EXISTS",
                "该中央仓库已登记，不能重复添加",
            ));
        }
    }

    let is_git = check_is_git_repo(Path::new(&canonical_path));
    let source_type = if is_git { "git" } else { "local" }.to_string();
    let git_remote = if is_git {
        get_git_remote(Path::new(&canonical_path))
    } else {
        None
    };
    let current_branch = if is_git {
        get_git_status(Path::new(&canonical_path))
            .ok()
            .map(|s| s.branch)
    } else {
        None
    };

    let repo = Repository {
        id: Uuid::new_v4().to_string(),
        name: repo_name,
        path: canonical_path.clone(),
        source_type,
        git_remote,
        current_branch,
        last_scanned_at: Some(Utc::now().to_rfc3339()),
    };

    // 保存仓库信息
    db.save_repository(&repo)?;

    // 立即执行一次扫描
    let (skills, _) = scan_repository(&canonical_path, &repo.id)?;
    db.upsert_skills(&skills)?;

    // 记录日志
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "repo_add".to_string(),
        entity_type: "repository".to_string(),
        entity_id: Some(repo.id.clone()),
        project_id: None,
        skill_name: None,
        target_path: Some(canonical_path),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("成功添加中央仓库，并扫描入库 {} 个 Skill", skills.len()),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(repo)
}

#[tauri::command]
pub async fn repository_scan(db: State<'_, Database>) -> Result<ScanResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未登记中央仓库"))?;

    crate::core::logger::info("Scanner", &format!("开始扫描中央仓库: {}", repo.path));
    let (skills, warnings) = scan_repository(&repo.path, &repo.id)?;
    db.upsert_skills(&skills)?;

    let mut updated_repo = repo;
    updated_repo.last_scanned_at = Some(Utc::now().to_rfc3339());
    db.save_repository(&updated_repo)?;

    // 关键修复：必须从数据库中取出实际持久化具有稳定 ID 的 Skill 记录返回给前端
    let saved_skills = db.get_skills(&updated_repo.id)?;
    crate::core::logger::info(
        "Scanner",
        &format!(
            "仓库扫描完成，共持久化 {} 个 Skill，警告: {}",
            saved_skills.len(),
            warnings.len()
        ),
    );

    Ok(ScanResult {
        skills: saved_skills,
        warnings,
    })
}

#[tauri::command]
pub async fn repository_git_status(db: State<'_, Database>) -> Result<GitStatusResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未登记中央仓库"))?;

    get_git_status(Path::new(&repo.path))
}

#[tauri::command]
pub async fn repository_git_pull(db: State<'_, Database>) -> Result<GitPullResult, AppError> {
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未登记中央仓库"))?;

    let mut pull_res = execute_git_pull(Path::new(&repo.path))?;

    // 如果有被删除或重命名的 Skill，检索受影响的项目挂载
    if pull_res.updated {
        let all_mounts = db.get_all_mounts()?;
        for affected in &mut pull_res.affected_skills {
            let mut affected_projects = Vec::new();
            for m in &all_mounts {
                if m.skill_name == affected.name {
                    affected_projects.push(m.project_id.clone());
                }
            }
            affected.affected_project_ids = affected_projects;
        }

        // 重新扫描入库
        let (skills, _) = scan_repository(&repo.path, &repo.id)?;
        db.upsert_skills(&skills)?;

        // 记录日志
        let log = OperationLog {
            id: Uuid::new_v4().to_string(),
            batch_id: None,
            operation_type: "git_pull".to_string(),
            entity_type: "repository".to_string(),
            entity_id: Some(repo.id.clone()),
            project_id: None,
            skill_name: None,
            target_path: Some(repo.path.clone()),
            backup_path: None,
            status: "SUCCESS".to_string(),
            error_code: None,
            message: format!(
                "Git 拉取成功: {} -> {}",
                pull_res.old_commit, pull_res.new_commit
            ),
            created_at: Utc::now().to_rfc3339(),
        };
        let _ = db.insert_log(&log);
    }

    Ok(pull_res)
}
