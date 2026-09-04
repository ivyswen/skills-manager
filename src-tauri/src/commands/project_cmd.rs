use crate::core::entry_engine::check_entry_link_status;
use crate::core::link_engine::remove_mount;
use crate::core::status_checker::diagnose_project_mounts;
use crate::db::Database;
use crate::models::{AgentTarget, AppError, OperationLog, Project, ProjectDiagnostic};
use chrono::Utc;
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn project_list(db: State<'_, Database>) -> Result<Vec<Project>, AppError> {
    db.get_projects()
}

#[tauri::command]
pub async fn project_add(
    path: String,
    name: Option<String>,
    db: State<'_, Database>,
) -> Result<Project, AppError> {
    let p = Path::new(&path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::path_not_found("项目路径不存在或不是有效目录"));
    }

    let canonical_path = dunce::canonicalize(p)
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or_else(|_| path.clone());

    if let Some(existing) = db.get_project_by_path(&canonical_path)? {
        return Ok(existing);
    }

    let proj_name = name.unwrap_or_else(|| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "My Project".to_string())
    });

    let project = Project {
        id: Uuid::new_v4().to_string(),
        name: proj_name,
        path: canonical_path.clone(),
        last_scanned_at: Some(Utc::now().to_rfc3339()),
        is_archived: false,
    };

    // 预先创建 .agents/skills 目录
    let skills_dir = Path::new(&canonical_path).join(".agents").join("skills");
    if !skills_dir.exists() {
        let _ = fs::create_dir_all(&skills_dir);
    }

    db.save_project(&project)?;

    // 默认创建 claude_code 目标
    let target = AgentTarget {
        id: Uuid::new_v4().to_string(),
        project_id: project.id.clone(),
        agent_type: "claude_code".to_string(),
        install_dir: ".agents/skills".to_string(),
        link_mode: "symlink".to_string(),
    };
    let _ = db.upsert_agent_target(&target);

    // 记录审计日志
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "project_add".to_string(),
        entity_type: "project".to_string(),
        entity_id: Some(project.id.clone()),
        project_id: Some(project.id.clone()),
        skill_name: None,
        target_path: Some(canonical_path),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("已添加项目: {}", project.name),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);
    crate::core::logger::info(
        "Project",
        &format!("成功添加项目: {} (路径: {})", project.name, project.path),
    );

    Ok(project)
}

#[tauri::command]
pub async fn project_relocate(
    project_id: String,
    new_path: String,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    let p = Path::new(&new_path);
    if !p.exists() || !p.is_dir() {
        return Err(AppError::path_not_found(
            "重定位的新路径不存在或不是有效目录",
        ));
    }

    let canonical = dunce::canonicalize(p)
        .map(|c| c.to_string_lossy().to_string())
        .unwrap_or_else(|_| new_path.clone());

    let old_project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    db.update_project_path(&project_id, &canonical)?;

    // 修复已有挂载记录的 link_path
    let mounts = db.get_mounts_by_project(&project_id)?;
    for mut m in mounts {
        let new_link = Path::new(&canonical)
            .join(".agents")
            .join("skills")
            .join(&m.skill_name);
        m.link_path = new_link.to_string_lossy().to_string();
        let _ = db.upsert_mount(&m);
    }

    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "project_relocate".to_string(),
        entity_type: "project".to_string(),
        entity_id: Some(project_id.clone()),
        project_id: Some(project_id),
        skill_name: None,
        target_path: Some(canonical.clone()),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("项目路径已重定位: {} -> {}", old_project.path, canonical),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(())
}

#[tauri::command]
pub async fn project_remove(
    project_id: String,
    cleanup_links: bool,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    let project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    if cleanup_links {
        let mounts = db.get_mounts_by_project(&project_id)?;
        let agents_skills_dir = Path::new(&project.path).join(".agents").join("skills");
        for m in mounts {
            let p = Path::new(&m.link_path);
            let _ = remove_mount(p, &agents_skills_dir, &m.mount_mode);
        }
    }

    db.delete_project(&project_id)?;

    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "project_remove".to_string(),
        entity_type: "project".to_string(),
        entity_id: Some(project_id.clone()),
        project_id: Some(project_id),
        skill_name: None,
        target_path: Some(project.path),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("已移除项目登记 (清理链接: {})", cleanup_links),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(())
}

#[tauri::command]
pub async fn project_diagnose(
    project_id: String,
    db: State<'_, Database>,
) -> Result<ProjectDiagnostic, AppError> {
    let project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    let p_path = Path::new(&project.path);
    let is_valid = p_path.exists() && p_path.is_dir();

    let db_mounts = db.get_mounts_by_project(&project_id)?;
    let repo_id = db.get_repository()?.map(|r| r.id).unwrap_or_default();
    let skills = db.get_skills(&repo_id)?;

    let diag = diagnose_project_mounts(p_path, &db_mounts, &skills);

    // 同步诊断后的状态到 DB
    for m in &diag.updated_mounts {
        let _ = db.update_mount_status(&m.id, &m.status);
    }

    // 检查入口链接
    let targets = db.get_agent_targets(&project_id)?;
    let mut entry_links = Vec::new();
    for t in targets {
        let links = db.get_entry_links(&t.id)?;
        for mut l in links {
            let status = check_entry_link_status(p_path, &l.link_path);
            l.status = status;
            let _ = db.upsert_entry_link(&l);
            entry_links.push(l);
        }
    }

    Ok(ProjectDiagnostic {
        project_id,
        mounts: diag.updated_mounts,
        unmanaged_dirs: diag.unmanaged_dirs,
        entry_links,
        is_valid,
    })
}

#[tauri::command]
pub async fn open_path_in_explorer(path: String) -> Result<(), AppError> {
    let p = Path::new(&path);
    if !p.exists() {
        crate::core::logger::warn("App", &format!("尝试打开不存在的路径: {}", path));
        return Err(AppError::path_not_found(format!(
            "指定的路径不存在: {}",
            path
        )));
    }

    crate::core::logger::info("App", &format!("在系统文件管理器中打开路径: {}", path));

    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&path).spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = open::that(&path);
    }
    Ok(())
}
