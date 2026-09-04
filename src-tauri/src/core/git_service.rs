use crate::models::{AffectedSkill, AppError, GitPullResult, GitStatusResult};
use std::path::Path;
use std::process::Command;

pub fn check_is_git_repo(repo_path: &Path) -> bool {
    repo_path.join(".git").exists()
}

pub fn get_git_status(repo_path: &Path) -> Result<GitStatusResult, AppError> {
    if !check_is_git_repo(repo_path) {
        return Ok(GitStatusResult {
            branch: String::new(),
            commit_hash: String::new(),
            commit_message: String::new(),
            is_dirty: false,
            changed_files: Vec::new(),
            error: Some("不是 Git 仓库".to_string()),
        });
    }

    // 分支
    let branch_output = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(repo_path)
        .output()
        .map_err(AppError::io_error)?;
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Commit 信息
    let log_output = Command::new("git")
        .arg("log")
        .arg("-1")
        .arg("--format=%H|%s")
        .current_dir(repo_path)
        .output()
        .map_err(AppError::io_error)?;
    let log_str = String::from_utf8_lossy(&log_output.stdout)
        .trim()
        .to_string();
    let parts: Vec<&str> = log_str.splitn(2, '|').collect();
    let commit_hash = parts.first().copied().unwrap_or("").to_string();
    let commit_message = parts.get(1).copied().unwrap_or("").to_string();

    // 脏工作区检查
    let status_output = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .current_dir(repo_path)
        .output()
        .map_err(AppError::io_error)?;
    let status_str = String::from_utf8_lossy(&status_output.stdout);
    let mut changed_files = Vec::new();
    for line in status_str.lines() {
        let l = line.trim();
        if !l.is_empty() {
            changed_files.push(l.to_string());
        }
    }

    let is_dirty = !changed_files.is_empty();

    Ok(GitStatusResult {
        branch,
        commit_hash,
        commit_message,
        is_dirty,
        changed_files,
        error: None,
    })
}

pub fn get_git_remote(repo_path: &Path) -> Option<String> {
    if !check_is_git_repo(repo_path) {
        return None;
    }
    let output = Command::new("git")
        .arg("config")
        .arg("--get")
        .arg("remote.origin.url")
        .current_dir(repo_path)
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// 解析 git diff 变更记录（去重并按 deleted > renamed > modified 优先级合并）
pub fn parse_git_diff_affected(diff_output: &str) -> Vec<AffectedSkill> {
    use std::collections::HashMap;
    let mut affected_map: HashMap<String, String> = HashMap::new();

    for line in diff_output.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.is_empty() {
            continue;
        }
        let status = parts[0];
        let (change_type, path_str) = if status.starts_with('D') && parts.len() >= 2 {
            ("deleted", parts[1])
        } else if status.starts_with('R') && parts.len() >= 3 {
            ("renamed", parts[1])
        } else if status.starts_with('M') && parts.len() >= 2 {
            ("modified", parts[1])
        } else {
            continue;
        };

        if let Some(skill_name) = extract_skill_name_from_diff(path_str) {
            affected_map
                .entry(skill_name)
                .and_modify(|existing| {
                    if *existing != "deleted"
                        && (change_type == "deleted" || change_type == "renamed")
                    {
                        *existing = change_type.to_string();
                    }
                })
                .or_insert_with(|| change_type.to_string());
        }
    }

    let mut result: Vec<AffectedSkill> = affected_map
        .into_iter()
        .map(|(name, change_type)| AffectedSkill {
            name,
            change_type,
            affected_project_ids: Vec::new(),
        })
        .collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));
    result
}

fn extract_skill_name_from_diff(path_str: &str) -> Option<String> {
    let p = Path::new(path_str);
    let mut components = p.components();
    // 预期形如 skills/skill-name/...
    if let Some(first) = components.next() {
        if first.as_os_str() == "skills" {
            if let Some(second) = components.next() {
                return Some(second.as_os_str().to_string_lossy().to_string());
            }
        }
    }
    None
}

/// 执行 git pull，带脏检查与 diff 感知
pub fn execute_git_pull(repo_path: &Path) -> Result<GitPullResult, AppError> {
    if !check_is_git_repo(repo_path) {
        return Err(AppError::with_details(
            "NOT_GIT_REPO",
            "当前仓库不是 Git 仓库",
            "",
        ));
    }

    // 脏检查
    let status = get_git_status(repo_path)?;
    if status.is_dirty {
        return Err(AppError::git_dirty(
            "中央仓库工作区存在未提交修改，更新操作已中断。请在外部提交或取消修改后再试。",
        ));
    }

    let old_commit = status.commit_hash;

    let pull_output = Command::new("git")
        .arg("pull")
        .current_dir(repo_path)
        .output()
        .map_err(AppError::io_error)?;

    if !pull_output.status.success() {
        let err_msg = String::from_utf8_lossy(&pull_output.stderr).to_string();
        if err_msg.contains("Authentication failed")
            || err_msg.contains("Permission denied (publickey)")
        {
            return Err(AppError::git_auth_failed(format!(
                "Git 认证失败: {}",
                err_msg
            )));
        }
        return Err(AppError::with_details(
            "GIT_PULL_FAILED",
            "Git 拉取失败",
            err_msg,
        ));
    }

    let new_status = get_git_status(repo_path)?;
    let new_commit = new_status.commit_hash;

    let mut affected_skills = Vec::new();
    let updated = old_commit != new_commit;

    if updated && !old_commit.is_empty() && !new_commit.is_empty() {
        let diff_output = Command::new("git")
            .arg("diff")
            .arg("--name-status")
            .arg(format!("{}..{}", old_commit, new_commit))
            .arg("--")
            .arg("skills")
            .current_dir(repo_path)
            .output()
            .map_err(AppError::io_error)?;

        let diff_str = String::from_utf8_lossy(&diff_output.stdout);
        affected_skills = parse_git_diff_affected(&diff_str);
    }

    Ok(GitPullResult {
        success: true,
        updated,
        old_commit,
        new_commit,
        affected_skills,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_diff_parser() {
        let diff_text = "D\tskills/old-skill/SKILL.md\nR100\tskills/renamed-skill/SKILL.md\tskills/new-name/SKILL.md\nM\tskills/mod-skill/SKILL.md\nM\tskills/mod-skill/helper.py\n";
        let affected = parse_git_diff_affected(diff_text);
        assert_eq!(affected.len(), 3);
        assert_eq!(affected[0].name, "mod-skill");
        assert_eq!(affected[0].change_type, "modified");
        assert_eq!(affected[1].name, "old-skill");
        assert_eq!(affected[1].change_type, "deleted");
        assert_eq!(affected[2].name, "renamed-skill");
        assert_eq!(affected[2].change_type, "renamed");
    }
}
