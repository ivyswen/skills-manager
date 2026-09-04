import { invoke } from "@tauri-apps/api/core";
import { openPath as openerOpenPath } from "@tauri-apps/plugin-opener";
import {
  Repository,
  Project,
  OperationLog,
  MountResult,
  UnmountResult,
  GitStatusResult,
  GitPullResult,
  ScanResult,
  ProjectDiagnostic,
  ImportPreviewResult,
  EntryLink,
  LogFileInfo,
} from "../types";

// 统一封装 Tauri Invoke
export const api = {
  // --- Repository ---
  async getRepository(): Promise<Repository | null> {
    return await invoke<Repository | null>("repository_get");
  },

  async addRepository(path: string, name?: string): Promise<Repository> {
    return await invoke<Repository>("repository_add", { path, name });
  },

  async scanRepository(): Promise<ScanResult> {
    return await invoke<ScanResult>("repository_scan");
  },

  async getGitStatus(): Promise<GitStatusResult> {
    return await invoke<GitStatusResult>("repository_git_status");
  },

  async gitPull(): Promise<GitPullResult> {
    return await invoke<GitPullResult>("repository_git_pull");
  },

  // --- Project ---
  async listProjects(): Promise<Project[]> {
    return await invoke<Project[]>("project_list");
  },

  async addProject(path: string, name?: string): Promise<Project> {
    return await invoke<Project>("project_add", { path, name });
  },

  async relocateProject(projectId: string, newPath: string): Promise<void> {
    return await invoke<void>("project_relocate", { projectId, newPath });
  },

  async removeProject(projectId: string, cleanupLinks: boolean): Promise<void> {
    return await invoke<void>("project_remove", { projectId, cleanupLinks });
  },

  async diagnoseProject(projectId: string): Promise<ProjectDiagnostic> {
    return await invoke<ProjectDiagnostic>("project_diagnose", { projectId });
  },

  async openFolder(path: string): Promise<void> {
    try {
      await invoke<void>("open_path_in_explorer", { path });
    } catch (e) {
      await openerOpenPath(path);
    }
  },

  // --- Mount ---
  async mountSkills(
    skillIds: string[],
    projectId: string,
    conflictStrategy: "backup_and_replace" | "skip" | "cancel" = "backup_and_replace",
    preferredMode?: "symlink" | "junction" | "copy"
  ): Promise<MountResult[]> {
    return await invoke<MountResult[]>("mount_skills", {
      request: {
        skill_ids: skillIds,
        project_id: projectId,
        conflict_strategy: conflictStrategy,
        preferred_mode: preferredMode,
      },
    });
  },

  async unmountSkills(mountIds: string[], force: boolean = false): Promise<UnmountResult[]> {
    return await invoke<UnmountResult[]>("unmount_skills", { mountIds, force });
  },

  async repairMount(mountId: string, preferredMode?: string): Promise<void> {
    return await invoke<void>("mount_repair", { mountId, preferredMode });
  },

  async rollbackBatch(batchId: string): Promise<void> {
    return await invoke<void>("mount_rollback_batch", { batchId });
  },

  // --- Entry Link ---
  async setupEntryLink(
    projectId: string,
    agentType: string,
    linkPath: string,
    preferredMode?: string
  ): Promise<EntryLink> {
    return await invoke<EntryLink>("entry_link_setup", {
      projectId,
      agentType,
      linkPath,
      preferredMode,
    });
  },

  async removeEntryLink(entryLinkId: string, projectId: string, linkPath: string): Promise<void> {
    return await invoke<void>("entry_link_remove", { entryLinkId, projectId, linkPath });
  },

  // --- Logs ---
  async queryLogs(projectId?: string, limit: number = 100): Promise<OperationLog[]> {
    return await invoke<OperationLog[]>("operation_log_query", { projectId, limit });
  },

  async getLogFileInfo(): Promise<LogFileInfo> {
    return await invoke<LogFileInfo>("log_get_info");
  },

  async readLogText(lines?: number): Promise<string> {
    return await invoke<string>("log_read_text", { lines });
  },

  async clearLogFile(): Promise<void> {
    return await invoke<void>("log_clear");
  },

  async openLogDir(): Promise<void> {
    return await invoke<void>("log_open_dir");
  },

  async openLogFile(): Promise<void> {
    return await invoke<void>("log_open_file");
  },

  // --- Config IO ---
  async exportConfig(outputPath: string): Promise<string> {
    return await invoke<string>("config_export", { outputPath });
  },

  async previewImportConfig(filePath: string): Promise<ImportPreviewResult> {
    return await invoke<ImportPreviewResult>("config_import_preview", { filePath });
  },

  async applyImportConfig(filePath: string, pathMappings: Record<string, string>): Promise<void> {
    return await invoke<void>("config_import_apply", { filePath, pathMappings });
  },
};
