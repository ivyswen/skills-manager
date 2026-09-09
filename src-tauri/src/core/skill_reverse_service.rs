use crate::core::link_engine::{create_mount, jail_check};
use crate::core::market_service::{
    copy_skill_dir_clean, remove_dir_all_force, sanitize_skill_name,
};
use crate::core::scanner::{measure_skill_dir, scan_repository};
use crate::core::skill_update_service::create_skill_backup;
use crate::db::Database;
use crate::models::{
    AppError, ConflictStrategy, Mount, MountResult, MountStatus, OperationLog, ReversePushRequest,
    ReversePushResult, SkillDiffItem, SkillDiffResult,
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;
use walkdir::WalkDir;

/// 计算单个文件的 SHA-256 哈希值
fn compute_file_hash(path: &Path) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// 收集目录内的所有有效文件相对路径及其哈希
fn collect_skill_files(dir: &Path) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if !dir.exists() || !dir.is_dir() {
        return map;
    }
    for entry in WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with(".git") && name != ".DS_Store" && name != ".skill-meta.json"
        })
        .flatten()
    {
        if entry.file_type().is_file() {
            if let Ok(rel) = entry.path().strip_prefix(dir) {
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if let Ok(h) = compute_file_hash(entry.path()) {
                    map.insert(rel_str, h);
                }
            }
        }
    }
    map
}

