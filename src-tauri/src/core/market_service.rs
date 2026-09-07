use crate::core::link_engine::{jail_check, remove_mount};
use crate::core::scanner::scan_repository;
use crate::db::Database;
use crate::models::{
    AppError, MarketInstallRequest, MarketInstallResult, MarketSearchResponse, MarketSkillDetail,
    MarketSkillItem, MarketUninstallCheckResult, MarketUninstallRequest, MarketUninstallResult,
    OperationLog,
};
use chrono::Utc;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;
use walkdir::WalkDir;

/// 强制递归删除目录（在 Windows 下确保清理只读属性，防止 Git pack 只读导致删除失败）
pub fn remove_dir_all_force(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if let Ok(metadata) = entry.metadata() {
            let mut permissions = metadata.permissions();
            if permissions.readonly() {
                #[allow(clippy::permissions_set_readonly_false)]
                permissions.set_readonly(false);
                let _ = fs::set_permissions(entry.path(), permissions);
            }
        }
    }
    fs::remove_dir_all(dir)
}

/// 复制 Skill 目录到中央仓库，自动跳过 .git、.github 及系统元数据，确保中央仓库权威纯净
pub fn copy_skill_dir_clean(src: &Path, dst: &Path) -> Result<(), AppError> {
    fs::create_dir_all(dst).map_err(AppError::io_error)?;
    for entry in fs::read_dir(src).map_err(AppError::io_error)? {
        let entry = entry.map_err(AppError::io_error)?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(".git") || name == ".DS_Store" || name.starts_with(".github") {
            continue;
        }
        let ty = entry.file_type().map_err(AppError::io_error)?;
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        if ty.is_dir() {
            copy_skill_dir_clean(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(AppError::io_error)?;
        }
    }
    Ok(())
}

/// 净化技能名称以适配跨平台文件系统（尤其是 Windows NTFS 非法字符限制，如 `:` 等）
pub fn sanitize_skill_name(name: &str) -> String {
    name.replace(':', "-")
        .replace('*', "-")
        .replace('?', "-")
        .replace('"', "-")
        .replace('<', "-")
        .replace('>', "-")
        .replace('|', "-")
        .trim()
        .trim_matches('.')
        .to_string()
}

#[derive(Deserialize)]
struct RawSkillsShItem {
    id: Option<String>,
    #[serde(rename = "skillId")]
    skill_id: Option<String>,
    name: Option<String>,
    installs: Option<u64>,
    source: Option<String>,
}

#[derive(Deserialize)]
struct RawSkillsShResponse {
    skills: Option<Vec<RawSkillsShItem>>,
    #[allow(dead_code)]
    count: Option<usize>,
}

/// 检索 skills.sh 市场中的技能
pub async fn search_skills(
    query: &str,
    installed_names: &[String],
) -> Result<MarketSearchResponse, AppError> {
    let trimmed = query.trim();

    // 当搜索词为空时，查询常用关键词（如 skills），以便初次打开展示热门推荐
    let actual_query = if trimmed.is_empty() {
        "skills"
    } else if trimmed.chars().count() < 2 {
        // skills.sh 要求 query 至少 2 个字符（按字符计数，而非 UTF-8 字节数）
        return Ok(MarketSearchResponse {
            skills: Vec::new(),
            count: 0,
        });
    } else {
        trimmed
    };

    let mut url = reqwest::Url::parse("https://skills.sh/api/search")
        .map_err(|e| AppError::with_details("INVALID_URL", "无法构建请求 URL", e.to_string()))?;
    url.query_pairs_mut().append_pair("q", actual_query);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("skills-manager/0.1.0")
        .build()
        .map_err(|e| {
            AppError::with_details("HTTP_CLIENT_ERROR", "HTTP 客户端初始化失败", e.to_string())
        })?;

    let resp = client.get(url).send().await.map_err(|e| {
        AppError::with_details(
            "MARKET_SEARCH_FAILED",
            "请求市场服务超时或网络异常",
            e.to_string(),
        )
    })?;

    if !resp.status().is_success() {
        let status = resp.status();
        // 如果返回 400 Bad Request（例如特殊字符未通过 skills.sh 校验），友好返回空结果
        if status.as_u16() == 400 {
            return Ok(MarketSearchResponse {
                skills: Vec::new(),
                count: 0,
            });
        }
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::with_details(
            "MARKET_HTTP_ERROR",
            format!("市场接口返回异常状态码: {}", status),
            body,
        ));
    }

    let parsed: RawSkillsShResponse = resp.json().await.map_err(|e| {
        AppError::with_details("MARKET_PARSE_ERROR", "解析市场数据格式失败", e.to_string())
    })?;

    let raw_list = parsed.skills.unwrap_or_default();
    let mut skills = Vec::new();

    for item in raw_list {
        let source = match item.source {
            Some(s) if !s.trim().is_empty() && s.contains('/') => s.trim().to_string(),
            _ => continue, // 过滤无效或非规范仓库源
        };

        let skill_id = item.skill_id.unwrap_or_else(|| {
            item.id
                .clone()
                .and_then(|id| id.rsplit('/').next().map(|s| s.to_string()))
                .unwrap_or_default()
        });

        let name = item.name.unwrap_or_else(|| skill_id.clone());
        if name.is_empty() {
            continue;
        }

        let id = item
            .id
            .unwrap_or_else(|| format!("{}/{}", source, skill_id));
        let installs = item.installs.unwrap_or(0);

        let sanitized_name = sanitize_skill_name(&name);
        let sanitized_skill_id = sanitize_skill_name(&skill_id);

        // 判断是否已在本地安装（同时匹配原始名与跨平台净化名）
        let is_installed = installed_names.iter().any(|n| {
            n.eq_ignore_ascii_case(&name)
                || n.eq_ignore_ascii_case(&skill_id)
                || n.eq_ignore_ascii_case(&sanitized_name)
                || n.eq_ignore_ascii_case(&sanitized_skill_id)
        });

        skills.push(MarketSkillItem {
            id,
            skill_id,
            name,
            installs,
            source,
            is_installed,
        });
    }

    let count = skills.len();
    Ok(MarketSearchResponse { skills, count })
}

