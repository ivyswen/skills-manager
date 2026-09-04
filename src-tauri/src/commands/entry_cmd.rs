use crate::core::entry_engine::{check_entry_link_status, remove_entry_link, setup_entry_link};
use crate::db::Database;
use crate::models::{AppError, EntryLink};
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn entry_link_setup(
    project_id: String,
    agent_type: String,
    link_path: String,
    preferred_mode: Option<String>,
    db: State<'_, Database>,
) -> Result<EntryLink, AppError> {
    let project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    let targets = db.get_agent_targets(&project_id)?;
    let target = targets
        .into_iter()
        .find(|t| t.agent_type == agent_type)
        .ok_or_else(|| AppError::new("AGENT_NOT_FOUND", "未找到对应的 Agent 配置"))?;

    setup_entry_link(
        Path::new(&project.path),
        &link_path,
        preferred_mode.as_deref(),
    )?;

    let status = check_entry_link_status(Path::new(&project.path), &link_path);

    let entry_link = EntryLink {
        id: Uuid::new_v4().to_string(),
        agent_target_id: target.id,
        link_path: link_path.clone(),
        target_path: ".agents/skills".to_string(),
        status,
    };

    db.upsert_entry_link(&entry_link)?;

    let log = crate::models::OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "entry_link_setup".to_string(),
        entity_type: "entry_link".to_string(),
        entity_id: Some(entry_link.id.clone()),
        project_id: Some(project.id.clone()),
        skill_name: None,
        target_path: Some(link_path),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("已配置 Agent 入口软链接: {}", entry_link.link_path),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(entry_link)
}

#[tauri::command]
pub async fn entry_link_remove(
    entry_link_id: String,
    project_id: String,
    link_path: String,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    let project = db
        .get_project_by_id(&project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    remove_entry_link(Path::new(&project.path), &link_path)?;
    db.delete_entry_link(&entry_link_id)?;

    let log = crate::models::OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "entry_link_remove".to_string(),
        entity_type: "entry_link".to_string(),
        entity_id: Some(entry_link_id),
        project_id: Some(project.id.clone()),
        skill_name: None,
        target_path: Some(link_path.clone()),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("已删除 Agent 入口软链接: {}", link_path),
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(())
}
