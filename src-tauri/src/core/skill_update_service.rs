use crate::core::link_engine::jail_check;
use crate::core::market_service::{
    copy_skill_dir_clean, find_skill_dir_recursive, remove_dir_all_force, sanitize_skill_name,
};
use crate::core::scanner::{measure_skill_dir, scan_repository};
use crate::db::Database;
use crate::models::{
    AppError, BatchUpdateResult, OperationLog, Skill, SkillBackupItem, SkillMetaFile,
    SkillUpdateInfo, SkillUpdateResult,
};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

pub const SKILL_BACKUP_RETAIN_COUNT: usize = 20;

/// 将各种形式的开源源规范化为 Git 克隆 URL
pub fn resolve_clone_url(source: &str) -> String {
    let s = source.trim();
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("git@") {
        if s.ends_with(".git") {
            s.to_string()
        } else {
            format!("{}.git", s)
        }
    } else {
        let stripped = s.strip_prefix("github.com/").unwrap_or(s);
        format!("https://github.com/{}.git", stripped)
    }
}

/// 严格校验备份标识符安全性，杜绝目录穿越与意外清空风险
pub fn validate_backup_id(backup_id: &str) -> Result<(), AppError> {
    let trimmed = backup_id.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(AppError::new(
            "INVALID_BACKUP_ID",
            format!("非法备份标识符: {}", backup_id),
        ));
    }
    Ok(())
}