/// 候选技能目录评分与匹配规则
fn score_candidate_skill_dir(
    dir_path: &Path,
    skill_id: &str,
    skill_name: &str,
    skill_md_content: Option<&str>,
) -> u32 {
    let dir_name = dir_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let safe_id = sanitize_skill_name(skill_id);
    let safe_name = sanitize_skill_name(skill_name);

    // 1. 目录名完全一致（最高分 100）
    if dir_name.eq_ignore_ascii_case(skill_id)
        || dir_name.eq_ignore_ascii_case(skill_name)
        || dir_name.eq_ignore_ascii_case(&safe_id)
        || dir_name.eq_ignore_ascii_case(&safe_name)
    {
        return 100;
    }

    // 2. SKILL.md Frontmatter 中的 name 字段完全一致（满分 100）
    if let Some(content) = skill_md_content {
        if let Some(front_name) = extract_frontmatter_name(content) {
            if front_name.eq_ignore_ascii_case(skill_id)
                || front_name.eq_ignore_ascii_case(skill_name)
                || front_name.eq_ignore_ascii_case(&safe_id)
                || front_name.eq_ignore_ascii_case(&safe_name)
            {
                return 100;
            }
        }
    }

    // 3. 路径任一层级目录名完全匹配（高分 80）
    for comp in dir_path.components() {
        if let Some(c_str) = comp.as_os_str().to_str() {
            if c_str.eq_ignore_ascii_case(skill_id)
                || c_str.eq_ignore_ascii_case(skill_name)
                || c_str.eq_ignore_ascii_case(&safe_id)
                || c_str.eq_ignore_ascii_case(&safe_name)
            {
                return 80;
            }
        }
    }

    // 4. 目录名以 - 或 _ 为边界包含 skill_id 或 skill_name（避免 "task" 错误匹配 "ask"）
    let dir_lower = dir_name.to_lowercase();
    let id_lower = skill_id.to_lowercase();
    let name_lower = skill_name.to_lowercase();
    let safe_id_lower = safe_id.to_lowercase();
    let safe_name_lower = safe_name.to_lowercase();

    for token in &[&id_lower, &name_lower, &safe_id_lower, &safe_name_lower] {
        if token.is_empty() {
            continue;
        }
        if dir_lower.starts_with(&format!("{}-", token))
            || dir_lower.starts_with(&format!("{}_", token))
            || dir_lower.ends_with(&format!("-{}", token))
            || dir_lower.ends_with(&format!("_{}", token))
            || dir_lower.contains(&format!("-{}-", token))
            || dir_lower.contains(&format!("_{}_", token))
        {
            return 60;
        }
    }

    0
}

