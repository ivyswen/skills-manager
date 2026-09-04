use crate::db::Database;
use crate::models::{AppError, ExportConfigData, ImportPreviewResult, ImportProjectPreview};
use chrono::Utc;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn export_config(db: &Database, output_path: &Path) -> Result<String, AppError> {
    let repository = db.get_repository()?;
    let skills = if let Some(ref r) = repository {
        db.get_skills(&r.id)?
    } else {
        Vec::new()
    };
    let projects = db.get_projects()?;
    let mut all_agent_targets = Vec::new();
    let mut all_entry_links = Vec::new();

    for p in &projects {
        let targets = db.get_agent_targets(&p.id)?;
        for t in &targets {
            let links = db.get_entry_links(&t.id)?;
            all_entry_links.extend(links);
        }
        all_agent_targets.extend(targets);
    }

    let mounts = db.get_all_mounts()?;
    let settings = db.get_all_settings()?;

    let data = ExportConfigData {
        version: "1.1".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        repository,
        skills,
        projects,
        agent_targets: all_agent_targets,
        entry_links: all_entry_links,
        mounts,
        settings,
    };

    let json_str = serde_json::to_string_pretty(&data)
        .map_err(|e| AppError::with_details("EXPORT_FAILED", "JSON 序列化失败", e.to_string()))?;

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(AppError::io_error)?;
    }
    fs::write(output_path, json_str).map_err(AppError::io_error)?;

    Ok(format!("配置已成功导出至 {}", output_path.display()))
}

pub fn preview_import_config(file_path: &Path) -> Result<ImportPreviewResult, AppError> {
    if !file_path.exists() {
        return Err(AppError::path_not_found("指定的配置文件不存在"));
    }

    let content = fs::read_to_string(file_path).map_err(AppError::io_error)?;
    let data: ExportConfigData = serde_json::from_str(&content).map_err(|e| {
        AppError::with_details(
            "INVALID_CONFIG",
            "配置文件解析失败，格式不正确",
            e.to_string(),
        )
    })?;

    let repo_exists = data
        .repository
        .as_ref()
        .map(|r| Path::new(&r.path).exists())
        .unwrap_or(false);

    let mut projects_preview = Vec::new();
    for p in data.projects {
        let exists = Path::new(&p.path).exists();
        let p_mounts: Vec<String> = data
            .mounts
            .iter()
            .filter(|m| m.project_id == p.id)
            .map(|m| m.skill_name.clone())
            .collect();

        projects_preview.push(ImportProjectPreview {
            original_path: p.path.clone(),
            remapped_path: p.path,
            exists,
            skill_names: p_mounts,
        });
    }

    Ok(ImportPreviewResult {
        repository: data.repository,
        repository_exists: repo_exists,
        projects: projects_preview,
        settings_count: data.settings.len(),
    })
}

pub fn apply_import_config(
    db: &Database,
    file_path: &Path,
    path_mappings: HashMap<String, String>, // original_path -> remapped_path
) -> Result<(), AppError> {
    let content = fs::read_to_string(file_path).map_err(AppError::io_error)?;
    let data: ExportConfigData = serde_json::from_str(&content)
        .map_err(|e| AppError::with_details("INVALID_CONFIG", "配置文件格式错误", e.to_string()))?;

    // 1. 仓库与 Skill 导入
    let mut old_repo_path_opt = None;
    let mut new_repo_path_opt = None;
    if let Some(mut repo) = data.repository {
        let orig_repo_path = repo.path.clone();
        if let Some(remapped) = path_mappings.get(&orig_repo_path) {
            repo.path = remapped.clone();
        }
        old_repo_path_opt = Some(orig_repo_path);
        new_repo_path_opt = Some(repo.path.clone());
        let _ = db.save_repository(&repo);

        // 如果新机器上的仓库路径存在，扫描并更新 skills
        if Path::new(&repo.path).exists() {
            if let Ok((scanned_skills, _)) =
                crate::core::scanner::scan_repository(&repo.path, &repo.id)
            {
                let _ = db.upsert_skills(&scanned_skills);
            } else if !data.skills.is_empty() {
                let _ = db.upsert_skills(&data.skills);
            }
        } else if !data.skills.is_empty() {
            let _ = db.upsert_skills(&data.skills);
        }
    }

    // 2. 项目与挂载导入
    for mut p in data.projects {
        let old_path = p.path.clone();
        if let Some(remapped) = path_mappings.get(&old_path) {
            p.path = remapped.clone();
        }
        db.save_project(&p)?;

        // 对应 agent_targets
        let targets: Vec<_> = data
            .agent_targets
            .iter()
            .filter(|t| t.project_id == p.id)
            .collect();
        for t in targets {
            let _ = db.upsert_agent_target(t);
            let links: Vec<_> = data
                .entry_links
                .iter()
                .filter(|l| l.agent_target_id == t.id)
                .collect();
            for l in links {
                let _ = db.upsert_entry_link(l);
            }
        }

        // 对应 mounts
        let project_mounts: Vec<_> = data
            .mounts
            .iter()
            .filter(|m| m.project_id == p.id)
            .collect();
        for m in project_mounts {
            let mut new_mount = m.clone();
            // 如果项目路径变动，更新 link_path
            if p.path != old_path {
                let new_link = Path::new(&p.path)
                    .join(".agents")
                    .join("skills")
                    .join(&new_mount.skill_name);
                new_mount.link_path = new_link.to_string_lossy().to_string();
            }
            // 如果仓库路径变动，更新 resolved_target
            if let (Some(old_r), Some(new_r)) = (&old_repo_path_opt, &new_repo_path_opt) {
                if old_r != new_r && new_mount.resolved_target.starts_with(old_r) {
                    new_mount.resolved_target = new_mount.resolved_target.replacen(old_r, new_r, 1);
                }
            }
            let _ = db.upsert_mount(&new_mount);
        }
    }

    // 3. 设置导入
    for s in data.settings {
        let _ = db.set_setting(&s.key, &s.value);
    }

    Ok(())
}
