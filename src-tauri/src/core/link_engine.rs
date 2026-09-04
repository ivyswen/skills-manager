use crate::models::{AppError, ConflictStrategy};
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

/// 检查两个路径是否属于同一个盘符/卷（在 Windows 下判断首个组件是否相同）
pub fn is_same_volume(p1: &Path, p2: &Path) -> bool {
    let p1_can = dunce::canonicalize(p1).unwrap_or_else(|_| p1.to_path_buf());
    let p2_can = dunce::canonicalize(p2).unwrap_or_else(|_| p2.to_path_buf());

    let prefix1 = p1_can.components().next();
    let prefix2 = p2_can.components().next();

    match (prefix1, prefix2) {
        (Some(c1), Some(c2)) => c1 == c2,
        _ => true,
    }
}

/// 计算从 link_dir 到 target 的相对路径
pub fn calculate_relative_target(link_dir: &Path, target: &Path) -> Result<PathBuf, AppError> {
    let target_can = dunce::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
    let link_dir_can = dunce::canonicalize(link_dir).unwrap_or_else(|_| link_dir.to_path_buf());

    if !is_same_volume(&link_dir_can, &target_can) {
        return Err(AppError::symlink_permission_denied(
            "跨物理卷无法创建相对符号链接，需使用 Junction 或 Copy 模式",
        ));
    }

    pathdiff::diff_paths(&target_can, &link_dir_can)
        .ok_or_else(|| AppError::symlink_permission_denied("无法计算相对路径"))
}

/// 检查路径是否是符号链接或 Junction
pub fn is_reparse_point(path: &Path) -> bool {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return true;
        }
    }
    #[cfg(windows)]
    {
        if junction::exists(path).unwrap_or(false) {
            return true;
        }
    }
    false
}

/// 解析软链接或 Junction 的实际目标绝对路径
pub fn resolve_link_target(path: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if junction::exists(path).unwrap_or(false) {
            if let Ok(target) = junction::get_target(path) {
                let target_str = target.to_string_lossy();
                let clean_str = if let Some(stripped) = target_str.strip_prefix(r"\??\") {
                    stripped
                } else if let Some(stripped) = target_str.strip_prefix(r"\\?\") {
                    stripped
                } else {
                    &target_str
                };
                let clean_path = PathBuf::from(clean_str);
                return Some(dunce::canonicalize(&clean_path).unwrap_or(clean_path));
            }
        }
    }

    if let Ok(target) = fs::read_link(path) {
        let target_str = target.to_string_lossy();
        let clean_str = if let Some(stripped) = target_str.strip_prefix(r"\??\") {
            stripped
        } else if let Some(stripped) = target_str.strip_prefix(r"\\?\") {
            stripped
        } else {
            &target_str
        };
        let clean_target = PathBuf::from(clean_str);
        let full = if clean_target.is_relative() {
            if let Some(parent) = path.parent() {
                parent.join(clean_target)
            } else {
                clean_target
            }
        } else {
            clean_target
        };
        return Some(dunce::canonicalize(&full).unwrap_or(full));
    }

    None
}

/// 递归复制目录（用于 Copy 模式）
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst).map_err(AppError::io_error)?;
    for entry in fs::read_dir(src).map_err(AppError::io_error)? {
        let entry = entry.map_err(AppError::io_error)?;
        let ty = entry.file_type().map_err(AppError::io_error)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(AppError::io_error)?;
        }
    }
    Ok(())
}

/// 安全 Jail 校验：确保 target 路径在 parent 目录之内，防止逃逸
pub fn jail_check(parent: &Path, target: &Path) -> Result<(), AppError> {
    let parent_can = dunce::canonicalize(parent).unwrap_or_else(|_| parent.to_path_buf());
    // 注意：不能直接对 target 调 canonicalize，因为若 target 是符号链接或 Junction，
    // canonicalize 会跟随链接解析到外部原件路径！
    // 我们需要校验 target 所在的目录是否在 parent 之内或就是 parent 本身。
    let target_parent = match target.parent() {
        Some(p) => p,
        None => return Err(AppError::new("JAIL_VIOLATION", "目标路径无父目录")),
    };
    let target_parent_can =
        dunce::canonicalize(target_parent).unwrap_or_else(|_| target_parent.to_path_buf());

    if !target_parent_can.starts_with(&parent_can) && target_parent_can != parent_can {
        return Err(AppError::new(
            "JAIL_VIOLATION",
            "目标路径越界，严禁在项目目录外执行破坏性操作",
        ));
    }
    Ok(())
}