/// 递归扫描沙箱目录定位目标技能所在路径（支持最大 5 层深度搜索并避免误匹配）
pub fn find_skill_dir_recursive(
    root: &Path,
    skill_id: &str,
    skill_name: &str,
    max_depth: usize,
) -> Result<PathBuf, AppError> {
    let mut candidate_dirs_with_skill_md: Vec<PathBuf> = Vec::new();
    let mut scored_candidates: Vec<(PathBuf, u32)> = Vec::new();

    for entry in WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(|e| {
            let fname = e.file_name().to_string_lossy();
            !fname.starts_with(".git")
        })
        .flatten()
    {
        if entry.file_type().is_dir() {
            let path = entry.path();
            let skill_md_path = path.join("SKILL.md");
            if skill_md_path.exists() && skill_md_path.is_file() {
                candidate_dirs_with_skill_md.push(path.to_path_buf());

                let md_content = fs::read_to_string(&skill_md_path).ok();
                let score =
                    score_candidate_skill_dir(path, skill_id, skill_name, md_content.as_deref());

                if score == 100 {
                    // 满分直接返回
                    return Ok(path.to_path_buf());
                } else if score > 0 {
                    scored_candidates.push((path.to_path_buf(), score));
                }
            }
        }
    }

    // 若存在带分值的候选项，按得分降序择优选取
    if !scored_candidates.is_empty() {
        scored_candidates.sort_by(|a, b| b.1.cmp(&a.1));
        return Ok(scored_candidates.remove(0).0);
    }

    // 若未按名称匹配到，但整个仓库仅有 1 个包含 SKILL.md 的目录，直接采用该目录
    if candidate_dirs_with_skill_md.len() == 1 {
        return Ok(candidate_dirs_with_skill_md.remove(0));
    }

    Err(AppError::with_details(
        "SKILL_NOT_FOUND",
        "未能在此仓库中定位到包含 SKILL.md 的有效技能目录",
        format!("目标技能: {}, 源仓库内未找到匹配项", skill_name),
    ))
}

/// 解析 SKILL.md 中的 Frontmatter name 属性
fn extract_frontmatter_name(content: &str) -> Option<String> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let rest = &trimmed[3..];
    let end_idx = rest.find("---")?;
    let yaml_str = &rest[..end_idx];
    if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(yaml_str) {
        if let Some(name_val) = val.get("name").and_then(|v| v.as_str()) {
            return Some(name_val.to_string());
        }
    }
    None
}