/// 执行安全浅克隆（包含超时、换行一致性与防挂起策略）
fn execute_shallow_clone(clone_url: &str, dest_path: &Path) -> Result<(), AppError> {
    let output = Command::new("git")
        .arg("-c")
        .arg("core.autocrlf=false")
        .arg("-c")
        .arg("http.connectTimeout=15")
        .arg("-c")
        .arg("http.lowSpeedLimit=1000")
        .arg("-c")
        .arg("http.lowSpeedTime=20")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(clone_url)
        .arg(dest_path)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GCM_INTERACTIVE", "never")
        .env("GIT_ASKPASS", "")
        .output()
        .map_err(|e| {
            AppError::with_details(
                "GIT_EXEC_ERROR",
                "执行 Git 失败，请检查 Git 是否正确安装并在 PATH 中",
                e.to_string(),
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(AppError::with_details(
            "GIT_CLONE_FAILED",
            "克隆技能开源仓库失败，请检查网络连接或仓库状态",
            stderr,
        ));
    }
    Ok(())
}

/// 同步缺失上游源（source）信息的历史技能
pub fn sync_missing_skill_sources(
    central_repo_path: &Path,
    repo_id: &str,
    db: &Database,
) -> Result<(), AppError> {
    let skills = db.get_skills(repo_id)?;
    for skill in skills {
        if skill.source.is_none() {
            let meta_path = central_repo_path
                .join("skills")
                .join(&skill.name)
                .join(".skill-meta.json");
            if meta_path.exists() {
                if let Ok(c) = fs::read_to_string(&meta_path) {
                    if let Ok(meta) = serde_json::from_str::<SkillMetaFile>(&c) {
                        let _ =
                            db.set_skill_source(repo_id, &skill.name, &meta.source, &meta.skill_id);
                        continue;
                    }
                }
            }

            // 尝试从 operation_logs 历史日志提取
            if let Ok(Some((src, sk_id))) = db.get_skill_source_from_logs(&skill.name) {
                let _ = db.set_skill_source(repo_id, &skill.name, &src, &sk_id);
                let meta = SkillMetaFile {
                    name: skill.name.clone(),
                    source: src,
                    skill_id: sk_id,
                    installed_at: skill.last_seen_at.clone(),
                    updated_at: None,
                };
                let _ = fs::write(
                    meta_path,
                    serde_json::to_string_pretty(&meta).unwrap_or_default(),
                );
            }
        }
    }
    Ok(())
}

/// 检查中央仓库技能更新（按源仓库聚合克隆，避免 Monorepo 重复网络开销）
pub async fn check_skill_updates(
    central_repo_path: &Path,
    target_skill_name: Option<&str>,
    db: &Database,
) -> Result<Vec<SkillUpdateInfo>, AppError> {
    let repo_opt = db.get_repository()?;
    let repo = repo_opt.ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    // 自动补齐历史技能缺失的 source 属性
    let _ = sync_missing_skill_sources(central_repo_path, &repo.id, db);

    let all_skills = db.get_skills(&repo.id)?;
    let mut skills_to_check: Vec<Skill> = Vec::new();

    for s in all_skills {
        if let Some(target) = target_skill_name {
            if !s.name.eq_ignore_ascii_case(target) {
                continue;
            }
        }
        if s.source.is_some() {
            skills_to_check.push(s);
        }
    }

    if skills_to_check.is_empty() {
        return Ok(Vec::new());
    }

    // 按上游仓库 source 分组，避免同一 Monorepo 多次重复浅克隆
    let mut repo_groups: HashMap<String, Vec<Skill>> = HashMap::new();
    for s in skills_to_check {
        if let Some(ref src) = s.source {
            repo_groups.entry(src.clone()).or_default().push(s);
        }
    }

    let mut update_infos = Vec::new();

    for (source, group) in repo_groups {
        let sandbox_id = format!("skills_update_chk_{}", Uuid::new_v4());
        let temp_sandbox = std::env::temp_dir().join(sandbox_id);
        let _ = fs::create_dir_all(&temp_sandbox);

        let clone_url = resolve_clone_url(&source);
        crate::core::logger::info(
            "SkillUpdate",
            &format!(
                "检查更新：浅克隆远端仓库 {} -> {}",
                clone_url,
                temp_sandbox.display()
            ),
        );

        if let Err(e) = execute_shallow_clone(&clone_url, &temp_sandbox) {
            crate::core::logger::warn(
                "SkillUpdate",
                &format!("检查更新浅克隆 {} 失败: {e}", clone_url),
            );
            let _ = remove_dir_all_force(&temp_sandbox);
            continue;
        }

        let now_str = Utc::now().to_rfc3339();

        for skill in group {
            let skill_id_to_find = skill.remote_skill_id.as_deref().unwrap_or(&skill.name);

            let found_dir =
                find_skill_dir_recursive(&temp_sandbox, skill_id_to_find, &skill.name, 5);

            match found_dir {
                Ok(remote_dir) => {
                    let (_, _, remote_hash) = measure_skill_dir(&remote_dir);
                    let has_update = remote_hash != skill.content_hash;

                    let _ = db.update_skill_update_status(
                        &repo.id,
                        &skill.name,
                        &remote_hash,
                        has_update,
                        &now_str,
                    );

                    update_infos.push(SkillUpdateInfo {
                        skill_name: skill.name.clone(),
                        source: source.clone(),
                        skill_id: skill_id_to_find.to_string(),
                        current_hash: skill.content_hash.clone(),
                        remote_hash: remote_hash.clone(),
                        has_update,
                        last_checked_at: now_str.clone(),
                    });
                }
                Err(err) => {
                    crate::core::logger::warn(
                        "SkillUpdate",
                        &format!("未在远端仓库中定位到技能 {}: {err}", skill.name),
                    );
                }
            }
        }

        let _ = remove_dir_all_force(&temp_sandbox);
    }

    Ok(update_infos)
}

/// 批量更新技能（按仓库聚合浅克隆，严格保障网络失败时不产生多余快照与破坏）
pub async fn batch_update_skills(
    central_repo_path: &Path,
    skill_names: &[String],
    db: &Database,
) -> Result<BatchUpdateResult, AppError> {
    let repo_opt = db.get_repository()?;
    let repo = repo_opt.ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    // 若未指定技能名单，则更新全部标记为 has_update 的技能
    let target_skills: Vec<Skill> = if skill_names.is_empty() {
        let skills = db.get_skills(&repo.id)?;
        skills
            .into_iter()
            .filter(|s| s.has_update == Some(true) && s.source.is_some())
            .collect()
    } else {
        let mut list = Vec::new();
        for name in skill_names {
            let raw = name.trim();
            let safe = sanitize_skill_name(raw);
            if let Some(s) = db
                .get_skill_by_name(&repo.id, raw)?
                .or_else(|| db.get_skill_by_name(&repo.id, &safe).ok().flatten())
            {
                list.push(s);
            } else {
                return Err(AppError::path_not_found(format!(
                    "未在中央仓库找到技能: {name}"
                )));
            }
        }
        list
    };

    let total = target_skills.len();
    if total == 0 {
        return Ok(BatchUpdateResult {
            total: 0,
            success_count: 0,
            failed_count: 0,
            results: Vec::new(),
        });
    }

    // 按 upstream source 分组聚合，同一仓库只浅克隆一次
    let mut repo_groups: HashMap<String, Vec<Skill>> = HashMap::new();
    for s in target_skills {
        let source = s.source.clone().or_else(|| {
            let safe_name = sanitize_skill_name(&s.name);
            let meta_file = central_repo_path
                .join("skills")
                .join(&safe_name)
                .join(".skill-meta.json");
            if let Ok(c) = fs::read_to_string(meta_file) {
                if let Ok(m) = serde_json::from_str::<SkillMetaFile>(&c) {
                    return Some(m.source);
                }
            }
            if let Ok(Some((src, _))) = db.get_skill_source_from_logs(&s.name) {
                return Some(src);
            }
            None
        });

        match source {
            Some(src) => {
                repo_groups.entry(src).or_default().push(s);
            }
            None => {
                // 没有记录源的技能记录为失败
                repo_groups
                    .entry("__NO_SOURCE__".to_string())
                    .or_default()
                    .push(s);
            }
        }
    }

    let mut results = Vec::new();
    let mut success_count = 0;
    let mut failed_count = 0;
    let central_skills_dir = central_repo_path.join("skills");

    for (source, group) in repo_groups {
        if source == "__NO_SOURCE__" {
            for skill in group {
                failed_count += 1;
                results.push(SkillUpdateResult {
                    success: false,
                    skill_name: skill.name,
                    old_hash: skill.content_hash,
                    new_hash: String::new(),
                    backup_path: None,
                    message: "未记录上游开源仓库源，无法执行在线更新".to_string(),
                });
            }
            continue;
        }

        let sandbox_id = format!("skills_update_run_{}", Uuid::new_v4());
        let temp_sandbox = std::env::temp_dir().join(sandbox_id);
        let _ = fs::create_dir_all(&temp_sandbox);

        let clone_url = resolve_clone_url(&source);
        crate::core::logger::info(
            "SkillUpdate",
            &format!(
                "批量更新：克隆远端仓库 {} -> {}",
                clone_url,
                temp_sandbox.display()
            ),
        );

        if let Err(e) = execute_shallow_clone(&clone_url, &temp_sandbox) {
            let _ = remove_dir_all_force(&temp_sandbox);
            for skill in group {
                failed_count += 1;
                results.push(SkillUpdateResult {
                    success: false,
                    skill_name: skill.name,
                    old_hash: skill.content_hash,
                    new_hash: String::new(),
                    backup_path: None,
                    message: format!("克隆仓库失败: {}", e.message),
                });
            }
            continue;
        }

        for skill in group {
            let safe_name = sanitize_skill_name(&skill.name);
            let remote_skill_id = skill
                .remote_skill_id
                .clone()
                .unwrap_or_else(|| skill.name.clone());

            let found_dir_res =
                find_skill_dir_recursive(&temp_sandbox, &remote_skill_id, &skill.name, 5);
            let found_dir = match found_dir_res {
                Ok(d) => d,
                Err(err) => {
                    failed_count += 1;
                    results.push(SkillUpdateResult {
                        success: false,
                        skill_name: safe_name,
                        old_hash: skill.content_hash,
                        new_hash: String::new(),
                        backup_path: None,
                        message: format!("在远端仓库中未定位到技能目录: {}", err.message),
                    });
                    continue;
                }
            };

            let target_dest = central_skills_dir.join(&safe_name);

            // Jail 校验
            if let Err(jail_err) = jail_check(&central_skills_dir, &target_dest) {
                failed_count += 1;
                results.push(SkillUpdateResult {
                    success: false,
                    skill_name: safe_name,
                    old_hash: skill.content_hash,
                    new_hash: String::new(),
                    backup_path: None,
                    message: format!("Jail 安全校验失败: {}", jail_err.message),
                });
                continue;
            }

            // 关键：仅在远端源码已成功克隆并验证通过后，方才创建安全备份！
            let backup_path =
                match create_skill_backup(central_repo_path, &safe_name, db, "pre-update") {
                    Ok(p) => p,
                    Err(e) => {
                        failed_count += 1;
                        results.push(SkillUpdateResult {
                            success: false,
                            skill_name: safe_name,
                            old_hash: skill.content_hash,
                            new_hash: String::new(),
                            backup_path: None,
                            message: format!("创建安全备份失败: {}", e.message),
                        });
                        continue;
                    }
                };

            // 安全搬运与覆盖
            if target_dest.exists() {
                if let Err(e) = remove_dir_all_force(&target_dest) {
                    failed_count += 1;
                    results.push(SkillUpdateResult {
                        success: false,
                        skill_name: safe_name,
                        old_hash: skill.content_hash,
                        new_hash: String::new(),
                        backup_path: backup_path.map(|p| p.to_string_lossy().to_string()),
                        message: format!("清空原目录失败: {e}"),
                    });
                    continue;
                }
            }

            if let Err(e) = copy_skill_dir_clean(&found_dir, &target_dest) {
                failed_count += 1;
                results.push(SkillUpdateResult {
                    success: false,
                    skill_name: safe_name,
                    old_hash: skill.content_hash,
                    new_hash: String::new(),
                    backup_path: backup_path.map(|p| p.to_string_lossy().to_string()),
                    message: format!("写入最新技能文件失败: {}", e.message),
                });
                continue;
            }

            let now_str = Utc::now().to_rfc3339();

            // 重新写入最新 .skill-meta.json
            let meta = SkillMetaFile {
                name: safe_name.clone(),
                source: source.clone(),
                skill_id: remote_skill_id.clone(),
                installed_at: skill.last_seen_at.clone(),
                updated_at: Some(now_str.clone()),
            };
            let _ = fs::write(
                target_dest.join(".skill-meta.json"),
                serde_json::to_string_pretty(&meta).unwrap_or_default(),
            );

            let (_, _, new_hash) = measure_skill_dir(&target_dest);
            let _ = db.update_skill_after_update(&repo.id, &safe_name, &new_hash, &now_str);

            let backup_path_str = backup_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string());

            // 记录审计日志
            let log = OperationLog {
                id: Uuid::new_v4().to_string(),
                batch_id: None,
                operation_type: "skill_update".to_string(),
                entity_type: "skill".to_string(),
                entity_id: None,
                project_id: None,
                skill_name: Some(safe_name.clone()),
                target_path: Some(target_dest.to_string_lossy().to_string()),
                backup_path: backup_path_str.clone(),
                status: "SUCCESS".to_string(),
                error_code: None,
                message: format!(
                    "技能 '{}' 更新成功 (旧哈希: {}..., 新哈希: {}...)",
                    safe_name,
                    &skill.content_hash[..skill.content_hash.len().min(8)],
                    &new_hash[..new_hash.len().min(8)]
                ),
                created_at: now_str,
            };
            let _ = db.insert_log(&log);

            success_count += 1;
            results.push(SkillUpdateResult {
                success: true,
                skill_name: safe_name,
                old_hash: skill.content_hash,
                new_hash,
                backup_path: backup_path_str,
                message: "更新成功".to_string(),
            });
        }

        let _ = remove_dir_all_force(&temp_sandbox);
    }

    // 重新扫描中央仓库并统一入库
    let (scanned_skills, _) = scan_repository(&repo.path, &repo.id)?;
    let _ = db.upsert_skills(&scanned_skills);

    Ok(BatchUpdateResult {
        total,
        success_count,
        failed_count,
        results,
    })
}

/// 更新单个技能（直接复用 batch_update_skills 保证安全与行为一致）
pub async fn update_single_skill(
    central_repo_path: &Path,
    skill_name: &str,
    db: &Database,
) -> Result<SkillUpdateResult, AppError> {
    let batch_res = batch_update_skills(central_repo_path, &[skill_name.to_string()], db).await?;
    if let Some(res) = batch_res.results.into_iter().next() {
        if !res.success {
            return Err(AppError::new("UPDATE_FAILED", res.message));
        }
        Ok(res)
    } else {
        Err(AppError::path_not_found(format!(
            "未找到指定技能: {skill_name}"
        )))
    }
}

/// 创建技能安全备份（按 Skill 维度独立清理超过保留数量的旧备份，防止跨技能误删）
pub fn create_skill_backup(
    central_repo_path: &Path,
    skill_name: &str,
    db: &Database,
    reason: &str,
) -> Result<Option<PathBuf>, AppError> {
    let safe_name = sanitize_skill_name(skill_name);
    let skill_dir = central_repo_path.join("skills").join(&safe_name);
    if !skill_dir.exists() {
        return Ok(None);
    }

    let backup_root = central_repo_path.join(".backups");
    fs::create_dir_all(&backup_root).map_err(AppError::io_error)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_id = format!("{}_{}", timestamp, safe_name);
    let mut backup_dir = backup_root.join(&backup_id);
    let mut counter = 1;
    while backup_dir.exists() {
        backup_dir = backup_root.join(format!("{}_{}_{}", timestamp, safe_name, counter));
        counter += 1;
    }

    let skill_backup_dir = backup_dir.join("skill");
    copy_skill_dir_clean(&skill_dir, &skill_backup_dir)?;

    let (_, _, content_hash) = measure_skill_dir(&skill_dir);

    let (source, _) = if let Ok(Some(repo)) = db.get_repository() {
        if let Ok(Some(s)) = db.get_skill_by_name(&repo.id, skill_name) {
            (s.source, s.remote_skill_id)
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    let source = source.or_else(|| {
        let meta_file = skill_dir.join(".skill-meta.json");
        if let Ok(c) = fs::read_to_string(meta_file) {
            if let Ok(m) = serde_json::from_str::<SkillMetaFile>(&c) {
                return Some(m.source);
            }
        }
        None
    });

    let meta = SkillBackupItem {
        id: backup_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string(),
        skill_name: safe_name.clone(),
        source,
        created_at: Utc::now().to_rfc3339(),
        backup_path: backup_dir.to_string_lossy().to_string(),
        content_hash,
        reason: Some(reason.to_string()),
    };

    let meta_json = serde_json::to_string_pretty(&meta).map_err(|e| {
        AppError::with_details("BACKUP_META_ERROR", "序列化备份元数据失败", e.to_string())
    })?;
    fs::write(backup_dir.join("meta.json"), meta_json).map_err(AppError::io_error)?;

    // 按 Skill 独立保留最新的 20 个备份快照
    let _ = cleanup_old_backups_for_skill(&backup_root, &safe_name, SKILL_BACKUP_RETAIN_COUNT);

    Ok(Some(backup_dir))
}

/// 针对特定技能清理旧备份（保留最新的 retain_count 个副本，绝对不影响其他技能）
pub fn cleanup_old_backups_for_skill(
    backup_root: &Path,
    skill_name: &str,
    retain_count: usize,
) -> Result<(), AppError> {
    if !backup_root.exists() {
        return Ok(());
    }
    let safe_name = sanitize_skill_name(skill_name);
    let mut skill_backups = Vec::new();

    for entry in fs::read_dir(backup_root).map_err(AppError::io_error)? {
        let entry = entry.map_err(AppError::io_error)?;
        if !entry.file_type().map_err(AppError::io_error)?.is_dir() {
            continue;
        }
        let p = entry.path();
        let meta_file = p.join("meta.json");
        let matches = if meta_file.exists() {
            if let Ok(c) = fs::read_to_string(&meta_file) {
                if let Ok(meta) = serde_json::from_str::<SkillBackupItem>(&c) {
                    meta.skill_name.eq_ignore_ascii_case(&safe_name)
                        || meta.skill_name.eq_ignore_ascii_case(skill_name)
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            let fname = entry.file_name().to_string_lossy().to_string();
            fname.ends_with(&format!("_{}", safe_name))
        };

        if matches {
            skill_backups.push(p);
        }
    }

    skill_backups.sort();
    if skill_backups.len() > retain_count {
        let remove_count = skill_backups.len() - retain_count;
        for path in skill_backups.iter().take(remove_count) {
            let _ = remove_dir_all_force(path);
        }
    }
    Ok(())
}

/// 查询技能备份列表
pub fn list_skill_backups(
    central_repo_path: &Path,
    skill_name: Option<&str>,
) -> Result<Vec<SkillBackupItem>, AppError> {
    let backup_root = central_repo_path.join(".backups");
    if !backup_root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(&backup_root).map_err(AppError::io_error)? {
        let entry = entry.map_err(AppError::io_error)?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let meta_file = path.join("meta.json");
        if meta_file.exists() {
            if let Ok(content) = fs::read_to_string(&meta_file) {
                if let Ok(meta) = serde_json::from_str::<SkillBackupItem>(&content) {
                    if let Some(target) = skill_name {
                        if !meta.skill_name.eq_ignore_ascii_case(target) {
                            continue;
                        }
                    }
                    items.push(meta);
                }
            }
        }
    }

    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

/// 从安全备份中恢复技能（受 Jail 保护，恢复前双重自动备份）
pub fn restore_skill_backup(
    central_repo_path: &Path,
    backup_id: &str,
    db: &Database,
) -> Result<SkillUpdateResult, AppError> {
    validate_backup_id(backup_id)?;

    let repo_opt = db.get_repository()?;
    let repo = repo_opt.ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    let backup_root = central_repo_path.join(".backups");
    let backup_dir = backup_root.join(backup_id);
    jail_check(&backup_root, &backup_dir)?;

    if !backup_dir.exists() {
        return Err(AppError::path_not_found(format!(
            "指定备份不存在: {}",
            backup_id
        )));
    }

    let meta_file = backup_dir.join("meta.json");
    let meta: SkillBackupItem = fs::read_to_string(&meta_file)
        .map_err(AppError::io_error)
        .and_then(|c| {
            serde_json::from_str(&c).map_err(|e| {
                AppError::with_details("PARSE_ERROR", "解析备份元数据失败", e.to_string())
            })
        })?;

    let safe_name = sanitize_skill_name(&meta.skill_name);
    if safe_name.is_empty() {
        return Err(AppError::new(
            "INVALID_SKILL_NAME",
            "备份元数据中的技能名称无效",
        ));
    }

    let backup_skill_dir = backup_dir.join("skill");
    if !backup_skill_dir.exists() {
        return Err(AppError::path_not_found("备份目录中缺少 skill 实体内容"));
    }

    let central_skills_dir = central_repo_path.join("skills");
    let target_dest = central_skills_dir.join(&safe_name);
    jail_check(&central_skills_dir, &target_dest)?;

    // 恢复前先为当前状态备份（双重安全防御）
    let _ = create_skill_backup(central_repo_path, &safe_name, db, "pre-restore");

    if target_dest.exists() {
        remove_dir_all_force(&target_dest).map_err(AppError::io_error)?;
    }
    copy_skill_dir_clean(&backup_skill_dir, &target_dest)?;

    // 重新扫描入库
    let (skills, _) = scan_repository(&repo.path, &repo.id)?;
    db.upsert_skills(&skills)?;

    let restored_skill = db.get_skill_by_name(&repo.id, &safe_name)?;
    let new_hash = restored_skill
        .map(|s| s.content_hash)
        .unwrap_or_else(|| meta.content_hash.clone());

    let now_str = Utc::now().to_rfc3339();
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "skill_restore".to_string(),
        entity_type: "skill".to_string(),
        entity_id: None,
        project_id: None,
        skill_name: Some(safe_name.clone()),
        target_path: Some(target_dest.to_string_lossy().to_string()),
        backup_path: Some(backup_dir.to_string_lossy().to_string()),
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("已从备份 '{}' 成功恢复技能 '{}'", backup_id, safe_name),
        created_at: now_str,
    };
    let _ = db.insert_log(&log);

    Ok(SkillUpdateResult {
        success: true,
        skill_name: safe_name,
        old_hash: String::new(),
        new_hash,
        backup_path: Some(backup_dir.to_string_lossy().to_string()),
        message: "恢复成功".to_string(),
    })
}

/// 删除指定备份（带严格安全与 Jail 保护）
pub fn delete_skill_backup(central_repo_path: &Path, backup_id: &str) -> Result<bool, AppError> {
    validate_backup_id(backup_id)?;

    let backup_root = central_repo_path.join(".backups");
    let backup_dir = backup_root.join(backup_id);
    jail_check(&backup_root, &backup_dir)?;

    if !backup_dir.exists() {
        return Ok(false);
    }

    remove_dir_all_force(&backup_dir).map_err(AppError::io_error)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_clone_url() {
        assert_eq!(
            resolve_clone_url("owner/repo"),
            "https://github.com/owner/repo.git"
        );
        assert_eq!(
            resolve_clone_url("github.com/owner/repo"),
            "https://github.com/owner/repo.git"
        );
        assert_eq!(
            resolve_clone_url("https://github.com/owner/repo"),
            "https://github.com/owner/repo.git"
        );
        assert_eq!(
            resolve_clone_url("https://github.com/owner/repo.git"),
            "https://github.com/owner/repo.git"
        );
        assert_eq!(
            resolve_clone_url("git@github.com:owner/repo.git"),
            "git@github.com:owner/repo.git"
        );
    }

    #[test]
    fn test_validate_backup_id_security() {
        assert!(validate_backup_id("20260101_120000_my-skill").is_ok());
        assert!(validate_backup_id("").is_err());
        assert!(validate_backup_id("   ").is_err());
        assert!(validate_backup_id(".").is_err());
        assert!(validate_backup_id("..").is_err());
        assert!(validate_backup_id("../etc").is_err());
        assert!(validate_backup_id("..\\etc").is_err());
        assert!(validate_backup_id("foo/bar").is_err());
        assert!(validate_backup_id("foo\\bar").is_err());
    }

    #[test]
    fn test_backup_and_restore_cycle() {
        let temp = std::env::temp_dir().join(format!("test_backup_cycle_{}", Uuid::new_v4()));
        let central_repo = temp.join("repo");
        let skills_dir = central_repo.join("skills");
        let skill_path = skills_dir.join("test-skill");
        fs::create_dir_all(&skill_path).unwrap();
        fs::write(skill_path.join("SKILL.md"), "version 1").unwrap();
        fs::write(skill_path.join("run.sh"), "echo 1").unwrap();

        let db = Database::in_memory().unwrap();
        let repo = crate::models::Repository {
            id: "repo-1".to_string(),
            name: "Test Repo".to_string(),
            path: central_repo.to_string_lossy().to_string(),
            source_type: "local".to_string(),
            git_remote: None,
            current_branch: None,
            last_scanned_at: None,
        };
        db.save_repository(&repo).unwrap();

        // 1. 创建备份
        let backup_path = create_skill_backup(&central_repo, "test-skill", &db, "unit-test")
            .unwrap()
            .unwrap();
        assert!(backup_path.exists());
        assert!(backup_path.join("skill").join("SKILL.md").exists());
        assert!(backup_path.join("meta.json").exists());

        let backup_id = backup_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        // 2. 列出备份
        let backups = list_skill_backups(&central_repo, Some("test-skill")).unwrap();
        assert_eq!(backups.len(), 1);
        assert_eq!(backups[0].skill_name, "test-skill");

        // 3. 修改原始文件
        fs::write(skill_path.join("SKILL.md"), "version 2 modified").unwrap();

        // 4. 从备份恢复
        let res = restore_skill_backup(&central_repo, &backup_id, &db).unwrap();
        assert!(res.success);

        // 验证文件恢复回 version 1
        let content = fs::read_to_string(skill_path.join("SKILL.md")).unwrap();
        assert_eq!(content, "version 1");

        // 5. 删除备份
        let del_res = delete_skill_backup(&central_repo, &backup_id).unwrap();
        assert!(del_res);
        assert!(!backup_path.exists());

        let _ = remove_dir_all_force(&temp);
    }

    #[test]
    fn test_backup_retention_per_skill_preserves_other_skills() {
        let temp = std::env::temp_dir().join(format!("test_retain_{}", Uuid::new_v4()));
        let central_repo = temp.join("repo");
        let backup_root = central_repo.join(".backups");
        fs::create_dir_all(&backup_root).unwrap();

        // 技能 A 创建 1 个珍贵备份
        let b_a = backup_root.join("20260101_000000_skill-a");
        fs::create_dir_all(&b_a).unwrap();
        let meta_a = SkillBackupItem {
            id: "20260101_000000_skill-a".to_string(),
            skill_name: "skill-a".to_string(),
            source: None,
            created_at: Utc::now().to_rfc3339(),
            backup_path: b_a.to_string_lossy().to_string(),
            content_hash: "hash_a".to_string(),
            reason: None,
        };
        fs::write(
            b_a.join("meta.json"),
            serde_json::to_string(&meta_a).unwrap(),
        )
        .unwrap();

        // 技能 B 连续创建 25 个备份
        for i in 0..25 {
            let b_dir = backup_root.join(format!("20260102_{:06}_skill-b", i));
            fs::create_dir_all(&b_dir).unwrap();
            let meta_b = SkillBackupItem {
                id: format!("20260102_{:06}_skill-b", i),
                skill_name: "skill-b".to_string(),
                source: None,
                created_at: Utc::now().to_rfc3339(),
                backup_path: b_dir.to_string_lossy().to_string(),
                content_hash: format!("hash_b_{}", i),
                reason: None,
            };
            fs::write(
                b_dir.join("meta.json"),
                serde_json::to_string(&meta_b).unwrap(),
            )
            .unwrap();
        }

        // 调用按技能独立清理逻辑，保留 20 个
        cleanup_old_backups_for_skill(&backup_root, "skill-b", 20).unwrap();

        // 验证：技能 B 剩余 20 个，技能 A 的 1 个备份完好无损！
        let backups_b = list_skill_backups(&central_repo, Some("skill-b")).unwrap();
        assert_eq!(backups_b.len(), 20);

        let backups_a = list_skill_backups(&central_repo, Some("skill-a")).unwrap();
        assert_eq!(backups_a.len(), 1);
        assert_eq!(backups_a[0].skill_name, "skill-a");

        let _ = remove_dir_all_force(&temp);
    }

    #[test]
    fn test_scan_preserves_has_update_status() {
        let temp = std::env::temp_dir().join(format!("test_scan_status_{}", Uuid::new_v4()));
        let central_repo = temp.join("repo");
        let skill_dir = central_repo.join("skills").join("tracked-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "hello").unwrap();

        let db = Database::in_memory().unwrap();
        let repo = crate::models::Repository {
            id: "repo-test".to_string(),
            name: "Test".to_string(),
            path: central_repo.to_string_lossy().to_string(),
            source_type: "local".to_string(),
            git_remote: None,
            current_branch: None,
            last_scanned_at: None,
        };
        db.save_repository(&repo).unwrap();

        // 1. 初次扫描并入库
        let (skills, _) = scan_repository(&central_repo.to_string_lossy(), "repo-test").unwrap();
        db.upsert_skills(&skills).unwrap();

        let initial_skill = db
            .get_skill_by_name("repo-test", "tracked-skill")
            .unwrap()
            .unwrap();
        assert_eq!(initial_skill.has_update, Some(false));

        // 2. 模拟检查更新发现了新版本
        db.update_skill_update_status(
            "repo-test",
            "tracked-skill",
            "remote_new_hash_123",
            true,
            "2026-09-04T12:00:00Z",
        )
        .unwrap();

        let updated_in_db = db
            .get_skill_by_name("repo-test", "tracked-skill")
            .unwrap()
            .unwrap();
        assert_eq!(updated_in_db.has_update, Some(true));
        assert_eq!(
            updated_in_db.remote_hash.as_deref(),
            Some("remote_new_hash_123")
        );

        // 3. 用户或前端触发了 repository_scan（重新扫描本地文件系统）
        let (rescanned_skills, _) =
            scan_repository(&central_repo.to_string_lossy(), "repo-test").unwrap();
        db.upsert_skills(&rescanned_skills).unwrap();

        // 关键校验：重新扫描绝不可将 has_update 冲掉重置为 false！
        let skill_after_rescan = db
            .get_skill_by_name("repo-test", "tracked-skill")
            .unwrap()
            .unwrap();
        assert_eq!(
            skill_after_rescan.has_update,
            Some(true),
            "scan_repository & upsert_skills must preserve has_update = true!"
        );
        assert_eq!(
            skill_after_rescan.remote_hash.as_deref(),
            Some("remote_new_hash_123")
        );

        let _ = remove_dir_all_force(&temp);
    }
}
