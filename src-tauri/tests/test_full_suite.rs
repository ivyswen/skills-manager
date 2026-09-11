use chrono::Utc;
use skills_manager_lib::core::config_io::{
    apply_import_config, export_config, preview_import_config,
};
use skills_manager_lib::core::entry_engine::{
    check_entry_link_status, remove_entry_link, setup_entry_link,
};
use skills_manager_lib::core::link_engine::{
    create_mount, is_reparse_point, remove_mount, resolve_link_target,
};
use skills_manager_lib::core::scanner::scan_repository;
use skills_manager_lib::core::status_checker::diagnose_project_mounts;
use skills_manager_lib::db::Database;
use skills_manager_lib::models::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use uuid::Uuid;

#[test]
fn test_database_crud_and_single_repo_rule() {
    let db = Database::in_memory().unwrap();

    // 1. 测试添加仓库
    let repo1 = Repository {
        id: "repo-1".to_string(),
        name: "Repo 1".to_string(),
        path: "C:/repo1".to_string(),
        source_type: "git".to_string(),
        git_remote: Some("https://github.com/test/repo1.git".to_string()),
        current_branch: Some("main".to_string()),
        last_scanned_at: Some(Utc::now().to_rfc3339()),
    };
    db.save_repository(&repo1).unwrap();

    let fetched1 = db.get_repository().unwrap().unwrap();
    assert_eq!(fetched1.name, "Repo 1");

    // 单中央仓库约束：添加 repo2 应当替换之前的仓库
    let repo2 = Repository {
        id: "repo-2".to_string(),
        name: "Repo 2".to_string(),
        path: "C:/repo2".to_string(),
        source_type: "local".to_string(),
        git_remote: None,
        current_branch: None,
        last_scanned_at: None,
    };
    db.save_repository(&repo2).unwrap();

    let fetched2 = db.get_repository().unwrap().unwrap();
    assert_eq!(fetched2.id, "repo-2");
    assert_eq!(fetched2.name, "Repo 2");

    // 2. 项目 CRUD 与路径重定位
    let proj = Project {
        id: "proj-1".to_string(),
        name: "My App".to_string(),
        path: "C:/projects/my-app".to_string(),
        last_scanned_at: None,
        is_archived: false,
    };
    db.save_project(&proj).unwrap();
    assert_eq!(db.get_projects().unwrap().len(), 1);

    // 重定位路径
    db.update_project_path("proj-1", "D:/projects/my-app-relocated")
        .unwrap();
    let updated_proj = db.get_project_by_id("proj-1").unwrap().unwrap();
    assert_eq!(updated_proj.path, "D:/projects/my-app-relocated");
}