/// 安装技能：浅克隆沙箱目录 -> 递归扫描定位 -> 权威纯净搬运 -> 重建索引
pub fn install_skill(
    central_repo_path: &Path,
    req: MarketInstallRequest,
    db: &Database,
) -> Result<MarketInstallResult, AppError> {
    let repo_opt = db.get_repository()?;
    let repo = repo_opt.ok_or_else(|| AppError::new("NO_REPO", "尚未登记中央仓库，无法安装"))?;

    // 校验中央仓库目录与 skills 目录
    let central_skills_dir = central_repo_path.join("skills");
    if !central_skills_dir.exists() {
        fs::create_dir_all(&central_skills_dir).map_err(AppError::io_error)?;
    }

    // 校验技能名称安全性，防止路径穿越
    let raw_skill_name = req.skill_name.trim();
    if raw_skill_name.is_empty()
        || raw_skill_name.contains('/')
        || raw_skill_name.contains('\\')
        || raw_skill_name.contains("..")
    {
        return Err(AppError::new(
            "INVALID_SKILL_NAME",
            "非法技能名称，禁止包含路径分隔符或父路径操作",
        ));
    }

    let safe_skill_name = sanitize_skill_name(raw_skill_name);
    let target_dest = central_skills_dir.join(&safe_skill_name);

    // Jail 校验：目标必须严格在 central_repo/skills 内
    jail_check(&central_skills_dir, &target_dest)?;

    if target_dest.exists() {
        if req.force_overwrite != Some(true) {
            return Err(AppError::target_conflict(format!(
                "中央仓库已存在名为 '{}' 的 Skill，如需重新安装请确认覆盖",
                safe_skill_name
            )));
        } else {
            remove_dir_all_force(&target_dest).map_err(AppError::io_error)?;
        }
    }

    // 建立临时克隆沙箱
    let sandbox_id = format!("skills_mkt_{}", Uuid::new_v4());
    let temp_sandbox = std::env::temp_dir().join(sandbox_id);
    let _ = fs::create_dir_all(&temp_sandbox);

    // 确保退出时清理临时沙箱目录
    let result = (|| -> Result<MarketInstallResult, AppError> {
        let clone_url = format!("https://github.com/{}.git", req.source.trim());
        crate::core::logger::info(
            "Market",
            &format!(
                "正在浅克隆技能源码库: {} -> {}",
                clone_url,
                temp_sandbox.display()
            ),
        );

        // 设置环境变量禁用控制台交互与凭据弹窗，防止网络不通或私有库导致后台挂死
        let clone_output = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1")
            .arg(&clone_url)
            .arg(&temp_sandbox)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GCM_INTERACTIVE", "never")
            .env("GIT_ASKPASS", "")
            .output()
            .map_err(AppError::io_error)?;

        if !clone_output.status.success() {
            let stderr = String::from_utf8_lossy(&clone_output.stderr).to_string();
            return Err(AppError::with_details(
                "GIT_CLONE_FAILED",
                "克隆开源技能仓库失败，请检查网络连接或仓库是否公开存在",
                stderr,
            ));
        }

        // 递归检索目标技能目录
        let found_skill_dir =
            find_skill_dir_recursive(&temp_sandbox, &req.skill_id, raw_skill_name, 5)?;

        crate::core::logger::info(
            "Market",
            &format!(
                "定位到目标技能目录: {}，准备复制至中央仓库: {}",
                found_skill_dir.display(),
                target_dest.display()
            ),
        );

        // 搬运目录并安全跳过 .git / .github 等 VCS 元数据
        copy_skill_dir_clean(&found_skill_dir, &target_dest)?;

        // 写入 .skill-meta.json 记录元数据
        let meta = crate::models::SkillMetaFile {
            name: safe_skill_name.clone(),
            source: req.source.trim().to_string(),
            skill_id: req.skill_id.clone(),
            installed_at: Utc::now().to_rfc3339(),
            updated_at: None,
        };
        let _ = fs::write(
            target_dest.join(".skill-meta.json"),
            serde_json::to_string_pretty(&meta).unwrap_or_default(),
        );

        // 触发本地中央仓库重新扫描入库
        let (skills, _) = scan_repository(&repo.path, &repo.id)?;
        db.upsert_skills(&skills)?;

        // 显式保证数据库记录了 source 与 remote_skill_id
        let _ = db.set_skill_source(&repo.id, &safe_skill_name, &req.source, &req.skill_id);

        // 记录安装成功日志
        let log = OperationLog {
            id: Uuid::new_v4().to_string(),
            batch_id: None,
            operation_type: "market_install".to_string(),
            entity_type: "skill".to_string(),
            entity_id: None,
            project_id: None,
            skill_name: Some(safe_skill_name.clone()),
            target_path: Some(target_dest.to_string_lossy().to_string()),
            backup_path: None,
            status: "SUCCESS".to_string(),
            error_code: None,
            message: format!(
                "从市场成功安装 Skill: {} (源: {})",
                safe_skill_name, req.source
            ),
            created_at: Utc::now().to_rfc3339(),
        };
        let _ = db.insert_log(&log);

        Ok(MarketInstallResult {
            success: true,
            skill_name: safe_skill_name.clone(),
            message: format!("技能 '{}' 安装成功", safe_skill_name),
        })
    })();

    // 无论安装成功还是失败，均安全清理沙箱临时文件
    let _ = remove_dir_all_force(&temp_sandbox);

    result
}

