export interface Repository {
  id: string;
  name: string;
  path: string;
  source_type: "git" | "local";
  git_remote?: string | null;
  current_branch?: string | null;
  last_scanned_at?: string | null;
}

export interface Skill {
  id: string;
  repository_id: string;
  name: string;
  relative_path: string;
  canonical_path: string;
  description: string;
  metadata_status: "valid" | "incomplete";
  content_hash: string;
  file_count: number;
  char_count: number;
  last_seen_at: string;
  source?: string | null;
  remote_skill_id?: string | null;
  remote_hash?: string | null;
  has_update?: boolean | null;
  last_checked_at?: string | null;
  updated_at?: string | null;
}

export interface Project {
  id: string;
  name: string;
  path: string;
  last_scanned_at?: string | null;
  is_archived: boolean;
}

export interface AgentTarget {
  id: string;
  project_id: string;
  agent_type: string;
  install_dir: string;
  link_mode: string;
}

export interface EntryLink {
  id: string;
  agent_target_id: string;
  link_path: string;
  target_path: string;
  status: "valid" | "broken" | "conflict" | "missing";
}

export type MountStatus =
  | "NORMAL"
  | "BROKEN"
  | "WRONG_TARGET"
  | "CONFLICT"
  | "UNMANAGED"
  | "PERMISSION_DENIED"
  | "UNCHECKED";

export interface Mount {
  id: string;
  skill_id: string;
  skill_name: string;
  project_id: string;
  link_path: string;
  resolved_target: string;
  mount_mode: "symlink" | "junction" | "copy";
  status: MountStatus;
  managed_by_app: boolean;
  backup_path?: string | null;
  content_hash?: string | null;
  is_outdated?: boolean | null;
  created_at: string;
}

export interface OperationLog {
  id: string;
  batch_id?: string | null;
  operation_type: string;
  entity_type: string;
  entity_id?: string | null;
  project_id?: string | null;
  skill_name?: string | null;
  target_path?: string | null;
  backup_path?: string | null;
  status: "SUCCESS" | "FAILED" | "ROLLED_BACK";
  error_code?: string | null;
  message: string;
  created_at: string;
}

export interface AppSetting {
  key: string;
  value: string;
  updated_at: string;
}

export type ConflictStrategy = "backup_and_replace" | "skip" | "cancel";

export interface MountResult {
  skill_id: string;
  skill_name: string;
  success: boolean;
  status: string;
  mount_mode: string;
  error_code?: string | null;
  error_message?: string | null;
  backup_path?: string | null;
  batch_id?: string | null;
}

export interface UnmountResult {
  mount_id: string;
  skill_name: string;
  success: boolean;
  error_code?: string | null;
  error_message?: string | null;
}

export interface GitStatusResult {
  branch: string;
  commit_hash: string;
  commit_message: string;
  is_dirty: boolean;
  changed_files: string[];
  error?: string | null;
}

export interface AffectedSkill {
  name: string;
  change_type: "deleted" | "renamed" | "modified";
  affected_project_ids: string[];
}

export interface GitPullResult {
  success: boolean;
  updated: boolean;
  old_commit: string;
  new_commit: string;
  affected_skills: AffectedSkill[];
  error_message?: string | null;
}

export interface ScanResult {
  skills: Skill[];
  warnings: string[];
}

export interface ProjectDiagnostic {
  project_id: string;
  mounts: Mount[];
  unmanaged_dirs: string[];
  entry_links: EntryLink[];
  is_valid: boolean;
}

export interface ImportProjectPreview {
  original_path: string;
  remapped_path: string;
  exists: boolean;
  skill_names: string[];
}

export interface ImportPreviewResult {
  repository: Repository | null;
  repository_exists: boolean;
  projects: ImportProjectPreview[];
  settings_count: number;
}

export interface LogFileInfo {
  path: string;
  dir: string;
  size_bytes: number;
  exists: boolean;
}

export interface MarketSkillItem {
  id: string;
  skillId: string;
  name: string;
  installs: number;
  source: string;
  is_installed: boolean;
}

export interface MarketSearchResponse {
  skills: MarketSkillItem[];
  count: number;
}

export interface MarketInstallRequest {
  source: string;
  skill_id: string;
  skill_name: string;
  force_overwrite?: boolean;
}

export interface MarketInstallResult {
  success: boolean;
  skill_name: string;
  message: string;
}

export interface MarketUninstallCheckResult {
  skill_name: string;
  mounted_projects: string[];
  can_direct_delete: boolean;
}

export interface MarketUninstallRequest {
  skill_name: string;
  cascade_unmount: boolean;
}

export interface MarketUninstallResult {
  success: boolean;
  skill_name: string;
  unmounted_count: number;
  message: string;
}

export interface MarketSkillDetail {
  skill_id: string;
  name: string;
  source: string;
  installs: number;
  is_installed: boolean;
  local_path?: string | null;
  content?: string | null;
}

export interface SkillUpdateInfo {
  skill_name: string;
  source: string;
  skill_id: string;
  current_hash: string;
  remote_hash: string;
  has_update: boolean;
  last_checked_at: string;
}

export interface SkillUpdateResult {
  success: boolean;
  skill_name: string;
  old_hash: string;
  new_hash: string;
  backup_path?: string | null;
  message: string;
}

export interface BatchUpdateResult {
  total: number;
  success_count: number;
  failed_count: number;
  results: SkillUpdateResult[];
}

export interface SkillBackupItem {
  id: string;
  skill_name: string;
  source?: string | null;
  created_at: string;
  backup_path: string;
  content_hash: string;
  reason?: string | null;
}

