use crate::models::AppError;
use rusqlite::Connection;

pub fn initialize_tables(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS repositories (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            source_type TEXT NOT NULL DEFAULT 'local',
            git_remote TEXT,
            current_branch TEXT,
            last_scanned_at TEXT,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            repository_id TEXT NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
            name TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            canonical_path TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            metadata_status TEXT NOT NULL DEFAULT 'incomplete',
            content_hash TEXT NOT NULL DEFAULT '',
            file_count INTEGER NOT NULL DEFAULT 0,
            char_count INTEGER NOT NULL DEFAULT 0,
            last_seen_at TEXT NOT NULL,
            UNIQUE(repository_id, name)
        );

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            last_scanned_at TEXT,
            is_archived INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS agent_targets (
            id TEXT PRIMARY KEY,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            agent_type TEXT NOT NULL,
            install_dir TEXT NOT NULL DEFAULT '.agents/skills',
            link_mode TEXT NOT NULL DEFAULT 'symlink',
            UNIQUE(project_id, agent_type)
        );

        CREATE TABLE IF NOT EXISTS entry_links (
            id TEXT PRIMARY KEY,
            agent_target_id TEXT NOT NULL REFERENCES agent_targets(id) ON DELETE CASCADE,
            link_path TEXT NOT NULL,
            target_path TEXT NOT NULL DEFAULT '.agents/skills',
            status TEXT NOT NULL DEFAULT 'missing',
            UNIQUE(agent_target_id, link_path)
        );

        CREATE TABLE IF NOT EXISTS mounts (
            id TEXT PRIMARY KEY,
            skill_id TEXT NOT NULL,
            skill_name TEXT NOT NULL,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            link_path TEXT NOT NULL,
            resolved_target TEXT NOT NULL,
            mount_mode TEXT NOT NULL DEFAULT 'symlink',
            status TEXT NOT NULL DEFAULT 'UNCHECKED',
            managed_by_app INTEGER NOT NULL DEFAULT 1,
            backup_path TEXT,
            content_hash TEXT,
            created_at TEXT NOT NULL,
            UNIQUE(project_id, skill_name)
        );

        CREATE TABLE IF NOT EXISTS operation_logs (
            id TEXT PRIMARY KEY,
            batch_id TEXT,
            operation_type TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT,
            project_id TEXT,
            skill_name TEXT,
            target_path TEXT,
            backup_path TEXT,
            status TEXT NOT NULL,
            error_code TEXT,
            message TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS app_settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_mounts_project ON mounts(project_id);
        CREATE INDEX IF NOT EXISTS idx_skills_repo ON skills(repository_id);
        CREATE INDEX IF NOT EXISTS idx_logs_created_at ON operation_logs(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_logs_batch_id ON operation_logs(batch_id);
        ",
    )
    .map_err(AppError::db_error)?;

    Ok(())
}
