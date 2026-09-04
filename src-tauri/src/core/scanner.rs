use crate::models::{AppError, Skill};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

/// 解析 Frontmatter 或首段内容
fn parse_skill_md(content: &str, folder_name: &str) -> (String, String, String) {
    let normalized = content.replace("\r\n", "\n");
    let trimmed = normalized.trim();
    if trimmed.starts_with("---") {
        if let Some(end_idx) = trimmed[3..].find("---") {
            let frontmatter_str = &trimmed[3..3 + end_idx];
            let body_str = &trimmed[3 + end_idx + 3..];

            // 尝试解析 YAML
            if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(frontmatter_str) {
                // PRD 6.1.2: Skill 名称优先取目录名
                let name = folder_name.to_string();
                let description = yaml
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if !description.is_empty() {
                    return (name, description, "valid".to_string());
                } else {
                    // 若无 description，截取正文首段
                    let desc = extract_first_paragraph(body_str);
                    return (name, desc, "incomplete".to_string());
                }
            }
        }
    }

    // 无 Frontmatter 或解析失败，截取正文首段有效纯文本段落（<= 300 字）
    let desc = extract_first_paragraph(trimmed);
    (folder_name.to_string(), desc, "incomplete".to_string())
}

/// 截取首个非空非标题段落（<= 300 字）
fn extract_first_paragraph(text: &str) -> String {
    let normalized = text.replace("\r\n", "\n");
    for para in normalized.split("\n\n") {
        let p = para.trim();
        if p.is_empty() || p.starts_with('#') {
            continue;
        }
        let clean = p.replace('\n', " ");
        let mut chars: Vec<char> = clean.chars().collect();
        if chars.len() > 300 {
            chars.truncate(300);
            return format!("{}...", chars.into_iter().collect::<String>());
        }
        return chars.into_iter().collect();
    }
    String::new()
}

/// 判断文件是否为二进制（探测前 1024 字节）
fn is_binary_file(path: &Path) -> bool {
    if let Ok(file) = fs::File::open(path) {
        use std::io::Read;
        let mut buffer = [0u8; 1024];
        let mut reader = std::io::BufReader::new(file);
        if let Ok(n) = reader.read(&mut buffer) {
            if buffer[..n].contains(&0) {
                return true;
            }
        }
    }
    false
}

/// 计算 Skill 目录哈希与度量（文件数、UTF-8 字符数）
fn measure_skill_dir(dir: &Path) -> (u32, u64, String) {
    let mut file_count: u32 = 0;
    let mut char_count: u64 = 0;
    let mut hasher = Sha256::new();

    // 收集所有文件并排序，保证 Hash 确定性
    let mut entries: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !name.starts_with(".git") && name != ".DS_Store"
    }) {
        if let Ok(e) = entry {
            if e.file_type().is_file() {
                entries.push(e.path().to_path_buf());
            }
        }
    }
    entries.sort();

    for file_path in entries {
        file_count += 1;
        if let Ok(rel) = file_path.strip_prefix(dir) {
            hasher.update(rel.to_string_lossy().as_bytes());
        }

        if let Ok(bytes) = fs::read(&file_path) {
            hasher.update(&bytes);

            // 如果不是二进制文件，统计字符数
            if !is_binary_file(&file_path) {
                if let Ok(s) = std::str::from_utf8(&bytes) {
                    char_count += s.chars().count() as u64;
                }
            }
        }
    }

    let hash_result = format!("{:x}", hasher.finalize());
    (file_count, char_count, hash_result)
}

/// 扫描指定中央仓库中的所有 Skill
pub fn scan_repository(
    repo_path_str: &str,
    repo_id: &str,
) -> Result<(Vec<Skill>, Vec<String>), AppError> {
    let repo_path = Path::new(repo_path_str);
    if !repo_path.exists() || !repo_path.is_dir() {
        return Err(AppError::path_not_found("中央仓库目录不存在或不是目录"));
    }

    let skills_dir = repo_path.join("skills");
    let mut warnings = Vec::new();
    if !skills_dir.exists() || !skills_dir.is_dir() {
        warnings.push("仓库根目录下未找到 skills 目录，已识别为 0 个 Skill".to_string());
        return Ok((Vec::new(), warnings));
    }

    let entries = fs::read_dir(&skills_dir)
        .map_err(|e| AppError::path_not_readable(format!("无法读取 skills 目录: {}", e)))?;

    let mut skills = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = match path.file_name() {
            Some(n) => n.to_string_lossy().to_string(),
            None => continue,
        };

        // 仅识别包含 SKILL.md 的子目录
        let skill_md_path = path.join("SKILL.md");
        if !skill_md_path.exists() || !skill_md_path.is_file() {
            // 不含 SKILL.md，跳过
            continue;
        }

        // 读取 SKILL.md
        let md_content = fs::read_to_string(&skill_md_path).unwrap_or_default();
        let (name, description, metadata_status) = parse_skill_md(&md_content, &folder_name);

        let (file_count, char_count, content_hash) = measure_skill_dir(&path);
        let canonical = dunce::canonicalize(&path)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| path.to_string_lossy().to_string());

        let relative_path = format!("skills/{}", folder_name);

        skills.push(Skill {
            id: Uuid::new_v4().to_string(),
            repository_id: repo_id.to_string(),
            name,
            relative_path,
            canonical_path: canonical,
            description,
            metadata_status,
            content_hash,
            file_count,
            char_count,
            last_seen_at: Utc::now().to_rfc3339(),
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok((skills, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_scanner_yaml_frontmatter() {
        let content = r#"---
name: my-skill
description: This is a test skill.
---
# Content here
"#;
        let (name, desc, status) = parse_skill_md(content, "folder-x");
        // PRD 6.1.2 明确规定：Skill 名称优先取目录名
        assert_eq!(name, "folder-x");
        assert_eq!(desc, "This is a test skill.");
        assert_eq!(status, "valid");

        // 测试缺失 Frontmatter 与 Windows CRLF 换行
        let plain_crlf = "# Header\r\n\r\nFirst paragraph of text.\r\n\r\nSecond paragraph.";
        let (name2, desc2, status2) = parse_skill_md(plain_crlf, "folder-y");
        assert_eq!(name2, "folder-y");
        assert_eq!(desc2, "First paragraph of text.");
        assert_eq!(status2, "incomplete");
    }

    #[test]
    fn test_scanner_binary_filtering() {
        let dir = std::env::temp_dir().join("test_scanner_binary");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();

        let txt_file = dir.join("test.txt");
        fs::write(&txt_file, "Hello 你好").unwrap();

        let bin_file = dir.join("test.bin");
        let mut bin = File::create(&bin_file).unwrap();
        bin.write_all(&[0x00, 0x01, 0x02, 0xFF]).unwrap();

        assert!(!is_binary_file(&txt_file));
        assert!(is_binary_file(&bin_file));

        let (count, chars, _) = measure_skill_dir(&dir);
        assert_eq!(count, 2);
        // "Hello 你好" = 8 chars
        assert_eq!(chars, 8);

        let _ = fs::remove_dir_all(&dir);
    }
}
