use crate::core::link_engine::{calculate_relative_target, is_reparse_point, resolve_link_target};
use crate::models::AppError;
use std::fs;
use std::path::Path;

/// 检查并诊断入口链接状态
pub fn check_entry_link_status(project_path: &Path, rel_link_path: &str) -> String {
    let full_link_path = project_path.join(rel_link_path);
    let expected_target = project_path.join(".agents").join("skills");

    if !full_link_path.exists() && !is_reparse_point(&full_link_path) {
        return "missing".to_string();
    }

    if is_reparse_point(&full_link_path) {
        if let Some(target) = resolve_link_target(&full_link_path) {
            let expected_can = dunce::canonicalize(&expected_target).unwrap_or(expected_target);
            let target_can = dunce::canonicalize(&target).unwrap_or(target);
            if target_can == expected_can {
                "valid".to_string()
            } else {
                "conflict".to_string()
            }
        } else {
            "broken".to_string()
        }
    } else {
        // 是普通目录或普通文件
        "conflict".to_string()
    }
}

/// 创建或更新入口链接
pub fn setup_entry_link(
    project_path: &Path,
    rel_link_path: &str,
    preferred_mode: Option<&str>,
) -> Result<(), AppError> {
    let full_link_path = project_path.join(rel_link_path);
    let agents_skills = project_path.join(".agents").join("skills");

    // 确保目标 .agents/skills 存在
    if !agents_skills.exists() {
        fs::create_dir_all(&agents_skills).map_err(AppError::io_error)?;
    }

    // 确保入口链接父目录存在（例如 .claude）
    if let Some(parent) = full_link_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(AppError::io_error)?;
        }
    }

    // 幂等检查
    if is_reparse_point(&full_link_path) {
        if let Some(target) = resolve_link_target(&full_link_path) {
            let target_can = dunce::canonicalize(&target).unwrap_or(target);
            let expected_can = dunce::canonicalize(&agents_skills).unwrap_or(agents_skills.clone());
            if target_can == expected_can {
                return Ok(());
            }
        }
        // 目标不匹配，删除旧链接
        #[cfg(windows)]
        {
            if junction::exists(&full_link_path).unwrap_or(false) {
                let _ = junction::delete(&full_link_path);
            } else {
                let _ = fs::remove_dir(&full_link_path);
            }
        }
        #[cfg(unix)]
        {
            let _ = fs::remove_file(&full_link_path);
        }
    } else if full_link_path.exists() {
        return Err(AppError::target_conflict(format!(
            "入口链接目标路径 {} 已被普通目录或文件占用，禁止隐式覆盖",
            rel_link_path
        )));
    }

    let link_parent = full_link_path.parent().unwrap();
    let mode = preferred_mode.unwrap_or("symlink");

    match mode {
        "junction" => {
            #[cfg(windows)]
            {
                junction::create(&agents_skills, &full_link_path).map_err(|e| {
                    AppError::with_details(
                        "JUNCTION_FAILED",
                        "创建入口 Junction 失败",
                        e.to_string(),
                    )
                })?;
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&agents_skills, &full_link_path)
                    .map_err(AppError::io_error)?;
            }
        }
        _ => {
            // symlink 默认尝试相对软链接
            let rel_target = calculate_relative_target(link_parent, &agents_skills)?;
            #[cfg(windows)]
            {
                match std::os::windows::fs::symlink_dir(&rel_target, &full_link_path) {
                    Ok(_) => (),
                    Err(e) => {
                        // 降级为 Junction
                        junction::create(&agents_skills, &full_link_path).map_err(|je| {
                            AppError::with_details(
                                "SYMLINK_FAILED",
                                "创建入口软链接与 Junction 均失败",
                                format!("symlink: {}, junction: {}", e, je),
                            )
                        })?;
                    }
                }
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&rel_target, &full_link_path)
                    .map_err(AppError::io_error)?;
            }
        }
    }

    Ok(())
}

/// 删除入口链接
pub fn remove_entry_link(project_path: &Path, rel_link_path: &str) -> Result<(), AppError> {
    let full_link_path = project_path.join(rel_link_path);
    if !full_link_path.exists() && !is_reparse_point(&full_link_path) {
        return Ok(());
    }

    if is_reparse_point(&full_link_path) {
        #[cfg(windows)]
        {
            if junction::exists(&full_link_path).unwrap_or(false) {
                let _ = junction::delete(&full_link_path);
            }
        }
        let _ = fs::remove_dir(&full_link_path);
        let _ = fs::remove_file(&full_link_path);
        Ok(())
    } else {
        Err(AppError::target_conflict(
            "入口路径不是软链接或 Junction，拒绝删除普通文件",
        ))
    }
}