#[test]
fn test_scanner_and_mount_lifecycle() {
    let temp = std::env::temp_dir().join(format!("test_sm_lifecycle_{}", Uuid::new_v4()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    // 1. 创建中央仓库目录结构
    let central_repo = temp.join("central_repo");
    let skills_dir = central_repo.join("skills");
    let skill_a = skills_dir.join("code-review");
    fs::create_dir_all(&skill_a).unwrap();

    let skill_md_content = r#"---
name: code-review
description: Automated code review skill for pull requests.
---
# Instructions
Review the diff thoroughly.
"#;
    fs::write(skill_a.join("SKILL.md"), skill_md_content).unwrap();
    fs::write(skill_a.join("helper.py"), "print('helper')").unwrap();

    // 2. 执行扫描
    let (scanned_skills, warnings) =
        scan_repository(&central_repo.to_string_lossy(), "repo-1").unwrap();
    assert_eq!(scanned_skills.len(), 1);
    assert!(warnings.is_empty());
    let skill = &scanned_skills[0];
    assert_eq!(skill.name, "code-review");
    assert_eq!(skill.file_count, 2);
    assert_eq!(skill.metadata_status, "valid");
    assert!(!skill.content_hash.is_empty());

    // 3. 创建目标项目目录
    let project_dir = temp.join("target_project");
    let project_skills = project_dir.join(".agents").join("skills");
    fs::create_dir_all(&project_skills).unwrap();

    let target_mount = project_skills.join("code-review");

    // 4. 执行挂载 (Junction 模式)
    let (mode, backup) = create_mount(
        Path::new(&skill.canonical_path),
        &target_mount,
        Some("junction"),
        &ConflictStrategy::BackupAndReplace,
    )
    .unwrap();

    assert_eq!(mode, "junction");
    assert!(backup.is_none());
    assert!(is_reparse_point(&target_mount));

    let resolved = resolve_link_target(&target_mount);
    assert!(resolved.is_some());

    // 5. 双层解耦诊断
    let mount_record = Mount {
        id: Uuid::new_v4().to_string(),
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone(),
        project_id: "p1".to_string(),
        link_path: target_mount.to_string_lossy().to_string(),
        resolved_target: skill.canonical_path.clone(),
        mount_mode: "junction".to_string(),
        status: "UNCHECKED".to_string(),
        managed_by_app: true,
        backup_path: None,
        content_hash: Some(skill.content_hash.clone()),
        is_outdated: Some(false),
        has_local_changes: Some(false),
        has_conflict: Some(false),
        created_at: Utc::now().to_rfc3339(),
    };

    let diag = diagnose_project_mounts(&project_dir, &[mount_record.clone()], &scanned_skills);
    assert_eq!(diag.updated_mounts.len(), 1);
    assert_eq!(diag.updated_mounts[0].status, "NORMAL");
    assert!(diag.unmanaged_dirs.is_empty());

    // 6. 安全卸载 Junction (验证原件没有被误删！)
    remove_mount(&target_mount, &project_skills, "junction").unwrap();
    assert!(!target_mount.exists());
    // 关键安全验证：中央仓库原件完好无损！
    assert!(skill_a.join("SKILL.md").exists());
    assert!(skill_a.join("helper.py").exists());

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_conflict_backup_and_replace() {
    let temp = std::env::temp_dir().join(format!("test_sm_conflict_{}", Uuid::new_v4()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    let source = temp.join("source_skill");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "original skill").unwrap();

    let project_skills = temp.join("project").join(".agents").join("skills");
    fs::create_dir_all(&project_skills).unwrap();

    let target_link = project_skills.join("my-skill");
    // 预先存在用户自建同名普通目录
    fs::create_dir_all(&target_link).unwrap();
    fs::write(target_link.join("custom_user_code.txt"), "important data").unwrap();

    // 1. ConflictStrategy::Cancel 应当拦截并报错
    let cancel_res = create_mount(
        &source,
        &target_link,
        Some("junction"),
        &ConflictStrategy::Cancel,
    );
    assert!(cancel_res.is_err());
    assert_eq!(cancel_res.unwrap_err().code, "TARGET_CONFLICT");
    assert!(target_link.join("custom_user_code.txt").exists());

    // 2. ConflictStrategy::BackupAndReplace 应当将原目录移至 .backup/
    let (mode, backup_opt) = create_mount(
        &source,
        &target_link,
        Some("junction"),
        &ConflictStrategy::BackupAndReplace,
    )
    .unwrap();

    assert_eq!(mode, "junction");
    assert!(backup_opt.is_some());
    let backup_dir = backup_opt.unwrap();
    assert!(backup_dir.exists());
    assert!(backup_dir.join("custom_user_code.txt").exists());
    assert_eq!(
        fs::read_to_string(backup_dir.join("custom_user_code.txt")).unwrap(),
        "important data"
    );

    // 验证新链接已生效
    assert!(target_link.exists());
    assert!(target_link.join("SKILL.md").exists());

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_entry_link_lifecycle() {
    let temp = std::env::temp_dir().join(format!("test_sm_entry_{}", Uuid::new_v4()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    let project_dir = temp.join("project");
    let agents_skills = project_dir.join(".agents").join("skills");
    fs::create_dir_all(&agents_skills).unwrap();

    // 初始状态为 missing
    let status_init = check_entry_link_status(&project_dir, ".claude/skills");
    assert_eq!(status_init, "missing");

    // 创建 entry link (junction 模式)
    setup_entry_link(&project_dir, ".claude/skills", Some("junction")).unwrap();

    let status_after = check_entry_link_status(&project_dir, ".claude/skills");
    assert_eq!(status_after, "valid");

    // 幂等重新创建
    setup_entry_link(&project_dir, ".claude/skills", Some("junction")).unwrap();
    assert_eq!(
        check_entry_link_status(&project_dir, ".claude/skills"),
        "valid"
    );

    // 安全移除 entry link (确保 .agents/skills 绝不被删除！)
    remove_entry_link(&project_dir, ".claude/skills").unwrap();
    assert_eq!(
        check_entry_link_status(&project_dir, ".claude/skills"),
        "missing"
    );
    assert!(agents_skills.exists());

    // 测试 Pi 入口软链接 (.pi/skills)
    let status_pi_init = check_entry_link_status(&project_dir, ".pi/skills");
    assert_eq!(status_pi_init, "missing");

    setup_entry_link(&project_dir, ".pi/skills", Some("junction")).unwrap();
    assert_eq!(
        check_entry_link_status(&project_dir, ".pi/skills"),
        "valid"
    );

    remove_entry_link(&project_dir, ".pi/skills").unwrap();
    assert_eq!(
        check_entry_link_status(&project_dir, ".pi/skills"),
        "missing"
    );
    assert!(agents_skills.exists());

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_config_export_and_import_with_remapping() {
    let temp = std::env::temp_dir().join(format!("test_sm_config_{}", Uuid::new_v4()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    let db = Database::in_memory().unwrap();

    let repo = Repository {
        id: "repo-export".to_string(),
        name: "Central Export Repo".to_string(),
        path: temp.join("repo").to_string_lossy().to_string(),
        source_type: "git".to_string(),
        git_remote: Some("https://github.com/test/repo.git".to_string()),
        current_branch: Some("main".to_string()),
        last_scanned_at: Some(Utc::now().to_rfc3339()),
    };
    db.save_repository(&repo).unwrap();

    let orig_proj_path = "D:/old_machine/project_x";
    let proj = Project {
        id: "p-export".to_string(),
        name: "Project X".to_string(),
        path: orig_proj_path.to_string(),
        last_scanned_at: None,
        is_archived: false,
    };
    db.save_project(&proj).unwrap();

    let skill = Skill {
        id: "s-1".to_string(),
        repository_id: "repo-export".to_string(),
        name: "test-skill".to_string(),
        relative_path: "skills/test-skill".to_string(),
        canonical_path: "C:/repo/skills/test-skill".to_string(),
        description: "test".to_string(),
        metadata_status: "valid".to_string(),
        content_hash: "hash123".to_string(),
        file_count: 1,
        char_count: 10,
        last_seen_at: Utc::now().to_rfc3339(),
        source: None,
        remote_skill_id: None,
        remote_hash: None,
        has_update: Some(false),
        last_checked_at: None,
        updated_at: None,
    };
    db.upsert_skills(&[skill]).unwrap();

    let mount = Mount {
        id: "m-1".to_string(),
        skill_id: "s-1".to_string(),
        skill_name: "test-skill".to_string(),
        project_id: "p-export".to_string(),
        link_path: format!("{}/.agents/skills/test-skill", orig_proj_path),
        resolved_target: "C:/repo/skills/test-skill".to_string(),
        mount_mode: "junction".to_string(),
        status: "NORMAL".to_string(),
        managed_by_app: true,
        backup_path: None,
        content_hash: Some("hash123".to_string()),
        is_outdated: Some(false),
        has_local_changes: Some(false),
        has_conflict: Some(false),
        created_at: Utc::now().to_rfc3339(),
    };
    db.upsert_mount(&mount).unwrap();

    // 1. 导出配置
    let export_file = temp.join("config_backup.json");
    export_config(&db, &export_file).unwrap();
    assert!(export_file.exists());

    // 2. 预检导入
    let preview = preview_import_config(&export_file).unwrap();
    assert_eq!(preview.projects.len(), 1);
    assert_eq!(preview.projects[0].original_path, orig_proj_path);
    assert!(!preview.projects[0].exists); // 该旧机器路径不存在

    // 3. 应用重映射导入到新数据库
    let new_db = Database::in_memory().unwrap();
    let new_mapped_path = temp
        .join("new_machine_project")
        .to_string_lossy()
        .to_string();
    let mut mappings = HashMap::new();
    mappings.insert(orig_proj_path.to_string(), new_mapped_path.clone());

    apply_import_config(&new_db, &export_file, mappings).unwrap();

    // 验证新数据库的项目路径与挂载已成功重定位
    let imported_projs = new_db.get_projects().unwrap();
    assert_eq!(imported_projs.len(), 1);
    assert_eq!(imported_projs[0].path, new_mapped_path);

    let imported_mounts = new_db.get_mounts_by_project(&imported_projs[0].id).unwrap();
    assert_eq!(imported_mounts.len(), 1);
    assert!(imported_mounts[0].link_path.contains(&new_mapped_path));

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_broken_link_unmount() {
    let temp = std::env::temp_dir().join(format!("test_sm_broken_{}", Uuid::new_v4()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    let source = temp.join("source_skill");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("SKILL.md"), "test broken").unwrap();

    let project_skills = temp.join("project").join(".agents").join("skills");
    fs::create_dir_all(&project_skills).unwrap();

    let target_link = project_skills.join("broken-skill");

    // 创建挂载 (Junction)
    create_mount(
        &source,
        &target_link,
        Some("junction"),
        &ConflictStrategy::Cancel,
    )
    .unwrap();

    assert!(is_reparse_point(&target_link));

    // 人为删除原件，制造“断链” (BROKEN LINK)
    fs::remove_dir_all(&source).unwrap();
    assert!(!source.exists());

    // 验证此时 target_link.exists() 为 false（因为跟随断开的目标返回 false）
    assert!(!target_link.exists());
    // 但作为 Reparse Point 实体，它依然在磁盘上存在
    assert!(is_reparse_point(&target_link));

    // 执行卸载断链：必须能成功清除该残留 Reparse Point
    remove_mount(&target_link, &project_skills, "junction").unwrap();
    assert!(!is_reparse_point(&target_link));
    assert!(fs::symlink_metadata(&target_link).is_err());

    let _ = fs::remove_dir_all(&temp);
}

#[test]
fn test_copy_mode_outdated_detection() {
    let temp = std::env::temp_dir().join(format!("test_sm_copy_outdated_{}", Uuid::new_v4()));
    let _ = fs::remove_dir_all(&temp);
    fs::create_dir_all(&temp).unwrap();

    let repo_dir = temp.join("repo");
    let skill_dir = repo_dir.join("skills").join("copy-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "version 1").unwrap();

    let (scanned_v1, _) = scan_repository(&repo_dir.to_string_lossy(), "repo-copy").unwrap();
    let skill_v1 = &scanned_v1[0];

    let project_dir = temp.join("project");
    let project_skills = project_dir.join(".agents").join("skills");
    fs::create_dir_all(&project_skills).unwrap();
    let target_copy = project_skills.join("copy-skill");

    // Copy 模式安装
    create_mount(
        Path::new(&skill_v1.canonical_path),
        &target_copy,
        Some("copy"),
        &ConflictStrategy::Cancel,
    )
    .unwrap();

    let mount_rec = Mount {
        id: "m-copy".to_string(),
        skill_id: skill_v1.id.clone(),
        skill_name: skill_v1.name.clone(),
        project_id: "p-copy".to_string(),
        link_path: target_copy.to_string_lossy().to_string(),
        resolved_target: skill_v1.canonical_path.clone(),
        mount_mode: "copy".to_string(),
        status: "UNCHECKED".to_string(),
        managed_by_app: true,
        backup_path: None,
        content_hash: Some(skill_v1.content_hash.clone()),
        is_outdated: Some(false),
        has_local_changes: Some(false),
        has_conflict: Some(false),
        created_at: Utc::now().to_rfc3339(),
    };

    // 首次诊断：状态应为 NORMAL，且 is_outdated 为 false
    let diag1 = diagnose_project_mounts(&project_dir, &[mount_rec.clone()], &scanned_v1);
    assert_eq!(diag1.updated_mounts[0].status, "NORMAL");
    assert_eq!(diag1.updated_mounts[0].is_outdated, Some(false));

    // 原件内容更新：version 2
    fs::write(skill_dir.join("SKILL.md"), "version 2 modified").unwrap();
    let (scanned_v2, _) = scan_repository(&repo_dir.to_string_lossy(), "repo-copy").unwrap();
    assert_ne!(scanned_v1[0].content_hash, scanned_v2[0].content_hash);

    // 再次诊断：应当精确捕获并置 is_outdated = Some(true)
    let diag2 = diagnose_project_mounts(&project_dir, &[mount_rec], &scanned_v2);
    assert_eq!(diag2.updated_mounts[0].status, "NORMAL");
    assert_eq!(diag2.updated_mounts[0].is_outdated, Some(true));

    let _ = fs::remove_dir_all(&temp);
}
