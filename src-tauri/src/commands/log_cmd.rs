use crate::core::logger;
use crate::db::Database;
use crate::models::{AppError, OperationLog};
use serde::Serialize;
use tauri::{AppHandle, State};

#[derive(Serialize)]
pub struct LogFileInfo {
    pub path: String,
    pub dir: String,
    pub size_bytes: u64,
    pub exists: bool,
}

#[tauri::command]
pub async fn operation_log_query(
    project_id: Option<String>,
    limit: Option<usize>,
    db: State<'_, Database>,
) -> Result<Vec<OperationLog>, AppError> {
    let lim = limit.unwrap_or(100);
    db.query_logs(project_id.as_deref(), lim)
}

#[tauri::command]
pub async fn log_get_info() -> Result<LogFileInfo, AppError> {
    let path = logger::get_log_path();
    let dir = logger::get_log_dir();
    let (exists, size) = if let Ok(meta) = std::fs::metadata(&path) {
        (true, meta.len())
    } else {
        (false, 0)
    };
    Ok(LogFileInfo {
        path: path.to_string_lossy().to_string(),
        dir: dir.to_string_lossy().to_string(),
        size_bytes: size,
        exists,
    })
}

#[tauri::command]
pub async fn log_read_text(lines: Option<usize>) -> Result<String, AppError> {
    let lim = lines.unwrap_or(200);
    Ok(logger::read_recent_lines(lim))
}

#[tauri::command]
pub async fn log_clear() -> Result<(), AppError> {
    logger::clear_log_file().map_err(|e| AppError::new("LOG_CLEAR_FAILED", e.to_string()))
}

#[tauri::command]
pub async fn log_open_dir(_app: AppHandle) -> Result<(), AppError> {
    let dir = logger::get_log_dir();
    let _ = std::fs::create_dir_all(&dir);
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&dir).spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        use tauri_plugin_opener::OpenerExt;
        let _ = _app.opener().open_path(dir.to_string_lossy(), None::<&str>);
    }
    Ok(())
}

#[tauri::command]
pub async fn log_open_file(_app: AppHandle) -> Result<(), AppError> {
    let file = logger::get_log_path();
    if !file.exists() {
        let _ = std::fs::write(&file, "Skills Manager 日志文件\n");
    }
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("notepad.exe").arg(&file).spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        use tauri_plugin_opener::OpenerExt;
        let _ = _app
            .opener()
            .open_path(file.to_string_lossy(), None::<&str>);
    }
    Ok(())
}