/// 检查技能卸载关联影响（是否被业务项目挂载引用）
pub fn check_skill_uninstall(
    skill_name: &str,
    db: &Database,
) -> Result<MarketUninstallCheckResult, AppError> {
    let safe_name = sanitize_skill_name(skill_name);
    let mut mounts = db.get_mounts_by_skill_name(skill_name)?;
    if skill_name != safe_name {
        let safe_mounts = db.get_mounts_by_skill_name(&safe_name)?;
        mounts.extend(safe_mounts);
    }

    let mut mounted_projects = Vec::new();
    for m in mounts {
        if let Ok(Some(proj)) = db.get_project_by_id(&m.project_id) {
            mounted_projects.push(proj.name);
        } else {
            mounted_projects.push(m.project_id);
        }
    }

    mounted_projects.sort();
    mounted_projects.dedup();
    let can_direct_delete = mounted_projects.is_empty();

    Ok(MarketUninstallCheckResult {
        skill_name: skill_name.to_string(),
        mounted_projects,
        can_direct_delete,
    })
}

/// 卸载技能：支持级联解除各项目软链接软引用 + 物理删除中央仓库目录
pub fn uninstall_skill(
    central_repo_path: &Path,
    req: MarketUninstallRequest,
    db: &Database,
) -> Result<MarketUninstallResult, AppError> {
    let repo_opt = db.get_repository()?;
    let repo = repo_opt.ok_or_else(|| AppError::new("NO_REPO", "尚未登记中央仓库"))?;

    let raw_name = req.skill_name.trim();
    if raw_name.is_empty()
        || raw_name.contains('/')
        || raw_name.contains('\\')
        || raw_name.contains("..")
    {
        return Err(AppError::new(
            "INVALID_SKILL_NAME",
            "非法技能名称，禁止包含路径分隔符或父路径操作",
        ));
    }
    let safe_skill_name = sanitize_skill_name(raw_name);

    let mut mounts = db.get_mounts_by_skill_name(raw_name)?;
    if raw_name != safe_skill_name {
        let extra = db.get_mounts_by_skill_name(&safe_skill_name)?;
        mounts.extend(extra);
    }

    let mut unmounted_count = 0;

    if !mounts.is_empty() {
        if !req.cascade_unmount {
            return Err(AppError::new(
                "DEPENDENT_MOUNTS_EXIST",
                format!(
                    "技能 '{}' 当前被 {} 个项目挂载，请先解除项目挂载或勾选级联卸载",
                    req.skill_name,
                    mounts.len()
                ),
            ));
        }

        // 执行级联解绑
        for m in &mounts {
            if let Ok(Some(proj)) = db.get_project_by_id(&m.project_id) {
                let proj_path = Path::new(&proj.path);
                let agents_skills_dir = proj_path.join(".agents").join("skills");
                let link_path = PathBuf::from(&m.link_path);
                let _ = remove_mount(&link_path, &agents_skills_dir, &m.mount_mode);
            }
            let _ = db.delete_mount(&m.id);
            unmounted_count += 1;
        }
    }

    // 物理删除中央仓库目录
    let central_skills_dir = central_repo_path.join("skills");
    let target_dest = central_skills_dir.join(&safe_skill_name);
    let raw_dest = central_skills_dir.join(raw_name);

    if target_dest.exists() {
        jail_check(&central_skills_dir, &target_dest)?;
        remove_dir_all_force(&target_dest).map_err(AppError::io_error)?;
    }
    if raw_dest.exists() && raw_dest != target_dest {
        jail_check(&central_skills_dir, &raw_dest)?;
        remove_dir_all_force(&raw_dest).map_err(AppError::io_error)?;
    }

    // 从数据库中删除 Skill 记录
    db.delete_skill_by_name(&repo.id, raw_name)?;
    if raw_name != safe_skill_name {
        let _ = db.delete_skill_by_name(&repo.id, &safe_skill_name);
    }

    // 重新扫描以保证数据库状态完全一致
    let (skills, _) = scan_repository(&repo.path, &repo.id)?;
    db.upsert_skills(&skills)?;

    // 记录卸载日志
    let log = OperationLog {
        id: Uuid::new_v4().to_string(),
        batch_id: None,
        operation_type: "market_uninstall".to_string(),
        entity_type: "skill".to_string(),
        entity_id: None,
        project_id: None,
        skill_name: Some(req.skill_name.clone()),
        target_path: Some(target_dest.to_string_lossy().to_string()),
        backup_path: None,
        status: "SUCCESS".to_string(),
        error_code: None,
        message: format!(
            "已从中央仓库卸载 Skill: {} (级联解绑项目数: {})",
            req.skill_name, unmounted_count
        ),
        created_at: Utc::now().to_rfc3339(),
    };
    let _ = db.insert_log(&log);

    Ok(MarketUninstallResult {
        success: true,
        skill_name: req.skill_name,
        unmounted_count,
        message: "卸载成功".to_string(),
    })
}

