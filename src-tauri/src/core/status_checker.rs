use crate::core::link_engine::{is_reparse_point, resolve_link_target};
use crate::models::{Mount, MountStatus, Skill};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct DiagnosticResult {
    pub updated_mounts: Vec<Mount>,
    pub unmanaged_dirs: Vec<String>,
}

/// 诊断单个项目的挂载状态与未托管文件（双层解耦检测机）
pub fn diagnose_project_mounts(
    project_path: &Path,
    db_mounts: &[Mount],
    available_skills: &[Skill],
) -> DiagnosticResult {
    let mut updated_mounts = Vec::new();
    let agents_skills_dir = project_path.join(".agents").join("skills");

    // 第一层：诊断 DB 中登记的挂载项
    for mount in db_mounts {
        let mut m = mount.clone();
        let target_path = PathBuf::from(&m.link_path);

        // 查找对应的 Skill 原件
        let skill_opt = available_skills
            .iter()
            .find(|s| s.id == m.skill_id || s.name == m.skill_name);

        if !target_path.exists() && !is_reparse_point(&target_path) {
            m.status = MountStatus::Broken.as_str().to_string();
            updated_mounts.push(m);
            continue;
        }

        // 测试读取权限
        if let Err(e) = fs::metadata(&target_path) {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                m.status = MountStatus::PermissionDenied.as_str().to_string();
                updated_mounts.push(m);
                continue;
            }
        }

        let is_reparse = is_reparse_point(&target_path);

        if is_reparse {
            // 是软链接或 Junction
            if let Some(resolved) = resolve_link_target(&target_path) {
                if !resolved.exists() {
                    m.status = MountStatus::Broken.as_str().to_string();
                } else if let Some(skill) = skill_opt {
                    let expected_can = dunce::canonicalize(&skill.canonical_path)
                        .unwrap_or_else(|_| PathBuf::from(&skill.canonical_path));
                    let resolved_can = dunce::canonicalize(&resolved).unwrap_or(resolved);

                    #[cfg(windows)]
                    let is_match = resolved_can
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&expected_can.to_string_lossy());
                    #[cfg(not(windows))]
                    let is_match = resolved_can == expected_can;

                    if is_match {
                        m.status = MountStatus::Normal.as_str().to_string();
                    } else {
                        m.status = MountStatus::WrongTarget.as_str().to_string();
                    }
                } else {
                    m.status = MountStatus::WrongTarget.as_str().to_string();
                }
            } else {
                m.status = MountStatus::Broken.as_str().to_string();
            }
        } else {
            // 不是软链接/Junction
            if m.mount_mode == "copy" {
                if target_path.is_dir() {
                    m.status = MountStatus::Normal.as_str().to_string();
                    if let Some(skill) = skill_opt {
                        if let Some(ref mh) = m.content_hash {
                            m.is_outdated = Some(!mh.is_empty() && mh != &skill.content_hash);
                        }
                    }
                } else {
                    m.status = MountStatus::Conflict.as_str().to_string();
                }
            } else {
                // 预期为 symlink/junction，物理上却是普通文件或普通目录
                m.status = MountStatus::Conflict.as_str().to_string();
            }
        }

        updated_mounts.push(m);
    }

    // 第二层：物理文件系统发现未托管目录
    let mut unmanaged = Vec::new();
    if agents_skills_dir.exists() && agents_skills_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&agents_skills_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // 忽略系统备份目录与隐藏文件
                if name.starts_with('.') {
                    continue;
                }

                // 检查是否在 DB 登记挂载项中
                let registered = db_mounts.iter().any(|m| m.skill_name == name);
                if !registered {
                    unmanaged.push(name);
                }
            }
        }
    }

    unmanaged.sort();

    DiagnosticResult {
        updated_mounts,
        unmanaged_dirs: unmanaged,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_status_checker_decoupled() {
        let temp = std::env::temp_dir().join("test_status_decoupled");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let repo_skills = temp.join("repo").join("skills").join("skill-a");
        fs::create_dir_all(&repo_skills).unwrap();
        fs::write(repo_skills.join("SKILL.md"), "content").unwrap();

        let project_skills = temp.join("project").join(".agents").join("skills");
        fs::create_dir_all(&project_skills).unwrap();

        // 1. 创建未托管物理目录
        let unmanaged_dir = project_skills.join("unmanaged-skill");
        fs::create_dir_all(&unmanaged_dir).unwrap();

        // 2. 创建登记但物理缺失的断链项
        let broken_mount = Mount {
            id: Uuid::new_v4().to_string(),
            skill_id: "skill-a-id".to_string(),
            skill_name: "skill-a".to_string(),
            project_id: "proj-1".to_string(),
            link_path: project_skills.join("skill-a").to_string_lossy().to_string(),
            resolved_target: repo_skills.to_string_lossy().to_string(),
            mount_mode: "symlink".to_string(),
            status: "UNCHECKED".to_string(),
            managed_by_app: true,
            backup_path: None,
            content_hash: None,
            is_outdated: None,
            created_at: Utc::now().to_rfc3339(),
        };

        let skill = Skill {
            id: "skill-a-id".to_string(),
            repository_id: "repo-1".to_string(),
            name: "skill-a".to_string(),
            relative_path: "skills/skill-a".to_string(),
            canonical_path: repo_skills.to_string_lossy().to_string(),
            description: "test".to_string(),
            metadata_status: "valid".to_string(),
            content_hash: "hash".to_string(),
            file_count: 1,
            char_count: 10,
            last_seen_at: Utc::now().to_rfc3339(),
        };

        let res = diagnose_project_mounts(&temp.join("project"), &[broken_mount], &[skill]);

        assert_eq!(res.updated_mounts.len(), 1);
        assert_eq!(res.updated_mounts[0].status, "BROKEN");
        assert_eq!(res.unmanaged_dirs, vec!["unmanaged-skill"]);

        let _ = fs::remove_dir_all(&temp);
    }
}
