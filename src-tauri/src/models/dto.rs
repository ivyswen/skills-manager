use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: String,
    pub name: String,
    pub path: String,
    pub source_type: String, // "git" | "local"
    pub git_remote: Option<String>,
    pub current_branch: Option<String>,
    pub last_scanned_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub id: String,
    pub repository_id: String,
    pub name: String,
    pub relative_path: String,
    pub canonical_path: String,
    pub description: String,
    pub metadata_status: String, // "valid" | "incomplete"
    pub content_hash: String,
    pub file_count: u32,
    pub char_count: u64,
    pub last_seen_at: String,
    pub source: Option<String>,
    pub remote_skill_id: Option<String>,
    pub remote_hash: Option<String>,
    pub has_update: Option<bool>,
    pub last_checked_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_scanned_at: Option<String>,
    pub is_archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTarget {
    pub id: String,
    pub project_id: String,
    pub agent_type: String,  // "claude_code" | "codex" | "custom"
    pub install_dir: String, // fixed to ".agents/skills"
    pub link_mode: String,   // "symlink" | "junction" | "copy"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryLink {
    pub id: String,
    pub agent_target_id: String,
    pub link_path: String,   // e.g. ".claude/skills"
    pub target_path: String, // e.g. ".agents/skills"
    pub status: String,      // "valid" | "broken" | "conflict" | "missing"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MountStatus {
    Normal,
    Broken,
    WrongTarget,
    Conflict,
    Unmanaged,
    PermissionDenied,
    Unchecked,
}

impl MountStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MountStatus::Normal => "NORMAL",
            MountStatus::Broken => "BROKEN",
            MountStatus::WrongTarget => "WRONG_TARGET",
            MountStatus::Conflict => "CONFLICT",
            MountStatus::Unmanaged => "UNMANAGED",
            MountStatus::PermissionDenied => "PERMISSION_DENIED",
            MountStatus::Unchecked => "UNCHECKED",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "NORMAL" => MountStatus::Normal,
            "BROKEN" => MountStatus::Broken,
            "WRONG_TARGET" => MountStatus::WrongTarget,
            "CONFLICT" => MountStatus::Conflict,
            "UNMANAGED" => MountStatus::Unmanaged,
            "PERMISSION_DENIED" => MountStatus::PermissionDenied,
            _ => MountStatus::Unchecked,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub id: String,
    pub skill_id: String,
    pub skill_name: String,
    pub project_id: String,
    pub link_path: String,
    pub resolved_target: String,
    pub mount_mode: String, // "symlink" | "junction" | "copy"
    pub status: String,
    pub managed_by_app: bool,
    pub backup_path: Option<String>,
    pub content_hash: Option<String>,
    pub is_outdated: Option<bool>,
    #[serde(default)]
    pub has_local_changes: Option<bool>,
    #[serde(default)]
    pub has_conflict: Option<bool>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationLog {
    pub id: String,
    pub batch_id: Option<String>,
    pub operation_type: String, // "mount" | "unmount" | "repair" | "git_pull" | "entry_link" | "backup" | "rollback"
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub project_id: Option<String>,
    pub skill_name: Option<String>,
    pub target_path: Option<String>,
    pub backup_path: Option<String>,
    pub status: String, // "SUCCESS" | "FAILED" | "ROLLED_BACK"
    pub error_code: Option<String>,
    pub message: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSetting {
    pub key: String,
    pub value: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictStrategy {
    #[serde(rename = "backup_and_replace")]
    BackupAndReplace,
    #[serde(rename = "skip")]
    Skip,
    #[serde(rename = "cancel")]
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountRequest {
    pub skill_ids: Vec<String>,
    pub project_id: String,
    pub conflict_strategy: ConflictStrategy,
    pub preferred_mode: Option<String>, // "symlink" | "junction" | "copy"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountResult {
    pub skill_id: String,
    pub skill_name: String,
    pub success: bool,
    pub status: String,
    pub mount_mode: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub backup_path: Option<String>,
    pub batch_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmountResult {
    pub mount_id: String,
    pub skill_name: String,
    pub success: bool,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusResult {
    pub branch: String,
    pub commit_hash: String,
    pub commit_message: String,
    pub is_dirty: bool,
    pub changed_files: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedSkill {
    pub name: String,
    pub change_type: String, // "deleted" | "renamed" | "modified"
    pub affected_project_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitPullResult {
    pub success: bool,
    pub updated: bool,
    pub old_commit: String,
    pub new_commit: String,
    pub affected_skills: Vec<AffectedSkill>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub skills: Vec<Skill>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDiagnostic {
    pub project_id: String,
    pub mounts: Vec<Mount>,
    pub unmanaged_dirs: Vec<String>,
    pub entry_links: Vec<EntryLink>,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfigData {
    pub version: String,
    pub exported_at: String,
    pub repository: Option<Repository>,
    pub skills: Vec<Skill>,
    pub projects: Vec<Project>,
    pub agent_targets: Vec<AgentTarget>,
    pub entry_links: Vec<EntryLink>,
    pub mounts: Vec<Mount>,
    pub settings: Vec<AppSetting>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportProjectPreview {
    pub original_path: String,
    pub remapped_path: String,
    pub exists: bool,
    pub skill_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportPreviewResult {
    pub repository: Option<Repository>,
    pub repository_exists: bool,
    pub projects: Vec<ImportProjectPreview>,
    pub settings_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSkillItem {
    pub id: String,
    #[serde(rename = "skillId")]
    pub skill_id: String,
    pub name: String,
    pub installs: u64,
    pub source: String,
    pub is_installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSearchResponse {
    pub skills: Vec<MarketSkillItem>,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInstallRequest {
    pub source: String,
    pub skill_id: String,
    pub skill_name: String,
    pub force_overwrite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInstallResult {
    pub success: bool,
    pub skill_name: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUninstallCheckResult {
    pub skill_name: String,
    pub mounted_projects: Vec<String>,
    pub can_direct_delete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUninstallRequest {
    pub skill_name: String,
    pub cascade_unmount: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUninstallResult {
    pub success: bool,
    pub skill_name: String,
    pub unmounted_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSkillDetail {
    pub skill_id: String,
    pub name: String,
    pub source: String,
    pub installs: u64,
    pub is_installed: bool,
    pub local_path: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateInfo {
    pub skill_name: String,
    pub source: String,
    pub skill_id: String,
    pub current_hash: String,
    pub remote_hash: String,
    pub has_update: bool,
    pub last_checked_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillUpdateResult {
    pub success: bool,
    pub skill_name: String,
    pub old_hash: String,
    pub new_hash: String,
    pub backup_path: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchUpdateResult {
    pub total: usize,
    pub success_count: usize,
    pub failed_count: usize,
    pub results: Vec<SkillUpdateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillBackupItem {
    pub id: String,
    pub skill_name: String,
    pub source: Option<String>,
    pub created_at: String,
    pub backup_path: String,
    pub content_hash: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetaFile {
    pub name: String,
    pub source: String,
    pub skill_id: String,
    pub installed_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDiffItem {
    pub path: String,
    pub change_type: String, // "ADDED" | "MODIFIED" | "DELETED"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDiffResult {
    pub skill_name: String,
    pub has_conflict: bool,
    pub files: Vec<SkillDiffItem>,
    pub local_hash: String,
    pub central_hash: String,
    pub base_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReversePushRequest {
    pub project_id: String,
    pub skill_name: String,
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReversePushResult {
    pub success: bool,
    pub skill_name: String,
    pub backup_id: Option<String>,
    pub message: String,
}