/// 检查项目内技能与中央仓库原件的文件差异及冲突状态
pub fn inspect_skill_diff(
    project_path: &Path,
    central_repo_path: &Path,
    skill_name: &str,
    db: &Database,
) -> Result<SkillDiffResult, AppError> {
    let safe_name = sanitize_skill_name(skill_name);
    let project_skills_dir = project_path.join(".agents").join("skills");
    let local_skill_dir = project_skills_dir.join(&safe_name);
    let central_skills_dir = central_repo_path.join("skills");
    let central_skill_dir = central_skills_dir.join(&safe_name);

    // 安全 Jail 校验
    jail_check(&project_skills_dir, &local_skill_dir)?;
    jail_check(&central_skills_dir, &central_skill_dir)?;

    if !local_skill_dir.exists() || !local_skill_dir.is_dir() {
        return Err(AppError::new(
            "LOCAL_SKILL_NOT_FOUND",
            "项目内技能目录不存在",
        ));
    }

    // 获取项目的挂载基准哈希
    let project_id_opt = db
        .get_projects()?
        .into_iter()
        .find(|p| dunce::canonicalize(&p.path).ok() == dunce::canonicalize(project_path).ok())
        .map(|p| p.id);

    let base_hash = if let Some(ref pid) = project_id_opt {
        db.get_mount_by_project_and_skill(pid, &safe_name)?
            .and_then(|m| m.content_hash)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let (_, _, local_hash) = measure_skill_dir(&local_skill_dir);
    let central_hash = if central_skill_dir.exists() {
        measure_skill_dir(&central_skill_dir).2
    } else {
        String::new()
    };

    // 冲突判定：基准哈希存在，且两端各自发生变动，且变动后内容不一致
    let has_conflict = !base_hash.is_empty()
        && base_hash != local_hash
        && base_hash != central_hash
        && local_hash != central_hash;

    // 详细比对文件列表
    let local_files = collect_skill_files(&local_skill_dir);
    let central_files = collect_skill_files(&central_skill_dir);

    let mut files = Vec::new();

    for (rel_path, local_h) in &local_files {
        match central_files.get(rel_path) {
            Some(central_h) => {
                if central_h != local_h {
                    files.push(SkillDiffItem {
                        path: rel_path.clone(),
                        change_type: "MODIFIED".to_string(),
                    });
                }
            }
            None => {
                files.push(SkillDiffItem {
                    path: rel_path.clone(),
                    change_type: "ADDED".to_string(),
                });
            }
        }
    }

    for rel_path in central_files.keys() {
        if !local_files.contains_key(rel_path) {
            files.push(SkillDiffItem {
                path: rel_path.clone(),
                change_type: "DELETED".to_string(),
            });
        }
    }

    files.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(SkillDiffResult {
        skill_name: safe_name,
        has_conflict,
        files,
        local_hash,
        central_hash,
        base_hash,
    })
}

/// 执行反向更新推送到中央仓库原件
pub fn execute_reverse_push(
    project_path: &Path,
    central_repo_path: &Path,
    request: &ReversePushRequest,
    db: &Database,
) -> Result<ReversePushResult, AppError> {
    let safe_name = sanitize_skill_name(&request.skill_name);
    let project_skills_dir = project_path.join(".agents").join("skills");
    let local_skill_dir = project_skills_dir.join(&safe_name);
    let central_skills_dir = central_repo_path.join("skills");
    let central_skill_dir = central_skills_dir.join(&safe_name);

    jail_check(&project_skills_dir, &local_skill_dir)?;
    jail_check(&central_skills_dir, &central_skill_dir)?;

    if !local_skill_dir.exists() || !local_skill_dir.is_dir() {
        return Err(AppError::new(
            "LOCAL_SKILL_NOT_FOUND",
            "项目内技能目录不存在",
        ));
    }

    let skill_md_path = local_skill_dir.join("SKILL.md");
    if !skill_md_path.exists() || !skill_md_path.is_file() {
        return Err(AppError::new(
            "MISSING_SKILL_MD",
            "项目内技能缺少有效的 SKILL.md 规范文件，无法反向推送到中央仓库",
        ));
    }

    // 预检冲突
    let diff = inspect_skill_diff(project_path, central_repo_path, &safe_name, db)?;
    if diff.has_conflict && !request.force {
        return Err(AppError::new(
            "CONFLICT_DETECTED",
            "检测到中央仓库原件已被外部更新，存在双向修改冲突。若确认覆盖请强制推送。",
        ));
    }

    // 核心安全前置：自动为中央仓库现有原件创建快照备份
    let mut backup_id = None;
    if central_skill_dir.exists() {
        match create_skill_backup(central_repo_path, &safe_name, db, "reverse-push") {
            Ok(Some(bp)) => {
                backup_id = bp.file_name().map(|n| n.to_string_lossy().to_string());
                crate::core::logger::info(
                    "ReversePush",
                    &format!("反向更新前已自动创建中央仓库快照备份: {:?}", backup_id),
                );
            }
            Ok(None) => {}
            Err(e) => {
                return Err(AppError::with_details(
                    "BACKUP_FAILED",
                    "反向更新前创建原件快照备份失败，已中止操作以保障数据安全",
                    e.message,
                ));
            }
        }
    }

    // 清理并覆写中央仓库目录
    if central_skill_dir.exists() {
        remove_dir_all_force(&central_skill_dir).map_err(|e| {
            AppError::with_details(
                "CLEAN_CENTRAL_ERROR",
                "清空中央仓库原件目录失败",
                e.to_string(),
            )
        })?;
    }

    copy_skill_dir_clean(&local_skill_dir, &central_skill_dir)?;

    // 重新测量并刷新中央仓库与数据库
    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    let (scanned_skills, warnings) = scan_repository(&repo.path, &repo.id)?;
    if !warnings.is_empty() {
        crate::core::logger::warn("ReversePush", &format!("反向更新扫描警告: {:?}", warnings));
    }
    db.upsert_skills(&scanned_skills)?;

    // 获取更新后的中央仓库 Skill
    let updated_skill = db
        .get_skill_by_name(&repo.id, &safe_name)?
        .ok_or_else(|| AppError::new("SKILL_NOT_FOUND", "刷新技能数据失败"))?;

    // 对齐当前项目的 Mount 记录基准哈希
    if let Ok(Some(mut mount)) = db.get_mount_by_project_and_skill(&request.project_id, &safe_name)
    {
        mount.content_hash = Some(updated_skill.content_hash.clone());
        mount.is_outdated = Some(false);
        mount.has_local_changes = Some(false);
        mount.has_conflict = Some(false);
        db.upsert_mount(&mount)?;
    }

    // 记录审计日志
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "reverse_push_skill".to_string(),
        entity_type: "skill".to_string(),
        entity_id: Some(updated_skill.id),
        project_id: Some(request.project_id.clone()),
        skill_name: Some(safe_name.clone()),
        target_path: Some(central_skill_dir.to_string_lossy().to_string()),
        backup_path: backup_id.clone(),
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!(
            "反向更新成功：已将项目内技能 '{}' 安全同步至中央仓库 (备份标识: {:?})",
            safe_name, backup_id
        ),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    crate::core::logger::info(
        "ReversePush",
        &format!("技能 '{}' 反向更新中央仓库成功", safe_name),
    );

    Ok(ReversePushResult {
        success: true,
        skill_name: safe_name,
        backup_id,
        message: "反向更新中央仓库原件成功".to_string(),
    })
}

/// 将项目内新建/未托管的技能目录规范化入库并原地纳管
pub fn execute_import_unmanaged(
    project_path: &Path,
    central_repo_path: &Path,
    project_id: &str,
    dir_name: &str,
    preferred_mode: Option<&str>,
    db: &Database,
) -> Result<MountResult, AppError> {
    let safe_name = sanitize_skill_name(dir_name);
    let project_skills_dir = project_path.join(".agents").join("skills");
    let local_dir = project_skills_dir.join(&safe_name);
    let central_skills_dir = central_repo_path.join("skills");
    let central_dest = central_skills_dir.join(&safe_name);

    jail_check(&project_skills_dir, &local_dir)?;
    jail_check(&central_skills_dir, &central_dest)?;

    if !local_dir.exists() || !local_dir.is_dir() {
        return Err(AppError::new("DIR_NOT_FOUND", "未托管实体目录不存在"));
    }

    let skill_md_path = local_dir.join("SKILL.md");
    if !skill_md_path.exists() || !skill_md_path.is_file() {
        return Err(AppError::new(
            "MISSING_SKILL_MD",
            "未托管目录中缺少 SKILL.md 规范文件，请补齐后再尝试入库",
        ));
    }

    if central_dest.exists() {
        return Err(AppError::new(
            "SKILL_ALREADY_EXISTS",
            "中央仓库已存在同名技能，请先重命名本地目录或执行反向更新",
        ));
    }

    // 复制到中央仓库 skills/ 目录下
    copy_skill_dir_clean(&local_dir, &central_dest)?;

    let repo = db
        .get_repository()?
        .ok_or_else(|| AppError::new("NO_REPO", "尚未配置中央仓库"))?;

    // 扫描中央仓库并同步入库
    let (scanned_skills, _) = scan_repository(&repo.path, &repo.id)?;
    db.upsert_skills(&scanned_skills)?;

    let new_skill = db
        .get_skill_by_name(&repo.id, &safe_name)?
        .ok_or_else(|| AppError::new("SKILL_IMPORT_ERROR", "入库后未能检索到该技能"))?;

    let mode = preferred_mode.unwrap_or("copy");
    let mut backup_path_str = None;

    match mode {
        "copy" => {
            // Copy 模式：原物理目录保持不变，直接在数据库中写入挂载纳管记录
            let mount = Mount {
                id: Uuid::new_v4().to_string(),
                skill_id: new_skill.id.clone(),
                skill_name: new_skill.name.clone(),
                project_id: project_id.to_string(),
                link_path: local_dir.to_string_lossy().to_string(),
                resolved_target: new_skill.canonical_path.clone(),
                mount_mode: "copy".to_string(),
                status: MountStatus::Normal.as_str().to_string(),
                managed_by_app: true,
                backup_path: None,
                content_hash: Some(new_skill.content_hash.clone()),
                is_outdated: Some(false),
                has_local_changes: Some(false),
                has_conflict: Some(false),
                created_at: Utc::now().to_rfc3339(),
            };
            db.upsert_mount(&mount)?;
        }
        "junction" | "symlink" => {
            // Junction / Symlink 模式：将原目录移入 .backup，创建指向中央原件的链接
            let mount_res = create_mount(
                Path::new(&new_skill.canonical_path),
                &local_dir,
                Some(mode),
                &ConflictStrategy::BackupAndReplace,
            )?;
            let (actual_mode, backup_opt) = mount_res;
            backup_path_str = backup_opt.as_ref().map(|p| p.to_string_lossy().to_string());

            let mount = Mount {
                id: Uuid::new_v4().to_string(),
                skill_id: new_skill.id.clone(),
                skill_name: new_skill.name.clone(),
                project_id: project_id.to_string(),
                link_path: local_dir.to_string_lossy().to_string(),
                resolved_target: new_skill.canonical_path.clone(),
                mount_mode: actual_mode,
                status: MountStatus::Normal.as_str().to_string(),
                managed_by_app: true,
                backup_path: backup_path_str.clone(),
                content_hash: Some(new_skill.content_hash.clone()),
                is_outdated: Some(false),
                has_local_changes: Some(false),
                has_conflict: Some(false),
                created_at: Utc::now().to_rfc3339(),
            };
            db.upsert_mount(&mount)?;
        }
        _ => {
            return Err(AppError::new("INVALID_MODE", "未知的挂载模式"));
        }
    }

    // 记录操作日志
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "import_unmanaged_skill".to_string(),
        entity_type: "skill".to_string(),
        entity_id: Some(new_skill.id.clone()),
        project_id: Some(project_id.to_string()),
        skill_name: Some(safe_name.clone()),
        target_path: Some(central_dest.to_string_lossy().to_string()),
        backup_path: backup_path_str.clone(),
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!("未托管技能 '{}' 成功入库并纳管为项目挂载", safe_name),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(MountResult {
        skill_id: new_skill.id,
        skill_name: safe_name,
        success: true,
        status: "SUCCESS".to_string(),
        mount_mode: mode.to_string(),
        error_code: None,
        error_message: None,
        backup_path: backup_path_str,
        batch_id: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::models::{Project, Repository};
    use std::path::PathBuf;

    struct TestDirGuard(PathBuf);
    impl Drop for TestDirGuard {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn setup_test_env(name: &str) -> (TestDirGuard, Database, Repository, Project) {
        let temp_dir = std::env::temp_dir().join(format!("test_rev_{}_{}", name, Uuid::new_v4()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let repo_dir = temp_dir.join("repo");
        let skills_dir = repo_dir.join("skills");
        let project_dir = temp_dir.join("proj");
        let proj_skills_dir = project_dir.join(".agents").join("skills");

        fs::create_dir_all(&skills_dir).unwrap();
        fs::create_dir_all(&proj_skills_dir).unwrap();

        let db = Database::in_memory().unwrap();
        let repo = Repository {
            id: format!("repo-{}", name),
            name: "Central Repo".to_string(),
            path: repo_dir.to_string_lossy().to_string(),
            source_type: "local".to_string(),
            git_remote: None,
            current_branch: None,
            last_scanned_at: Some(Utc::now().to_rfc3339()),
        };
        db.save_repository(&repo).unwrap();

        let project = Project {
            id: format!("proj-{}", name),
            name: "Project A".to_string(),
            path: project_dir.to_string_lossy().to_string(),
            last_scanned_at: Some(Utc::now().to_rfc3339()),
            is_archived: false,
        };
        db.save_project(&project).unwrap();

        (TestDirGuard(temp_dir), db, repo, project)
    }

    #[test]
    fn test_inspect_diff_and_reverse_push() {
        let (_guard, db, repo, project) = setup_test_env("rev1");
        let repo_path = Path::new(&repo.path);
        let project_path = Path::new(&project.path);

        // 1. 在中央仓库创建技能 skill-1
        let central_s1 = repo_path.join("skills").join("skill-1");
        fs::create_dir_all(&central_s1).unwrap();
        fs::write(
            central_s1.join("SKILL.md"),
            "---\ndescription: v1\n---\n# S1",
        )
        .unwrap();
        fs::write(central_s1.join("base.txt"), "hello base").unwrap();

        let (scanned, _) = scan_repository(&repo.path, &repo.id).unwrap();
        db.upsert_skills(&scanned).unwrap();
        let s1 = db.get_skill_by_name(&repo.id, "skill-1").unwrap().unwrap();

        // 2. 在项目内创建 Copy 副本并记录挂载
        let local_s1 = project_path.join(".agents").join("skills").join("skill-1");
        fs::create_dir_all(&local_s1).unwrap();
        fs::write(local_s1.join("SKILL.md"), "---\ndescription: v1\n---\n# S1").unwrap();
        fs::write(local_s1.join("base.txt"), "hello base").unwrap();

        let mount = Mount {
            id: "m-1".to_string(),
            skill_id: s1.id.clone(),
            skill_name: "skill-1".to_string(),
            project_id: project.id.clone(),
            link_path: local_s1.to_string_lossy().to_string(),
            resolved_target: s1.canonical_path.clone(),
            mount_mode: "copy".to_string(),
            status: "NORMAL".to_string(),
            managed_by_app: true,
            backup_path: None,
            content_hash: Some(s1.content_hash.clone()),
            is_outdated: Some(false),
            has_local_changes: Some(false),
            has_conflict: Some(false),
            created_at: Utc::now().to_rfc3339(),
        };
        db.upsert_mount(&mount).unwrap();

        // 3. 项目端修改：修改 base.txt，新增 new.txt
        fs::write(local_s1.join("base.txt"), "hello modified").unwrap();
        fs::write(local_s1.join("new.txt"), "new file").unwrap();

        // 4. 检查差异
        let diff = inspect_skill_diff(project_path, repo_path, "skill-1", &db).unwrap();
        assert_eq!(diff.skill_name, "skill-1");
        assert!(!diff.has_conflict);
        assert_eq!(diff.files.len(), 2);
        assert!(diff
            .files
            .iter()
            .any(|f| f.path == "base.txt" && f.change_type == "MODIFIED"));
        assert!(diff
            .files
            .iter()
            .any(|f| f.path == "new.txt" && f.change_type == "ADDED"));

        // 5. 执行反向更新
        let req = ReversePushRequest {
            project_id: project.id.clone(),
            skill_name: "skill-1".to_string(),
            force: false,
        };
        let res = execute_reverse_push(project_path, repo_path, &req, &db).unwrap();
        assert!(res.success);
        assert!(res.backup_id.is_some());

        // 6. 验证中央仓库已被覆写且包含新文件
        assert_eq!(
            fs::read_to_string(central_s1.join("base.txt")).unwrap(),
            "hello modified"
        );
        assert!(central_s1.join("new.txt").exists());

        // 7. 验证中央仓库自动快照备份已生成
        let backups_dir = repo_path.join(".backups");
        assert!(backups_dir.join(res.backup_id.unwrap()).exists());

        // 8. 验证再次比对无差异
        let diff2 = inspect_skill_diff(project_path, repo_path, "skill-1", &db).unwrap();
        assert!(diff2.files.is_empty());
        assert_eq!(diff2.local_hash, diff2.central_hash);
    }

    #[test]
    fn test_import_unmanaged_skill() {
        let (_guard, db, repo, project) = setup_test_env("rev2");
        let repo_path = Path::new(&repo.path);
        let project_path = Path::new(&project.path);

        // 1. 在项目端创建未托管技能
        let local_unmanaged = project_path.join(".agents").join("skills").join("my-tool");
        fs::create_dir_all(&local_unmanaged).unwrap();

        // 缺少 SKILL.md 时应该报错阻断
        let err = execute_import_unmanaged(
            project_path,
            repo_path,
            &project.id,
            "my-tool",
            Some("copy"),
            &db,
        )
        .unwrap_err();
        assert_eq!(err.code, "MISSING_SKILL_MD");

        // 补齐 SKILL.md
        fs::write(
            local_unmanaged.join("SKILL.md"),
            "---\ndescription: my tool desc\n---\n# My Tool",
        )
        .unwrap();

        // 2. 执行入库纳管
        let mount_res = execute_import_unmanaged(
            project_path,
            repo_path,
            &project.id,
            "my-tool",
            Some("copy"),
            &db,
        )
        .unwrap();
        assert!(mount_res.success);

        // 3. 验证中央仓库已存在该技能
        let central_tool = repo_path.join("skills").join("my-tool");
        assert!(central_tool.join("SKILL.md").exists());

        // 4. 验证数据库中已建立技能记录与项目挂载记录
        let imported_skill = db.get_skill_by_name(&repo.id, "my-tool").unwrap();
        assert!(imported_skill.is_some());
        let m = db
            .get_mount_by_project_and_skill(&project.id, "my-tool")
            .unwrap();
        assert!(m.is_some());
        assert_eq!(m.unwrap().mount_mode, "copy");
    }
}
