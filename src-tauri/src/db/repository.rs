use crate::models::{
    AgentTarget, AppError, AppSetting, EntryLink, Mount, OperationLog, Project, Repository, Skill,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    pub fn in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory().map_err(AppError::db_error)?;
        crate::db::schema::initialize_tables(&conn)?;
        Ok(Self::new(conn))
    }

    pub fn open_file(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path).map_err(AppError::db_error)?;
        crate::db::schema::initialize_tables(&conn)?;
        Ok(Self::new(conn))
    }

    // --- Repositories ---
    pub fn get_repository(&self) -> Result<Option<Repository>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, source_type, git_remote, current_branch, last_scanned_at FROM repositories LIMIT 1"
        ).map_err(AppError::db_error)?;

        let repo = stmt
            .query_row([], |row| {
                Ok(Repository {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    source_type: row.get(3)?,
                    git_remote: row.get(4)?,
                    current_branch: row.get(5)?,
                    last_scanned_at: row.get(6)?,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(repo)
    }

    pub fn save_repository(&self, repo: &Repository) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        // V1 only supports 1 repository; delete existing repository if different ID
        conn.execute("DELETE FROM repositories WHERE id != ?1", params![repo.id])
            .map_err(AppError::db_error)?;
        conn.execute(
            "INSERT INTO repositories (id, name, path, source_type, git_remote, current_branch, last_scanned_at, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                source_type = excluded.source_type,
                git_remote = excluded.git_remote,
                current_branch = excluded.current_branch,
                last_scanned_at = excluded.last_scanned_at",
            params![
                repo.id,
                repo.name,
                repo.path,
                repo.source_type,
                repo.git_remote,
                repo.current_branch,
                repo.last_scanned_at,
                Utc::now().to_rfc3339()
            ],
        ).map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn delete_repository(&self, id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM repositories WHERE id = ?1", params![id])
            .map_err(AppError::db_error)?;
        Ok(())
    }

    // --- Skills ---
    pub fn upsert_skills(&self, skills: &[Skill]) -> Result<(), AppError> {
        let mut conn = self.conn.lock().unwrap();
        let tx = conn.transaction().map_err(AppError::db_error)?;
        for s in skills {
            tx.execute(
                "INSERT INTO skills (id, repository_id, name, relative_path, canonical_path, description, metadata_status, content_hash, file_count, char_count, last_seen_at, source, remote_skill_id, remote_hash, has_update, last_checked_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                 ON CONFLICT(repository_id, name) DO UPDATE SET
                    relative_path = excluded.relative_path,
                    canonical_path = excluded.canonical_path,
                    description = excluded.description,
                    metadata_status = excluded.metadata_status,
                    content_hash = excluded.content_hash,
                    file_count = excluded.file_count,
                    char_count = excluded.char_count,
                    last_seen_at = excluded.last_seen_at,
                    source = COALESCE(excluded.source, skills.source),
                    remote_skill_id = COALESCE(excluded.remote_skill_id, skills.remote_skill_id),
                    remote_hash = COALESCE(excluded.remote_hash, skills.remote_hash),
                    has_update = CASE
                        WHEN skills.remote_hash IS NOT NULL AND skills.remote_hash != '' AND excluded.content_hash = skills.remote_hash THEN 0
                        WHEN excluded.has_update IS NOT NULL THEN excluded.has_update
                        ELSE COALESCE(skills.has_update, 0)
                    END,
                    last_checked_at = COALESCE(excluded.last_checked_at, skills.last_checked_at),
                    updated_at = COALESCE(excluded.updated_at, skills.updated_at)",
                params![
                    s.id,
                    s.repository_id,
                    s.name,
                    s.relative_path,
                    s.canonical_path,
                    s.description,
                    s.metadata_status,
                    s.content_hash,
                    s.file_count,
                    s.char_count,
                    s.last_seen_at,
                    s.source,
                    s.remote_skill_id,
                    s.remote_hash,
                    s.has_update.map(|b| if b { 1 } else { 0 }),
                    s.last_checked_at,
                    s.updated_at,
                ],
            ).map_err(AppError::db_error)?;
        }
        tx.commit().map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn get_skills(&self, repository_id: &str) -> Result<Vec<Skill>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, repository_id, name, relative_path, canonical_path, description, metadata_status, content_hash, file_count, char_count, last_seen_at, source, remote_skill_id, remote_hash, has_update, last_checked_at, updated_at
             FROM skills WHERE repository_id = ?1 ORDER BY name ASC"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map(params![repository_id], |row| {
                let has_up_int: i32 = row.get(14).unwrap_or(0);
                Ok(Skill {
                    id: row.get(0)?,
                    repository_id: row.get(1)?,
                    name: row.get(2)?,
                    relative_path: row.get(3)?,
                    canonical_path: row.get(4)?,
                    description: row.get(5)?,
                    metadata_status: row.get(6)?,
                    content_hash: row.get(7)?,
                    file_count: row.get(8)?,
                    char_count: row.get(9)?,
                    last_seen_at: row.get(10)?,
                    source: row.get(11)?,
                    remote_skill_id: row.get(12)?,
                    remote_hash: row.get(13)?,
                    has_update: Some(has_up_int != 0),
                    last_checked_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn get_skill_by_name(
        &self,
        repository_id: &str,
        name: &str,
    ) -> Result<Option<Skill>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, repository_id, name, relative_path, canonical_path, description, metadata_status, content_hash, file_count, char_count, last_seen_at, source, remote_skill_id, remote_hash, has_update, last_checked_at, updated_at
             FROM skills WHERE repository_id = ?1 AND name = ?2 COLLATE NOCASE"
        ).map_err(AppError::db_error)?;

        let skill = stmt
            .query_row(params![repository_id, name], |row| {
                let has_up_int: i32 = row.get(14).unwrap_or(0);
                Ok(Skill {
                    id: row.get(0)?,
                    repository_id: row.get(1)?,
                    name: row.get(2)?,
                    relative_path: row.get(3)?,
                    canonical_path: row.get(4)?,
                    description: row.get(5)?,
                    metadata_status: row.get(6)?,
                    content_hash: row.get(7)?,
                    file_count: row.get(8)?,
                    char_count: row.get(9)?,
                    last_seen_at: row.get(10)?,
                    source: row.get(11)?,
                    remote_skill_id: row.get(12)?,
                    remote_hash: row.get(13)?,
                    has_update: Some(has_up_int != 0),
                    last_checked_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(skill)
    }

    pub fn get_skill_by_id(&self, skill_id: &str) -> Result<Option<Skill>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, repository_id, name, relative_path, canonical_path, description, metadata_status, content_hash, file_count, char_count, last_seen_at, source, remote_skill_id, remote_hash, has_update, last_checked_at, updated_at
             FROM skills WHERE id = ?1"
        ).map_err(AppError::db_error)?;

        let skill = stmt
            .query_row(params![skill_id], |row| {
                let has_up_int: i32 = row.get(14).unwrap_or(0);
                Ok(Skill {
                    id: row.get(0)?,
                    repository_id: row.get(1)?,
                    name: row.get(2)?,
                    relative_path: row.get(3)?,
                    canonical_path: row.get(4)?,
                    description: row.get(5)?,
                    metadata_status: row.get(6)?,
                    content_hash: row.get(7)?,
                    file_count: row.get(8)?,
                    char_count: row.get(9)?,
                    last_seen_at: row.get(10)?,
                    source: row.get(11)?,
                    remote_skill_id: row.get(12)?,
                    remote_hash: row.get(13)?,
                    has_update: Some(has_up_int != 0),
                    last_checked_at: row.get(15)?,
                    updated_at: row.get(16)?,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(skill)
    }

    pub fn update_skill_update_status(
        &self,
        repository_id: &str,
        skill_name: &str,
        remote_hash: &str,
        has_update: bool,
        last_checked_at: &str,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE skills SET
                remote_hash = ?1,
                has_update = ?2,
                last_checked_at = ?3
             WHERE repository_id = ?4 AND name = ?5 COLLATE NOCASE",
            params![
                remote_hash,
                has_update as i32,
                last_checked_at,
                repository_id,
                skill_name,
            ],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn update_skill_after_update(
        &self,
        repository_id: &str,
        skill_name: &str,
        new_hash: &str,
        updated_at: &str,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE skills SET
                content_hash = ?1,
                remote_hash = ?1,
                has_update = 0,
                updated_at = ?2,
                last_checked_at = ?2
             WHERE repository_id = ?3 AND name = ?4 COLLATE NOCASE",
            params![new_hash, updated_at, repository_id, skill_name,],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn set_skill_source(
        &self,
        repository_id: &str,
        skill_name: &str,
        source: &str,
        remote_skill_id: &str,
    ) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE skills SET
                source = ?1,
                remote_skill_id = ?2
             WHERE repository_id = ?3 AND name = ?4 COLLATE NOCASE",
            params![source, remote_skill_id, repository_id, skill_name,],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn get_skill_source_from_logs(
        &self,
        skill_name: &str,
    ) -> Result<Option<(String, String)>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT message FROM operation_logs
             WHERE operation_type IN ('market_install', 'skill_update')
               AND (skill_name = ?1 COLLATE NOCASE OR message LIKE ?2)
             ORDER BY created_at DESC LIMIT 1",
            )
            .map_err(AppError::db_error)?;

        let pattern = format!("%{}%", skill_name);
        let msg: Option<String> = stmt
            .query_row(params![skill_name, pattern], |r| r.get(0))
            .optional()
            .map_err(AppError::db_error)?;

        if let Some(text) = msg {
            // 解析格式: "从市场成功安装 Skill: xxx (源: owner/repo)"
            if let Some((_, after_src)) = text.split_once("(源: ") {
                if let Some((src, _)) = after_src.split_once(')') {
                    let src = src.trim().to_string();
                    return Ok(Some((src, skill_name.to_string())));
                }
            }
        }
        Ok(None)
    }

    pub fn delete_skill_by_name(&self, repository_id: &str, name: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "DELETE FROM skills WHERE repository_id = ?1 AND name = ?2 COLLATE NOCASE",
            params![repository_id, name],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    // --- Projects ---
    pub fn get_projects(&self) -> Result<Vec<Project>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, name, path, last_scanned_at, is_archived FROM projects ORDER BY name ASC"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map([], |row| {
                let archived_int: i32 = row.get(4)?;
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    last_scanned_at: row.get(3)?,
                    is_archived: archived_int != 0,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn get_project_by_id(&self, id: &str) -> Result<Option<Project>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, path, last_scanned_at, is_archived FROM projects WHERE id = ?1",
            )
            .map_err(AppError::db_error)?;

        let project = stmt
            .query_row(params![id], |row| {
                let archived_int: i32 = row.get(4)?;
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    last_scanned_at: row.get(3)?,
                    is_archived: archived_int != 0,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(project)
    }

    pub fn get_project_by_path(&self, path: &str) -> Result<Option<Project>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT id, name, path, last_scanned_at, is_archived FROM projects WHERE path = ?1",
            )
            .map_err(AppError::db_error)?;

        let project = stmt
            .query_row(params![path], |row| {
                let archived_int: i32 = row.get(4)?;
                Ok(Project {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    path: row.get(2)?,
                    last_scanned_at: row.get(3)?,
                    is_archived: archived_int != 0,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(project)
    }

    pub fn save_project(&self, project: &Project) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, path, last_scanned_at, is_archived, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                path = excluded.path,
                last_scanned_at = excluded.last_scanned_at,
                is_archived = excluded.is_archived",
            params![
                project.id,
                project.name,
                project.path,
                project.last_scanned_at,
                if project.is_archived { 1 } else { 0 },
                Utc::now().to_rfc3339()
            ],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn update_project_path(&self, project_id: &str, new_path: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE projects SET path = ?1, last_scanned_at = ?2 WHERE id = ?3",
            params![new_path, Utc::now().to_rfc3339(), project_id],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn delete_project(&self, id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM projects WHERE id = ?1", params![id])
            .map_err(AppError::db_error)?;
        Ok(())
    }

    // --- Agent Targets & Entry Links ---
    pub fn get_agent_targets(&self, project_id: &str) -> Result<Vec<AgentTarget>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, agent_type, install_dir, link_mode FROM agent_targets WHERE project_id = ?1"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                Ok(AgentTarget {
                    id: row.get(0)?,
                    project_id: row.get(1)?,
                    agent_type: row.get(2)?,
                    install_dir: row.get(3)?,
                    link_mode: row.get(4)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn upsert_agent_target(&self, target: &AgentTarget) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO agent_targets (id, project_id, agent_type, install_dir, link_mode)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(project_id, agent_type) DO UPDATE SET
                install_dir = excluded.install_dir,
                link_mode = excluded.link_mode",
            params![
                target.id,
                target.project_id,
                target.agent_type,
                target.install_dir,
                target.link_mode
            ],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn get_entry_links(&self, agent_target_id: &str) -> Result<Vec<EntryLink>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, agent_target_id, link_path, target_path, status FROM entry_links WHERE agent_target_id = ?1"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map(params![agent_target_id], |row| {
                Ok(EntryLink {
                    id: row.get(0)?,
                    agent_target_id: row.get(1)?,
                    link_path: row.get(2)?,
                    target_path: row.get(3)?,
                    status: row.get(4)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn upsert_entry_link(&self, link: &EntryLink) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO entry_links (id, agent_target_id, link_path, target_path, status)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(agent_target_id, link_path) DO UPDATE SET
                target_path = excluded.target_path,
                status = excluded.status",
            params![
                link.id,
                link.agent_target_id,
                link.link_path,
                link.target_path,
                link.status
            ],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn delete_entry_link(&self, id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM entry_links WHERE id = ?1", params![id])
            .map_err(AppError::db_error)?;
        Ok(())
    }

    // --- Mounts ---
    pub fn get_mounts_by_project(&self, project_id: &str) -> Result<Vec<Mount>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT m.id, m.skill_id, m.skill_name, m.project_id, m.link_path, m.resolved_target, m.mount_mode, m.status, m.managed_by_app, m.backup_path, m.content_hash, m.created_at, s.content_hash as current_skill_hash
             FROM mounts m
             LEFT JOIN skills s ON m.skill_id = s.id
             WHERE m.project_id = ?1 ORDER BY m.skill_name ASC"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map(params![project_id], |row| {
                let managed_int: i32 = row.get(8)?;
                let mount_hash: Option<String> = row.get(10)?;
                let current_hash: Option<String> = row.get(12)?;
                let is_outdated = match (&mount_hash, &current_hash) {
                    (Some(mh), Some(ch)) => !mh.is_empty() && !ch.is_empty() && mh != ch,
                    _ => false,
                };

                Ok(Mount {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    project_id: row.get(3)?,
                    link_path: row.get(4)?,
                    resolved_target: row.get(5)?,
                    mount_mode: row.get(6)?,
                    status: row.get(7)?,
                    managed_by_app: managed_int != 0,
                    backup_path: row.get(9)?,
                    content_hash: mount_hash,
                    is_outdated: Some(is_outdated),
                    created_at: row.get(11)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn get_all_mounts(&self) -> Result<Vec<Mount>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, skill_id, skill_name, project_id, link_path, resolved_target, mount_mode, status, managed_by_app, backup_path, content_hash, created_at
             FROM mounts ORDER BY created_at DESC"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map([], |row| {
                let managed_int: i32 = row.get(8)?;
                Ok(Mount {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    project_id: row.get(3)?,
                    link_path: row.get(4)?,
                    resolved_target: row.get(5)?,
                    mount_mode: row.get(6)?,
                    status: row.get(7)?,
                    managed_by_app: managed_int != 0,
                    backup_path: row.get(9)?,
                    content_hash: row.get(10)?,
                    is_outdated: None,
                    created_at: row.get(11)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn get_mounts_by_skill_name(&self, skill_name: &str) -> Result<Vec<Mount>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, skill_id, skill_name, project_id, link_path, resolved_target, mount_mode, status, managed_by_app, backup_path, content_hash, created_at
             FROM mounts WHERE skill_name = ?1"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map(params![skill_name], |row| {
                let managed_int: i32 = row.get(8)?;
                Ok(Mount {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    project_id: row.get(3)?,
                    link_path: row.get(4)?,
                    resolved_target: row.get(5)?,
                    mount_mode: row.get(6)?,
                    status: row.get(7)?,
                    managed_by_app: managed_int != 0,
                    backup_path: row.get(9)?,
                    content_hash: row.get(10)?,
                    is_outdated: Some(false),
                    created_at: row.get(11)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    pub fn get_mount_by_id(&self, mount_id: &str) -> Result<Option<Mount>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, skill_id, skill_name, project_id, link_path, resolved_target, mount_mode, status, managed_by_app, backup_path, content_hash, created_at
             FROM mounts WHERE id = ?1"
        ).map_err(AppError::db_error)?;

        let mount = stmt
            .query_row(params![mount_id], |row| {
                let managed_int: i32 = row.get(8)?;
                Ok(Mount {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    project_id: row.get(3)?,
                    link_path: row.get(4)?,
                    resolved_target: row.get(5)?,
                    mount_mode: row.get(6)?,
                    status: row.get(7)?,
                    managed_by_app: managed_int != 0,
                    backup_path: row.get(9)?,
                    content_hash: row.get(10)?,
                    is_outdated: None,
                    created_at: row.get(11)?,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(mount)
    }

    pub fn get_mount_by_project_and_skill(
        &self,
        project_id: &str,
        skill_name: &str,
    ) -> Result<Option<Mount>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, skill_id, skill_name, project_id, link_path, resolved_target, mount_mode, status, managed_by_app, backup_path, content_hash, created_at
             FROM mounts WHERE project_id = ?1 AND skill_name = ?2"
        ).map_err(AppError::db_error)?;

        let mount = stmt
            .query_row(params![project_id, skill_name], |row| {
                let managed_int: i32 = row.get(8)?;
                Ok(Mount {
                    id: row.get(0)?,
                    skill_id: row.get(1)?,
                    skill_name: row.get(2)?,
                    project_id: row.get(3)?,
                    link_path: row.get(4)?,
                    resolved_target: row.get(5)?,
                    mount_mode: row.get(6)?,
                    status: row.get(7)?,
                    managed_by_app: managed_int != 0,
                    backup_path: row.get(9)?,
                    content_hash: row.get(10)?,
                    is_outdated: None,
                    created_at: row.get(11)?,
                })
            })
            .optional()
            .map_err(AppError::db_error)?;

        Ok(mount)
    }

    pub fn upsert_mount(&self, mount: &Mount) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO mounts (id, skill_id, skill_name, project_id, link_path, resolved_target, mount_mode, status, managed_by_app, backup_path, content_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(project_id, skill_name) DO UPDATE SET
                skill_id = excluded.skill_id,
                link_path = excluded.link_path,
                resolved_target = excluded.resolved_target,
                mount_mode = excluded.mount_mode,
                status = excluded.status,
                managed_by_app = excluded.managed_by_app,
                backup_path = excluded.backup_path,
                content_hash = excluded.content_hash",
            params![
                mount.id,
                mount.skill_id,
                mount.skill_name,
                mount.project_id,
                mount.link_path,
                mount.resolved_target,
                mount.mount_mode,
                mount.status,
                if mount.managed_by_app { 1 } else { 0 },
                mount.backup_path,
                mount.content_hash,
                mount.created_at
            ],
        ).map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn update_mount_status(&self, mount_id: &str, status: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE mounts SET status = ?1 WHERE id = ?2",
            params![status, mount_id],
        )
        .map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn delete_mount(&self, mount_id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM mounts WHERE id = ?1", params![mount_id])
            .map_err(AppError::db_error)?;
        Ok(())
    }

    // --- Operation Logs ---
    pub fn insert_log(&self, log: &OperationLog) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO operation_logs (id, batch_id, operation_type, entity_type, entity_id, project_id, skill_name, target_path, backup_path, status, error_code, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                log.id,
                log.batch_id,
                log.operation_type,
                log.entity_type,
                log.entity_id,
                log.project_id,
                log.skill_name,
                log.target_path,
                log.backup_path,
                log.status,
                log.error_code,
                log.message,
                log.created_at
            ],
        ).map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn query_logs(
        &self,
        project_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<OperationLog>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut list = Vec::new();
        if let Some(pid) = project_id {
            let mut stmt = conn.prepare(
                "SELECT id, batch_id, operation_type, entity_type, entity_id, project_id, skill_name, target_path, backup_path, status, error_code, message, created_at
                 FROM operation_logs WHERE project_id = ?1 ORDER BY created_at DESC LIMIT ?2"
            ).map_err(AppError::db_error)?;
            let rows = stmt
                .query_map(params![pid, limit as i64], |row| {
                    Ok(OperationLog {
                        id: row.get(0)?,
                        batch_id: row.get(1)?,
                        operation_type: row.get(2)?,
                        entity_type: row.get(3)?,
                        entity_id: row.get(4)?,
                        project_id: row.get(5)?,
                        skill_name: row.get(6)?,
                        target_path: row.get(7)?,
                        backup_path: row.get(8)?,
                        status: row.get(9)?,
                        error_code: row.get(10)?,
                        message: row.get(11)?,
                        created_at: row.get(12)?,
                    })
                })
                .map_err(AppError::db_error)?;
            for r in rows {
                list.push(r.map_err(AppError::db_error)?);
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT id, batch_id, operation_type, entity_type, entity_id, project_id, skill_name, target_path, backup_path, status, error_code, message, created_at
                 FROM operation_logs ORDER BY created_at DESC LIMIT ?1"
            ).map_err(AppError::db_error)?;
            let rows = stmt
                .query_map(params![limit as i64], |row| {
                    Ok(OperationLog {
                        id: row.get(0)?,
                        batch_id: row.get(1)?,
                        operation_type: row.get(2)?,
                        entity_type: row.get(3)?,
                        entity_id: row.get(4)?,
                        project_id: row.get(5)?,
                        skill_name: row.get(6)?,
                        target_path: row.get(7)?,
                        backup_path: row.get(8)?,
                        status: row.get(9)?,
                        error_code: row.get(10)?,
                        message: row.get(11)?,
                        created_at: row.get(12)?,
                    })
                })
                .map_err(AppError::db_error)?;
            for r in rows {
                list.push(r.map_err(AppError::db_error)?);
            }
        }
        Ok(list)
    }

    pub fn get_logs_by_batch(&self, batch_id: &str) -> Result<Vec<OperationLog>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, batch_id, operation_type, entity_type, entity_id, project_id, skill_name, target_path, backup_path, status, error_code, message, created_at
             FROM operation_logs WHERE batch_id = ?1 ORDER BY created_at ASC"
        ).map_err(AppError::db_error)?;

        let rows = stmt
            .query_map(params![batch_id], |row| {
                Ok(OperationLog {
                    id: row.get(0)?,
                    batch_id: row.get(1)?,
                    operation_type: row.get(2)?,
                    entity_type: row.get(3)?,
                    entity_id: row.get(4)?,
                    project_id: row.get(5)?,
                    skill_name: row.get(6)?,
                    target_path: row.get(7)?,
                    backup_path: row.get(8)?,
                    status: row.get(9)?,
                    error_code: row.get(10)?,
                    message: row.get(11)?,
                    created_at: row.get(12)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }

    // --- App Settings ---
    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT value FROM app_settings WHERE key = ?1")
            .map_err(AppError::db_error)?;
        let val = stmt
            .query_row(params![key], |row| row.get(0))
            .optional()
            .map_err(AppError::db_error)?;
        Ok(val)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO app_settings (key, value, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
            params![key, value, Utc::now().to_rfc3339()],
        ).map_err(AppError::db_error)?;
        Ok(())
    }

    pub fn get_all_settings(&self) -> Result<Vec<AppSetting>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT key, value, updated_at FROM app_settings")
            .map_err(AppError::db_error)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(AppSetting {
                    key: row.get(0)?,
                    value: row.get(1)?,
                    updated_at: row.get(2)?,
                })
            })
            .map_err(AppError::db_error)?;

        let mut list = Vec::new();
        for r in rows {
            list.push(r.map_err(AppError::db_error)?);
        }
        Ok(list)
    }
}
