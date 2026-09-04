use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static LOG_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);
static LOG_FILE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// 初始化文件日志目录与主日志文件
pub fn init_logger(dir: &Path) {
    let _ = fs::create_dir_all(dir);
    let log_file = dir.join("skills-manager.log");

    if let Ok(mut g_dir) = LOG_DIR.lock() {
        *g_dir = Some(dir.to_path_buf());
    }
    if let Ok(mut g_file) = LOG_FILE.lock() {
        *g_file = Some(log_file);
    }

    info(
        "App",
        &format!(
            "=== Skills 管理工具启动，日志系统已初始化: {} ===",
            dir.display()
        ),
    );
}

/// 获取日志目录
pub fn get_log_dir() -> PathBuf {
    if let Ok(g) = LOG_DIR.lock() {
        if let Some(ref d) = *g {
            return d.clone();
        }
    }
    std::env::temp_dir()
        .join("skills_manager_data")
        .join("logs")
}

/// 获取主日志文件绝对路径
pub fn get_log_path() -> PathBuf {
    if let Ok(g) = LOG_FILE.lock() {
        if let Some(ref f) = *g {
            return f.clone();
        }
    }
    get_log_dir().join("skills-manager.log")
}

/// 检查并按需轮转日志文件（默认超过 5MB 轮转）
fn rotate_if_needed(file_path: &Path) {
    if let Ok(metadata) = fs::metadata(file_path) {
        if metadata.len() > 5 * 1024 * 1024 {
            let backup2 = file_path.with_extension("log.2");
            let backup1 = file_path.with_extension("log.1");
            let _ = fs::rename(&backup1, &backup2);
            let _ = fs::rename(file_path, &backup1);
        }
    }
}

/// 底层日志写入函数
pub fn write_entry(level: &str, module: &str, message: &str) {
    let now = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] [{}] [{}] {}\n", now, level, module, message);

    // 同时输出到控制台
    eprint!("{}", line);

    let log_file = get_log_path();
    let _ = fs::create_dir_all(log_file.parent().unwrap_or(Path::new(".")));
    rotate_if_needed(&log_file);

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_file) {
        let _ = file.write_all(line.as_bytes());
    }
}

pub fn info(module: &str, message: &str) {
    write_entry("INFO", module, message);
}

pub fn warn(module: &str, message: &str) {
    write_entry("WARN", module, message);
}

pub fn error(module: &str, message: &str) {
    write_entry("ERROR", module, message);
}

/// 读取最近 N 行日志文本
pub fn read_recent_lines(max_lines: usize) -> String {
    let log_file = get_log_path();
    if !log_file.exists() {
        return "暂无日志内容".to_string();
    }

    if let Ok(file) = fs::File::open(&log_file) {
        let reader = BufReader::new(file);
        let lines: Vec<String> = reader.lines().flatten().collect();
        let total = lines.len();
        let start = if total > max_lines {
            total - max_lines
        } else {
            0
        };
        lines[start..].join("\n")
    } else {
        "无法读取日志文件".to_string()
    }
}

/// 清空当前日志文件
pub fn clear_log_file() -> std::io::Result<()> {
    let log_file = get_log_path();
    if log_file.exists() {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&log_file)?;
        let now = Local::now().format("%Y-%m-%d %H:%M:%S");
        file.write_all(format!("[{}] [INFO] [App] 日志已被用户清空\n", now).as_bytes())?;
    }
    Ok(())
}
