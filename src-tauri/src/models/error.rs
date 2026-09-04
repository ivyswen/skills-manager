use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for AppError {}

impl AppError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn path_not_found(msg: impl Into<String>) -> Self {
        Self::new("PATH_NOT_FOUND", msg)
    }

    pub fn path_not_readable(msg: impl Into<String>) -> Self {
        Self::new("PATH_NOT_READABLE", msg)
    }

    pub fn target_conflict(msg: impl Into<String>) -> Self {
        Self::new("TARGET_CONFLICT", msg)
    }

    pub fn broken_link(msg: impl Into<String>) -> Self {
        Self::new("BROKEN_LINK", msg)
    }

    pub fn wrong_target(msg: impl Into<String>) -> Self {
        Self::new("WRONG_TARGET", msg)
    }

    pub fn git_dirty(msg: impl Into<String>) -> Self {
        Self::new("GIT_DIRTY", msg)
    }

    pub fn git_auth_failed(msg: impl Into<String>) -> Self {
        Self::new("GIT_AUTH_FAILED", msg)
    }

    pub fn symlink_permission_denied(msg: impl Into<String>) -> Self {
        Self::new("SYMLINK_PERMISSION_DENIED", msg)
    }

    pub fn rollback_failed(msg: impl Into<String>) -> Self {
        Self::new("ROLLBACK_FAILED", msg)
    }

    pub fn io_error(e: std::io::Error) -> Self {
        Self::with_details("IO_ERROR", "文件系统操作异常", e.to_string())
    }

    pub fn db_error(e: rusqlite::Error) -> Self {
        Self::with_details("DATABASE_ERROR", "数据库操作异常", e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::io_error(e)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::db_error(e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::with_details("JSON_ERROR", "JSON 序列化失败", e.to_string())
    }
}