/// 创建挂载：支持两阶段预检与原子回滚
pub fn create_mount(
    source_canonical_path: &Path,
    target_link_path: &Path,
    preferred_mode: Option<&str>,
    conflict_strategy: &ConflictStrategy,
) -> Result<(String, Option<PathBuf>), AppError> {
    let source_can = dunce::canonicalize(source_canonical_path)
        .map_err(|e| AppError::path_not_found(format!("原件不存在: {}", e)))?;

    let link_parent = target_link_path
        .parent()
        .ok_or_else(|| AppError::path_not_found("无效的挂载目标路径"))?;

    // 确保目标父目录存在
    if !link_parent.exists() {
        fs::create_dir_all(link_parent).map_err(AppError::io_error)?;
    }

    let mut backup_dir_opt: Option<PathBuf> = None;

    // 检查目标是否存在
    if target_link_path.exists() || is_reparse_point(target_link_path) {
        // 判断是否已经是正常指向该原件的链接（幂等）
        if let Some(resolved) = resolve_link_target(target_link_path) {
            if resolved == source_can {
                return Ok(("symlink".to_string(), None));
            }
        }

        // 存在冲突
        match conflict_strategy {
            ConflictStrategy::Cancel => {
                return Err(AppError::target_conflict(
                    "目标路径已存在冲突对象，操作已取消",
                ));
            }
            ConflictStrategy::Skip => {
                return Err(AppError::target_conflict("目标路径已存在冲突对象，已跳过"));
            }
            ConflictStrategy::BackupAndReplace => {
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let skill_name = target_link_path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                let backup_root = link_parent.join(".backup");
                fs::create_dir_all(&backup_root).map_err(AppError::io_error)?;

                let backup_dest = backup_root.join(format!("{}_{}", timestamp, skill_name));
                fs::rename(target_link_path, &backup_dest).map_err(|e| {
                    AppError::target_conflict(format!("移动冲突对象至备份目录失败: {}", e))
                })?;
                backup_dir_opt = Some(backup_dest);
            }
        }
    }

    // 执行挂载创建，附带失败回滚逻辑
    let actual_mode_result = try_create_link(&source_can, target_link_path, preferred_mode);

    match actual_mode_result {
        Ok(mode) => Ok((mode, backup_dir_opt)),
        Err(e) => {
            // 回滚备份对象
            if let Some(ref backup_path) = backup_dir_opt {
                if backup_path.exists() {
                    // 若 target_link_path 已被部分创建，必须先清理以避免 Windows rename 冲突
                    if target_link_path.exists() || is_reparse_point(target_link_path) {
                        let _ = fs::remove_dir_all(target_link_path);
                        let _ = fs::remove_file(target_link_path);
                    }
                    let _ = fs::rename(backup_path, target_link_path);
                }
            }
            Err(e)
        }
    }
}

/// 尝试不同模式创建链接
fn try_create_link(
    source: &Path,
    target: &Path,
    preferred_mode: Option<&str>,
) -> Result<String, AppError> {
    let link_parent = target.parent().unwrap();
    let same_vol = is_same_volume(link_parent, source);

    let mode = preferred_mode.unwrap_or(if same_vol { "symlink" } else { "junction" });

    match mode {
        "symlink" => {
            if !same_vol {
                return Err(AppError::symlink_permission_denied(
                    "跨盘符不支持相对符号链接，请选择 Junction 或 Copy 模式",
                ));
            }

            let rel_target = calculate_relative_target(link_parent, source)?;

            #[cfg(windows)]
            {
                match std::os::windows::fs::symlink_dir(&rel_target, target) {
                    Ok(_) => Ok("symlink".to_string()),
                    Err(e) => {
                        let raw_err = e.raw_os_error();
                        // 1314: ERROR_PRIVILEGE_NOT_HELD (未开启开发者模式且未提权)
                        if raw_err == Some(1314) || e.kind() == std::io::ErrorKind::PermissionDenied
                        {
                            Err(AppError::symlink_permission_denied(
                                "Windows 创建符号链接权限不足 (Win32 1314)。请在系统设置中开启“开发者模式”，或降级选择 Junction / Copy 模式。"
                            ))
                        } else {
                            Err(AppError::io_error(e))
                        }
                    }
                }
            }
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(&rel_target, target).map_err(AppError::io_error)?;
                Ok("symlink".to_string())
            }
        }
        "junction" => {
            #[cfg(windows)]
            {
                junction::create(source, target).map_err(|e| {
                    AppError::with_details(
                        "JUNCTION_FAILED",
                        "创建 NTFS Junction 失败",
                        e.to_string(),
                    )
                })?;
                Ok("junction".to_string())
            }
            #[cfg(unix)]
            {
                // Unix 不支持 Junction，自动降级为绝对软链接
                std::os::unix::fs::symlink(source, target).map_err(AppError::io_error)?;
                Ok("symlink".to_string())
            }
        }
        "copy" => {
            copy_dir_all(source, target)?;
            Ok("copy".to_string())
        }
        _ => Err(AppError::new("INVALID_MODE", "未知的挂载模式")),
    }
}