/// 获取技能详细信息与 SKILL.md 正文
pub async fn get_skill_detail(
    central_repo_path: &Path,
    skill_id: &str,
    skill_name: &str,
    source: &str,
    installs: u64,
    db: &Database,
) -> Result<MarketSkillDetail, AppError> {
    let repo_opt = db.get_repository()?;
    let safe_name = sanitize_skill_name(skill_name);

    let is_installed = if let Some(repo) = repo_opt {
        db.get_skill_by_name(&repo.id, skill_name)?.is_some()
            || db.get_skill_by_name(&repo.id, &safe_name)?.is_some()
    } else {
        false
    };

    let local_path = if is_installed {
        let p = central_repo_path.join("skills").join(&safe_name);
        if p.exists() {
            Some(p.to_string_lossy().to_string())
        } else {
            let p_raw = central_repo_path.join("skills").join(skill_name);
            Some(p_raw.to_string_lossy().to_string())
        }
    } else {
        None
    };

    let mut content = None;
    if is_installed {
        let md_file = central_repo_path
            .join("skills")
            .join(&safe_name)
            .join("SKILL.md");
        if md_file.exists() {
            content = fs::read_to_string(&md_file).ok();
        } else {
            let md_raw = central_repo_path
                .join("skills")
                .join(skill_name)
                .join("SKILL.md");
            if md_raw.exists() {
                content = fs::read_to_string(&md_raw).ok();
            }
        }
    } else {
        // 对于未安装的技能，尝试从 raw.githubusercontent.com 请求公网说明文档（2秒超时，失败静默回退）
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(2))
            .build();
        if let Ok(c) = client {
            let candidate_urls = [
                format!(
                    "https://raw.githubusercontent.com/{}/main/{}/SKILL.md",
                    source, skill_id
                ),
                format!(
                    "https://raw.githubusercontent.com/{}/main/skills/{}/SKILL.md",
                    source, skill_id
                ),
                format!("https://raw.githubusercontent.com/{}/main/SKILL.md", source),
            ];
            for url in candidate_urls {
                if let Ok(resp) = c.get(&url).send().await {
                    if resp.status().is_success() {
                        if let Ok(text) = resp.text().await {
                            if !text.trim().is_empty() {
                                content = Some(text);
                                break;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(MarketSkillDetail {
        skill_id: skill_id.to_string(),
        name: skill_name.to_string(),
        source: source.to_string(),
        installs,
        is_installed,
        local_path,
        content,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_frontmatter_name() {
        let content = "---\nname: my-cool-skill\ndescription: A test skill\n---\n# Content\n";
        assert_eq!(
            extract_frontmatter_name(content),
            Some("my-cool-skill".to_string())
        );

        let invalid = "Just some markdown without frontmatter";
        assert_eq!(extract_frontmatter_name(invalid), None);
    }

    #[test]
    fn test_find_skill_dir_monorepo_recursive() {
        let temp_dir = std::env::temp_dir().join(format!("test_mono_{}", Uuid::new_v4()));
        let skill_path = temp_dir.join("skills").join("engineering").join("ask-matt");
        fs::create_dir_all(&skill_path).unwrap();
        fs::write(
            skill_path.join("SKILL.md"),
            "---\nname: ask-matt\n---\n# Ask Matt Skill\n",
        )
        .unwrap();

        let found = find_skill_dir_recursive(&temp_dir, "ask-matt", "ask-matt", 5).unwrap();
        assert_eq!(found, skill_path);

        let _ = remove_dir_all_force(&temp_dir);
    }

    #[test]
    fn test_find_skill_dir_no_false_positive_substring() {
        // 关键防御性测试：验证寻找 "ask" 不会被名字包含子串的 "task-runner" 误匹配
        let temp_dir = std::env::temp_dir().join(format!("test_ask_{}", Uuid::new_v4()));

        let task_runner_path = temp_dir.join("skills").join("task-runner");
        fs::create_dir_all(&task_runner_path).unwrap();
        fs::write(
            task_runner_path.join("SKILL.md"),
            "---\nname: task-runner\n---\n# Task Runner\n",
        )
        .unwrap();

        let ask_matt_path = temp_dir.join("skills").join("ask-matt");
        fs::create_dir_all(&ask_matt_path).unwrap();
        fs::write(
            ask_matt_path.join("SKILL.md"),
            "---\nname: ask-matt\n---\n# Ask Matt\n",
        )
        .unwrap();

        let found = find_skill_dir_recursive(&temp_dir, "ask", "ask", 5).unwrap();
        // 必须精确匹配到具有边界分词的 ask-matt，决不能匹配到包含 ask 子串的 task-runner
        assert_eq!(found, ask_matt_path);

        let _ = remove_dir_all_force(&temp_dir);
    }

    #[test]
    fn test_copy_skill_dir_clean_skips_git() {
        let temp_dir = std::env::temp_dir().join(format!("test_clean_{}", Uuid::new_v4()));
        let src = temp_dir.join("src");
        let dst = temp_dir.join("dst");

        fs::create_dir_all(src.join(".git").join("objects")).unwrap();
        fs::write(src.join(".git").join("HEAD"), "ref: refs/heads/main").unwrap();
        fs::write(src.join("SKILL.md"), "# Cool Skill").unwrap();
        fs::create_dir_all(src.join("scripts")).unwrap();
        fs::write(src.join("scripts").join("run.py"), "print('hi')").unwrap();

        copy_skill_dir_clean(&src, &dst).unwrap();

        assert!(dst.join("SKILL.md").exists());
        assert!(dst.join("scripts").join("run.py").exists());
        // 关键断言：.git 绝不能被拷贝到中央仓库目录中
        assert!(!dst.join(".git").exists());

        let _ = remove_dir_all_force(&temp_dir);
    }

    #[test]
    fn test_sanitize_skill_name_windows() {
        assert_eq!(sanitize_skill_name("sdd:add-task"), "sdd-add-task");
        assert_eq!(sanitize_skill_name("invalid*name?"), "invalid-name-");
        assert_eq!(sanitize_skill_name("normal-name"), "normal-name");
    }

    #[test]
    fn test_remove_dir_all_force() {
        let temp_dir = std::env::temp_dir().join(format!("test_rm_{}", Uuid::new_v4()));
        let sub = temp_dir.join("subdir");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("test.txt"), "hello").unwrap();

        assert!(temp_dir.exists());
        remove_dir_all_force(&temp_dir).unwrap();
        assert!(!temp_dir.exists());
    }
}
