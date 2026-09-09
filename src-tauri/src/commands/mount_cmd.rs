use crate::core::link_engine::{create_mount, remove_mount};
use crate::db::Database;
use crate::models::{
    AppError, ConflictStrategy, Mount, MountRequest, MountResult, MountStatus, OperationLog,
    UnmountResult,
};
use chrono::Utc;
use std::fs;
use std::path::Path;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn mount_skills(
    request: MountRequest,
    db: State<'_, Database>,
) -> Result<Vec<MountResult>, AppError> {
    let project = db
        .get_project_by_id(&request.project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;

    let batch_id = Uuid::new_v4().to_string();
    let mut results = Vec::new();
    let agents_skills_dir = Path::new(&project.path).join(".agents").join("skills");

    crate::core::logger::info(
        "Mount",
        &format!(
            "收到批量挂载请求: 批次 [{}], 项目 '{}' ({}), 请求 Skill 数量: {}",
            batch_id,
            project.name,
            project.path,
            request.skill_ids.len()
        ),
    );

    for skill_id in &request.skill_ids {
        let mut skill_opt = db.get_skill_by_id(skill_id)?;
        if skill_opt.is_none() {
            // 防御性增强：若 ID 未能直接命中（例如前端持有旧 UUID），尝试在当前中央仓库中按 name 匹配
            if let Ok(Some(repo)) = db.get_repository() {
                if let Ok(Some(s)) = db.get_skill_by_name(&repo.id, skill_id) {
                    crate::core::logger::info(
                        "Mount",
                        &format!("通过名称兜底命中 Skill: '{}' (ID: {})", s.name, s.id),
                    );
                    skill_opt = Some(s);
                } else if let Ok(all_skills) = db.get_skills(&repo.id) {
                    if let Some(s) = all_skills
                        .into_iter()
                        .find(|s| s.name == *skill_id || s.id == *skill_id)
                    {
                        crate::core::logger::info(
                            "Mount",
                            &format!(
                                "通过列表全量遍历兜底命中 Skill: '{}' (ID: {})",
                                s.name, s.id
                            ),
                        );
                        skill_opt = Some(s);
                    }
                }
            }
        }

        if skill_opt.is_none() {
            crate::core::logger::error(
                "Mount",
                &format!("未找到指定的 Skill: ID 或标识为 '{}'", skill_id),
            );
            results.push(MountResult {
                skill_id: skill_id.clone(),
                skill_name: "Unknown".to_string(),
                success: false,
                status: "FAILED".to_string(),
                mount_mode: "".to_string(),
                error_code: Some("SKILL_NOT_FOUND".to_string()),
                error_message: Some("未找到指定的 Skill".to_string()),
                backup_path: None,
                batch_id: Some(batch_id.clone()),
            });
            continue;
        }

        let skill = skill_opt.unwrap();
        let target_link = agents_skills_dir.join(&skill.name);
        crate::core::logger::info(
            "Mount",
            &format!(
                "正在创建挂载: Skill '{}' -> '{}'",
                skill.name,
                target_link.display()
            ),
        );

        let mount_res = create_mount(
            Path::new(&skill.canonical_path),
            &target_link,
            request.preferred_mode.as_deref(),
            &request.conflict_strategy,
        );

        match mount_res {
            Ok((actual_mode, backup_opt)) => {
                let backup_str = backup_opt.as_ref().map(|p| p.to_string_lossy().to_string());
                let mount = Mount {
                    id: Uuid::new_v4().to_string(),
                    skill_id: skill.id.clone(),
                    skill_name: skill.name.clone(),
                    project_id: project.id.clone(),
                    link_path: target_link.to_string_lossy().to_string(),
                    resolved_target: skill.canonical_path.clone(),
                    mount_mode: actual_mode.clone(),
                    status: MountStatus::Normal.as_str().to_string(),
                    managed_by_app: true,
                    backup_path: backup_str.clone(),
                    content_hash: Some(skill.content_hash.clone()),
                    is_outdated: Some(false),
                    has_local_changes: Some(false),
                    has_conflict: Some(false),
                    created_at: Utc::now().to_rfc3339(),
                };

                db.upsert_mount(&mount)?;

                let log = OperationLog {
                    id: Uuid::new_v4().to_string(),
                    batch_id: Some(batch_id.clone()),
                    operation_type: "mount".to_string(),
                    entity_type: "mount".to_string(),
                    entity_id: Some(mount.id.clone()),
                    project_id: Some(project.id.clone()),
                    skill_name: Some(skill.name.clone()),
                    target_path: Some(target_link.to_string_lossy().to_string()),
                    backup_path: backup_str.clone(),
                    status: "SUCCESS".to_string(),
                    error_code: None,
                    message: format!("挂载成功 (模式: {})", actual_mode),
                    created_at: Utc::now().to_rfc3339(),
                };
                let _ = db.insert_log(&log);
                crate::core::logger::info(
                    "Mount",
                    &format!(
                        "挂载成功: Skill '{}' -> '{}' (模式: {})",
                        skill.name,
                        target_link.display(),
                        actual_mode
                    ),
                );

                results.push(MountResult {
                    skill_id: skill.id,
                    skill_name: skill.name,
                    success: true,
                    status: MountStatus::Normal.as_str().to_string(),
                    mount_mode: actual_mode,
                    error_code: None,
                    error_message: None,
                    backup_path: backup_str,
                    batch_id: Some(batch_id.clone()),
                });
            }
            Err(e) => {
                crate::core::logger::error(
                    "Mount",
                    &format!(
                        "挂载失败: Skill '{}' -> '{}', 错误码: {}, 错误信息: {}",
                        skill.name,
                        target_link.display(),
                        e.code,
                        e.message
                    ),
                );
                let log = OperationLog {
                    id: Uuid::new_v4().to_string(),
                    batch_id: Some(batch_id.clone()),
                    operation_type: "mount".to_string(),
                    entity_type: "mount".to_string(),
                    entity_id: None,
                    project_id: Some(project.id.clone()),
                    skill_name: Some(skill.name.clone()),
                    target_path: Some(target_link.to_string_lossy().to_string()),
                    backup_path: None,
                    status: "FAILED".to_string(),
                    error_code: Some(e.code.clone()),
                    message: format!("挂载失败: {}", e.message),
                    created_at: Utc::now().to_rfc3339(),
                };
                let _ = db.insert_log(&log);

                results.push(MountResult {
                    skill_id: skill.id,
                    skill_name: skill.name,
                    success: false,
                    status: "FAILED".to_string(),
                    mount_mode: "".to_string(),
                    error_code: Some(e.code),
                    error_message: Some(e.message),
                    backup_path: None,
                    batch_id: Some(batch_id.clone()),
                });
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn unmount_skills(
    mount_ids: Vec<String>,
    force: Option<bool>,
    db: State<'_, Database>,
) -> Result<Vec<UnmountResult>, AppError> {
    let mut results = Vec::new();

    for mid in &mount_ids {
        let mount_opt = db.get_mount_by_id(mid)?;
        if mount_opt.is_none() {
            results.push(UnmountResult {
                mount_id: mid.clone(),
                skill_name: "".to_string(),
                success: false,
                error_code: Some("NOT_FOUND".to_string()),
                error_message: Some("未找到指定的挂载记录".to_string()),
            });
            continue;
        }

        let mount = mount_opt.unwrap();
        let project_opt = db.get_project_by_id(&mount.project_id)?;
        if project_opt.is_none() {
            let _ = db.delete_mount(mid);
            results.push(UnmountResult {
                mount_id: mid.clone(),
                skill_name: mount.skill_name,
                success: true,
                error_code: None,
                error_message: None,
            });
            continue;
        }

        let project = project_opt.unwrap();
        let agents_skills = Path::new(&project.path).join(".agents").join("skills");
        let target_path = Path::new(&mount.link_path);

        let unmount_res = remove_mount(target_path, &agents_skills, &mount.mount_mode);

        match unmount_res {
            Ok(_) => {
                db.delete_mount(&mount.id)?;

                let log = OperationLog {
                    id: Uuid::new_v4().to_string(),
                    batch_id: None,
                    operation_type: "unmount".to_string(),
                    entity_type: "mount".to_string(),
                    entity_id: Some(mount.id.clone()),
                    project_id: Some(project.id.clone()),
                    skill_name: Some(mount.skill_name.clone()),
                    target_path: Some(mount.link_path.clone()),
                    backup_path: None,
                    status: "SUCCESS".to_string(),
                    error_code: None,
                    message: format!("已成功卸载 Skill 挂载: {}", mount.skill_name),
                    created_at: Utc::now().to_rfc3339(),
                };
                let _ = db.insert_log(&log);

                results.push(UnmountResult {
                    mount_id: mount.id,
                    skill_name: mount.skill_name,
                    success: true,
                    error_code: None,
                    error_message: None,
                });
            }
            Err(e) => {
                if force.unwrap_or(false) {
                    // 强制清理记录
                    let _ = db.delete_mount(&mount.id);
                }
                results.push(UnmountResult {
                    mount_id: mount.id,
                    skill_name: mount.skill_name,
                    success: false,
                    error_code: Some(e.code),
                    error_message: Some(e.message),
                });
            }
        }
    }

    Ok(results)
}

#[tauri::command]
pub async fn mount_repair(
    mount_id: String,
    preferred_mode: Option<String>,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    let mount = db
        .get_mount_by_id(&mount_id)?
        .ok_or_else(|| AppError::new("NOT_FOUND", "挂载记录不存在"))?;

    let skill = db
        .get_skill_by_id(&mount.skill_id)?
        .ok_or_else(|| AppError::new("SKILL_NOT_FOUND", "原件 Skill 不存在，无法修复"))?;

    let target_path = Path::new(&mount.link_path);
    let project = db
        .get_project_by_id(&mount.project_id)?
        .ok_or_else(|| AppError::new("PROJECT_NOT_FOUND", "项目不存在"))?;
    let agents_skills = Path::new(&project.path).join(".agents").join("skills");

    // 先安全清理旧链接
    let _ = remove_mount(target_path, &agents_skills, &mount.mount_mode);

    // 重新创建挂载
    let (actual_mode, backup_opt) = create_mount(
        Path::new(&skill.canonical_path),
        target_path,
        preferred_mode.as_deref().or(Some(&mount.mount_mode)),
        &ConflictStrategy::BackupAndReplace,
    )?;

    let backup_str = backup_opt.as_ref().map(|p| p.to_string_lossy().to_string());
    let mut updated = mount;
    updated.mount_mode = actual_mode.clone();
    updated.status = MountStatus::Normal.as_str().to_string();
    updated.resolved_target = skill.canonical_path;
    updated.content_hash = Some(skill.content_hash);
    if backup_str.is_some() {
        updated.backup_path = backup_str.clone();
    }
    db.upsert_mount(&updated)?;

    // 记录审计日志
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "repair".to_string(),
        entity_type: "mount".to_string(),
        entity_id: Some(updated.id.clone()),
        project_id: Some(project.id.clone()),
        skill_name: Some(updated.skill_name.clone()),
        target_path: Some(updated.link_path.clone()),
        backup_path: backup_str,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!(
            "已成功修复挂载: {} (模式: {})",
            updated.skill_name, actual_mode
        ),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(())
}

#[tauri::command]
pub async fn mount_rollback_batch(
    batch_id: String,
    db: State<'_, Database>,
) -> Result<(), AppError> {
    use crate::core::link_engine::is_reparse_point;
    let logs = db.get_logs_by_batch(&batch_id)?;
    if logs.is_empty() {
        return Err(AppError::new("BATCH_NOT_FOUND", "未找到指定批次的操作日志"));
    }

    let mut failed_rollbacks = Vec::new();

    for log in logs {
        if log.operation_type == "mount" && log.status == "SUCCESS" {
            if let Some(target_str) = &log.target_path {
                let target = Path::new(target_str);
                // 删除创建的目标（符号链接/Junction/Copy 目录）
                if is_reparse_point(target) {
                    #[cfg(windows)]
                    {
                        if junction::exists(target).unwrap_or(false) {
                            let _ = junction::delete(target);
                        }
                    }
                    let _ = fs::remove_dir(target);
                    let _ = fs::remove_file(target);
                } else if target.exists() {
                    if target.is_dir() {
                        let _ = fs::remove_dir_all(target);
                    } else {
                        let _ = fs::remove_file(target);
                    }
                }

                // 如果有备份，移回原位置
                if let Some(backup_str) = &log.backup_path {
                    let bpath = Path::new(backup_str);
                    if bpath.exists() {
                        if let Err(e) = fs::rename(bpath, target) {
                            failed_rollbacks.push(format!("{}: {}", target_str, e));
                        }
                    }
                }
            }

            if let Some(mid) = &log.entity_id {
                let _ = db.delete_mount(mid);
            }
        }
    }

    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: Some(batch_id.clone()),
        operation_type: "rollback".to_string(),
        entity_type: "batch".to_string(),
        entity_id: Some(batch_id.clone()),
        project_id: None,
        skill_name: None,
        target_path: None,
        backup_path: None,
        status: if failed_rollbacks.is_empty() {
            "SUCCESS".to_string()
        } else {
            "FAILED".to_string()
        },
        error_code: if failed_rollbacks.is_empty() {
            None
        } else {
            Some("ROLLBACK_FAILED".to_string())
        },
        message: if failed_rollbacks.is_empty() {
            format!("成功回滚批次操作: {}", batch_id)
        } else {
            format!("批次回滚部分失败: {}", failed_rollbacks.join("; "))
        },
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    if !failed_rollbacks.is_empty() {
        return Err(AppError::new(
            "ROLLBACK_FAILED",
            format!("部分操作未能完整回滚: {}", failed_rollbacks.join("; ")),
        ));
    }

    Ok(())
}