/// 安全卸载挂载对象
pub fn remove_mount(
    target_path: &Path,
    project_agents_skills_dir: &Path,
    mode: &str,
) -> Result<(), AppError> {
    if !target_path.exists() && !is_reparse_point(target_path) {
        // 目标已不存在，视作成功
        return Ok(());
    }

    // 强校验路径在 `.agents/skills` 内部
    jail_check(project_agents_skills_dir, target_path)?;

    if is_reparse_point(target_path) {
        // Reparse Point (符号链接或 Junction)
        #[cfg(windows)]
        {
            if junction::exists(target_path).unwrap_or(false) {
                let _ = junction::delete(target_path);
            }
        }
        // 软链接或 Junction 目录仅调用 remove_dir，决不能调用 remove_dir_all
        let _ = fs::remove_dir(target_path);
        let _ = fs::remove_file(target_path);

        // 关键断言：必须用 symlink_metadata 校验链接本身是否已被清除（不能使用 follows link 的 exists()）
        if fs::symlink_metadata(target_path).is_ok() {
            return Err(AppError::with_details(
                "UNMOUNT_FAILED",
                "删除符号链接失败，目标对象仍然残留",
                target_path.display().to_string(),
            ));
        }
        Ok(())
    } else if mode == "copy" {
        // Copy 模式是实体目录，经过 Jail 校验后调用 remove_dir_all
        fs::remove_dir_all(target_path).map_err(AppError::io_error)?;
        Ok(())
    } else {
        // 普通非链接目录但记录为 symlink/junction，提示冲突
        Err(AppError::target_conflict(
            "目标为非软链接的普通目录，拒绝直接删除，请使用冲突管理流程",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pathdiff_relative_calculation() {
        let base = Path::new("C:/project/.agents/skills");
        let target = Path::new("C:/repo/skills/skill-a");
        let rel = pathdiff::diff_paths(target, base).unwrap();
        assert_eq!(rel, Path::new("../../../repo/skills/skill-a"));
    }

    #[test]
    fn test_two_phase_mount_and_rollback() {
        let temp = std::env::temp_dir().join("test_link_engine_phase2");
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(&temp).unwrap();

        let source = temp.join("source_skill");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("SKILL.md"), "test").unwrap();

        let project_skills = temp.join("project").join(".agents").join("skills");
        fs::create_dir_all(&project_skills).unwrap();

        let target = project_skills.join("my-skill");
        // 预置冲突普通文件
        fs::write(&target, "conflict content").unwrap();

        // 选用 BackupAndReplace 策略以 Copy 模式挂载
        let (mode, backup) = create_mount(
            &source,
            &target,
            Some("copy"),
            &ConflictStrategy::BackupAndReplace,
        )
        .unwrap();

        assert_eq!(mode, "copy");
        assert!(backup.is_some());
        let bpath = backup.unwrap();
        assert!(bpath.exists());
        assert_eq!(fs::read_to_string(&bpath).unwrap(), "conflict content");

        // 验证 target 现在是新目录
        assert!(target.is_dir());
        assert!(target.join("SKILL.md").exists());

        // 卸载
        remove_mount(&target, &project_skills, "copy").unwrap();
        assert!(!target.exists());

        let _ = fs::remove_dir_all(&temp);
    }
}
