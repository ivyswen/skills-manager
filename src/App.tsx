import React, { useState, useEffect, useCallback, useRef } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import { api } from "./services/api";
import {
  Repository,
  Project,
  Skill,
  Mount,
  ProjectDiagnostic,
  OperationLog,
  ConflictStrategy,
  AffectedSkill,
  ImportPreviewResult,
} from "./types";

import { TopToolbar } from "./components/layout/TopToolbar";
import { LeftSidebar } from "./components/layout/LeftSidebar";
import { MiddleSkills } from "./components/layout/MiddleSkills";
import { RightDetails } from "./components/layout/RightDetails";
import { BottomStatusBar } from "./components/layout/BottomStatusBar";
import { MarketView } from "./components/market/MarketView";

import { ConflictDialog } from "./components/modals/ConflictDialog";
import { DegradationDialog } from "./components/modals/DegradationDialog";
import { GitPullConfirmModal } from "./components/modals/GitPullConfirmModal";
import { ImportPreviewModal } from "./components/modals/ImportPreviewModal";
import { OnboardingWizard } from "./components/modals/OnboardingWizard";
import { OperationLogDrawer } from "./components/modals/OperationLogDrawer";
import { AddProjectModal } from "./components/modals/AddProjectModal";
import { RepoSettingsModal } from "./components/modals/RepoSettingsModal";

export const App: React.FC = () => {
  // 核心状态
  const [repository, setRepository] = useState<Repository | null>(null);
  const [projects, setProjects] = useState<Project[]>([]);
  const [skills, setSkills] = useState<Skill[]>([]);
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null);
  const [selectedSkillIds, setSelectedSkillIds] = useState<Set<string>>(new Set());
  const [selectedSkillDetail, setSelectedSkillDetail] = useState<Skill | null>(null);
  const [viewMode, setViewMode] = useState<"project" | "skill">("project");
  // 顶层视图切换：仓库 (3栏) vs 市场 (skills.sh)
  const [appView, setAppView] = useState<"repo" | "market">("repo");
  const [marketSearchQuery, setMarketSearchQuery] = useState("");
  const [marketActiveQuery, setMarketActiveQuery] = useState("");
  const [marketSearchTrigger, setMarketSearchTrigger] = useState(0);

  // 诊断与状态
  const [diagnosticsMap, setDiagnosticsMap] = useState<Record<string, ProjectDiagnostic>>({});
  const [logs, setLogs] = useState<OperationLog[]>([]);
  const [loading, setLoading] = useState(false);
  const [statusMessage, setStatusMessage] = useState<string | null>(null);
  const [statusType, setStatusType] = useState<"info" | "success" | "error">("info");
  const [recentBatchId, setRecentBatchId] = useState<string | null>(null);
  const [rollingBack, setRollingBack] = useState(false);

  // 技能更新与备份状态
  const [isCheckingUpdates, setIsCheckingUpdates] = useState(false);
  const [updatingSkillNames, setUpdatingSkillNames] = useState<Set<string>>(new Set());
  const [isBatchUpdating, setIsBatchUpdating] = useState(false);

  // 弹窗状态
  const [isOnboardingOpen, setIsOnboardingOpen] = useState(false);
  const [isRepoSettingsOpen, setIsRepoSettingsOpen] = useState(false);
  const [isAddProjectOpen, setIsAddProjectOpen] = useState(false);
  const [isLogsOpen, setIsLogsOpen] = useState(false);

  // 挂载模式选择（默认为 copy 模式，完美兼容 AGY 与 Antigravity）
  const [mountMode, setMountMode] = useState<"copy" | "junction" | "symlink">("copy");

  // 冲突处理弹窗
  const [conflictModalOpen, setConflictModalOpen] = useState(false);
  const [pendingConflictSkill, setPendingConflictSkill] = useState<{ id: string; name: string } | null>(null);

  // 降级选择弹窗
  const [degradationModalOpen, setDegradationModalOpen] = useState(false);
  const [degradationReason, setDegradationReason] = useState("");
  const [pendingDegradationSkills, setPendingDegradationSkills] = useState<string[]>([]);

  // Git 变更与脏检查弹窗
  const [gitModalOpen, setGitModalOpen] = useState(false);
  const [isGitDirtyWarning, setIsGitDirtyWarning] = useState(false);
  const [gitDirtyFiles, setGitDirtyFiles] = useState<string[]>([]);
  const [affectedSkills, setAffectedSkills] = useState<AffectedSkill[]>([]);

  // 配置导入预检弹窗
  const [importModalOpen, setImportModalOpen] = useState(false);
  const [importFilePath, setImportFilePath] = useState("");
  const [importPreviewData, setImportPreviewData] = useState<ImportPreviewResult | null>(null);

  // 消息提示辅助
  const showFeedback = (msg: string, type: "info" | "success" | "error" = "info") => {
    setStatusMessage(msg);
    setStatusType(type);
  };

  // 1. 初始化加载
  const loadInitialData = useCallback(async () => {
    setLoading(true);
    try {
      const repo = await api.getRepository();
      setRepository(repo);

      const projs = await api.listProjects();
      setProjects(projs);

      if (repo) {
        const scanRes = await api.scanRepository();
        setSkills(scanRes.skills);
      }

      if (projs.length > 0) {
        setActiveProjectId((prev) => prev || projs[0].id);
      }

      // 如果完全没有仓库且没有项目，唤起新手引导
      if (!repo && projs.length === 0) {
        setIsOnboardingOpen(true);
      }
    } catch (e: any) {
      showFeedback(`初始化加载异常: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  }, []);

  const hasInitializedRef = useRef(false);

  useEffect(() => {
    if (hasInitializedRef.current) return;
    hasInitializedRef.current = true;
    loadInitialData();
  }, [loadInitialData]);

  // 2. 诊断当前活跃项目
  const diagnoseActiveProject = useCallback(async (projId: string) => {
    try {
      const diag = await api.diagnoseProject(projId);
      setDiagnosticsMap((prev) => ({ ...prev, [projId]: diag }));
    } catch (e: any) {
      console.error("Diagnosis error:", e);
    }
  }, []);

  useEffect(() => {
    if (activeProjectId) {
      diagnoseActiveProject(activeProjectId);
    }
  }, [activeProjectId, diagnoseActiveProject]);

  // 全局刷新
  const handleRefreshAll = async () => {
    setLoading(true);
    try {
      if (repository) {
        const scanRes = await api.scanRepository();
        setSkills(scanRes.skills);
      }
      const projs = await api.listProjects();
      setProjects(projs);
      for (const p of projs) {
        await diagnoseActiveProject(p.id);
      }
      showFeedback("已成功刷新所有数据与挂载诊断", "success");
    } catch (e: any) {
      showFeedback(`刷新失败: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  };

  // 3. 挂载操作 (带冲突策略与降级支持)
  const handleExecuteMount = async (
    skillIds: string[],
    conflictStrategy: ConflictStrategy = "cancel",
    preferredMode?: "symlink" | "junction" | "copy",
    keepSelection: boolean = false
  ) => {
    if (!activeProjectId) {
      showFeedback("请先在左侧选择一个目标项目", "error");
      return;
    }
    setLoading(true);
    try {
      const mode = preferredMode || mountMode;
      const results = await api.mountSkills(
        skillIds,
        activeProjectId,
        conflictStrategy,
        mode
      );

      const successCount = results.filter((r) => r.success).length;
      const failed = results.filter((r) => !r.success);

      if (results.length > 0 && results[0].batch_id) {
        setRecentBatchId(results[0].batch_id);
      }

      // 提取批次并更新诊断
      await diagnoseActiveProject(activeProjectId);

      if (failed.length === 0) {
        showFeedback(`成功挂载 ${successCount} 个 Skill (${mode.toUpperCase()} 模式)`, "success");
        if (!keepSelection) {
          setSelectedSkillIds(new Set());
        }
      } else {
        // 检查失败原因是否为 Windows 权限不足或跨盘符
        const symlinkErr = failed.find(
          (f) =>
            f.error_code === "SYMLINK_PERMISSION_DENIED" ||
            f.error_message?.includes("1314")
        );
        if (symlinkErr) {
          setDegradationReason(symlinkErr.error_message || "");
          setPendingDegradationSkills(failed.map((f) => f.skill_id));
          setDegradationModalOpen(true);
          return;
        }

        // 检查是否为冲突
        const conflictErr = failed.find((f) => f.error_code === "TARGET_CONFLICT");
        if (conflictErr) {
          setPendingConflictSkill({ id: conflictErr.skill_id, name: conflictErr.skill_name });
          setConflictModalOpen(true);
          return;
        }

        if (successCount > 0) {
          showFeedback(`挂载完成：${successCount} 成功，${failed.length} 失败。可在底部状态栏回滚本批次。`, "error");
        } else {
          showFeedback(`挂载失败：${failed.length} 项未成功 (${failed[0]?.error_message || "未知错误"})`, "error");
        }
      }
    } catch (e: any) {
      showFeedback(`挂载异常: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  };

  // 卸载操作
  const handleUnmount = async (mount: Mount) => {
    try {
      const results = await api.unmountSkills([mount.id]);
      if (results.some((r) => r.success)) {
        showFeedback(`已成功卸载挂载: ${mount.skill_name}`, "success");
        if (activeProjectId) {
          await diagnoseActiveProject(activeProjectId);
        }
      } else {
        showFeedback(`卸载失败: ${results[0]?.error_message || "未知错误"}`, "error");
      }
    } catch (e: any) {
      showFeedback(`卸载异常: ${e?.message || e}`, "error");
    }
  };

  // 修复 / 同步操作
  const handleRepair = async (mount: Mount) => {
    try {
      await api.repairMount(mount.id);
      showFeedback(`已完成挂载同步/修复: ${mount.skill_name}`, "success");
      if (activeProjectId) {
        await diagnoseActiveProject(activeProjectId);
      }
    } catch (e: any) {
      showFeedback(`操作失败: ${e?.message || e}`, "error");
    }
  };

  // 挂载模式一键切换
  const handleSwitchMountMode = async (mount: Mount, targetMode: "copy" | "junction" | "symlink") => {
    setLoading(true);
    try {
      await api.repairMount(mount.id, targetMode);
      const modeName = targetMode === "copy" ? "Copy 物理副本 (AGY 兼容)" : targetMode === "junction" ? "Junction 联接点" : "Symlink 软链接";
      showFeedback(`已将 ${mount.skill_name} 成功切换为 ${modeName} 模式`, "success");
      if (activeProjectId) {
        await diagnoseActiveProject(activeProjectId);
      }
    } catch (e: any) {
      showFeedback(`模式切换失败: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  };

  // 单个 Skill 快速挂载（并激活该卡片的选中状态）
  const handleQuickMount = async (skill: Skill) => {
    setSelectedSkillDetail(skill);
    setSelectedSkillIds((prev) => {
      const next = new Set(prev);
      next.add(skill.id);
      return next;
    });
    await handleExecuteMount([skill.id], "cancel", mountMode, true);
  };

  // 批次回滚
  const handleRollback = async (batchId: string) => {
    setRollingBack(true);
    try {
      await api.rollbackBatch(batchId);
      showFeedback(`已成功回滚批次操作并还原备份`, "success");
      setRecentBatchId(null);
      if (activeProjectId) {
        await diagnoseActiveProject(activeProjectId);
      }
    } catch (e: any) {
      showFeedback(`回滚失败: ${e?.message || e}`, "error");
    } finally {
      setRollingBack(false);
    }
  };

  // Git 更新
  const handleGitPull = async () => {
    if (!repository || repository.source_type !== "git") return;
    setLoading(true);
    try {
      // 先做状态检查
      const status = await api.getGitStatus();
      if (status.is_dirty) {
        setIsGitDirtyWarning(true);
        setGitDirtyFiles(status.changed_files);
        setGitModalOpen(true);
        setLoading(false);
        return;
      }

      // 执行 pull
      const pullRes = await api.gitPull();
      if (pullRes.success) {
        if (pullRes.affected_skills.length > 0) {
          setIsGitDirtyWarning(false);
          setAffectedSkills(pullRes.affected_skills);
          setGitModalOpen(true);
        } else {
          showFeedback(
            pullRes.updated ? "Git 拉取成功，原件已更新，链接无需重建" : "中央仓库已是最新版本",
            "success"
          );
        }
        await handleRefreshAll();
      }
    } catch (e: any) {
      showFeedback(`Git 操作失败: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  };

  // Git 删除/重命名清理
  const handleConfirmGitCleanup = async (skillNames: string[]) => {
    setGitModalOpen(false);
    if (skillNames.length === 0) return;
    setLoading(true);
    try {
      // 遍历各个项目卸载已选失效技能
      for (const p of projects) {
        const diag = diagnosticsMap[p.id];
        if (diag) {
          const toRemove = diag.mounts.filter((m) => skillNames.includes(m.skill_name));
          if (toRemove.length > 0) {
            await api.unmountSkills(toRemove.map((m) => m.id), true);
            await diagnoseActiveProject(p.id);
          }
        }
      }
      showFeedback(`已批量清理 ${skillNames.length} 个失效 Skill 挂载`, "success");
    } catch (e: any) {
      showFeedback(`清理失败: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  };

  // 检查技能更新
  const handleCheckSkillUpdates = async (skillName?: string) => {
    setIsCheckingUpdates(true);
    try {
      const results = await api.checkSkillUpdates(skillName);
      if (repository) {
        const scanRes = await api.scanRepository();
        setSkills(scanRes.skills);
        if (selectedSkillDetail) {
          const refreshed = scanRes.skills.find((s) => s.id === selectedSkillDetail.id);
          if (refreshed) setSelectedSkillDetail(refreshed);
        }
      }
      if (skillName) {
        const target = results.find((r) => r.skill_name === skillName);
        if (target?.has_update) {
          showFeedback(`技能 "${skillName}" 发现新版本！`, "info");
        } else {
          showFeedback(`技能 "${skillName}" 已是最新版本`, "success");
        }
      } else {
        const updatableCount = results.filter((r) => r.has_update).length;
        if (updatableCount > 0) {
          showFeedback(`更新检查完成：发现 ${updatableCount} 个技能有可用新版本`, "info");
        } else {
          showFeedback(`检查完成：当前所有可追踪技能均已是最新`, "success");
        }
      }
    } catch (e: any) {
      showFeedback(`检查更新失败: ${e?.message || e}`, "error");
    } finally {
      setIsCheckingUpdates(false);
    }
  };

  // 单个技能更新
  const handleUpdateSkill = async (skill: Skill) => {
    setUpdatingSkillNames((prev) => {
      const next = new Set(prev);
      next.add(skill.name);
      return next;
    });
    try {
      const res = await api.updateSkill(skill.name);
      if (res.success) {
        showFeedback(`技能 "${skill.name}" 已成功更新至最新版本！`, "success");
        if (repository) {
          const scanRes = await api.scanRepository();
          setSkills(scanRes.skills);
          const refreshed = scanRes.skills.find((s) => s.id === skill.id || s.name === skill.name);
          if (refreshed) setSelectedSkillDetail(refreshed);
        }
        if (activeProjectId) {
          await diagnoseActiveProject(activeProjectId);
        }
      } else {
        showFeedback(`技能 "${skill.name}" 更新失败: ${res.message || "未知原因"}`, "error");
      }
    } catch (e: any) {
      showFeedback(`更新异常: ${e?.message || e}`, "error");
    } finally {
      setUpdatingSkillNames((prev) => {
        const next = new Set(prev);
        next.delete(skill.name);
        return next;
      });
    }
  };

  // 批量更新技能
  const handleBatchUpdate = async (skillNames?: string[]) => {
    setIsBatchUpdating(true);
    try {
      const res = await api.batchUpdateSkills(skillNames);
      showFeedback(
        `批量更新完成：成功 ${res.success_count} 个，失败 ${res.failed_count} 个`,
        res.failed_count === 0 ? "success" : "info"
      );
      if (repository) {
        const scanRes = await api.scanRepository();
        setSkills(scanRes.skills);
        if (selectedSkillDetail) {
          const refreshed = scanRes.skills.find((s) => s.id === selectedSkillDetail.id);
          if (refreshed) setSelectedSkillDetail(refreshed);
        }
      }
      if (activeProjectId) {
        await diagnoseActiveProject(activeProjectId);
      }
    } catch (e: any) {
      showFeedback(`批量更新失败: ${e?.message || e}`, "error");
    } finally {
      setIsBatchUpdating(false);
    }
  };

  // 备份还原后同步数据
  const handleSkillRestored = async () => {
    showFeedback("已成功从安全备份快照还原技能", "success");
    if (repository) {
      const scanRes = await api.scanRepository();
      setSkills(scanRes.skills);
      if (selectedSkillDetail) {
        const refreshed = scanRes.skills.find((s) => s.id === selectedSkillDetail.id);
        if (refreshed) setSelectedSkillDetail(refreshed);
      }
    }
    if (activeProjectId) {
      await diagnoseActiveProject(activeProjectId);
    }
  };

  // 配置导出
  const handleExportConfig = async () => {
    try {
      const selected = await save({
        filters: [{ name: "JSON", extensions: ["json"] }],
        defaultPath: "skills-manager-backup.json",
      });
      if (selected) {
        const msg = await api.exportConfig(selected);
        showFeedback(msg, "success");
      }
    } catch (e: any) {
      showFeedback(`导出失败: ${e?.message || e}`, "error");
    }
  };

  // 配置导入
  const handleImportConfig = async () => {
    try {
      const selected = await open({
        filters: [{ name: "JSON", extensions: ["json"] }],
        multiple: false,
      });
      if (selected && typeof selected === "string") {
        const preview = await api.previewImportConfig(selected);
        setImportFilePath(selected);
        setImportPreviewData(preview);
        setImportModalOpen(true);
      }
    } catch (e: any) {
      showFeedback(`导入预检失败: ${e?.message || e}`, "error");
    }
  };

  const handleApplyImport = async (mappings: Record<string, string>) => {
    setImportModalOpen(false);
    setLoading(true);
    try {
      await api.applyImportConfig(importFilePath, mappings);
      showFeedback("配置导入成功，已恢复项目及挂载设置", "success");
      await loadInitialData();
    } catch (e: any) {
      showFeedback(`应用配置失败: ${e?.message || e}`, "error");
    } finally {
      setLoading(false);
    }
  };

  // 入口链接操作
  const handleSetupEntryLink = async (agentType: string, linkPath: string) => {
    if (!activeProjectId) return;
    try {
      await api.setupEntryLink(activeProjectId, agentType, linkPath);
      showFeedback(`成功创建 Agent 入口软链接: ${linkPath}`, "success");
      await diagnoseActiveProject(activeProjectId);
    } catch (e: any) {
      showFeedback(`创建入口链接失败: ${e?.message || e}`, "error");
    }
  };

  const handleRemoveEntryLink = async (linkId: string, linkPath: string) => {
    if (!activeProjectId) return;
    try {
      await api.removeEntryLink(linkId, activeProjectId, linkPath);
      showFeedback(`已删除入口软链接: ${linkPath}`, "success");
      await diagnoseActiveProject(activeProjectId);
    } catch (e: any) {
      showFeedback(`删除入口链接失败: ${e?.message || e}`, "error");
    }
  };

  const handleMarketSearchSubmit = (overrideQuery?: string) => {
    const q = (overrideQuery !== undefined ? overrideQuery : marketSearchQuery).trim();
    if (!q) {
      showFeedback("请输入技能名称或关键词后再搜索", "info");
      return;
    }
    if (overrideQuery !== undefined) {
      setMarketSearchQuery(overrideQuery);
    }
    setAppView("market");
    setMarketActiveQuery(q);
    setMarketSearchTrigger((prev) => prev + 1);
  };

  const activeProject = projects.find((p) => p.id === activeProjectId) || null;
  const activeDiagnostic = activeProjectId ? diagnosticsMap[activeProjectId] || null : null;

  return (
    <div className="flex flex-col h-screen bg-slate-950 text-slate-100 font-sans">
      {/* 顶部导航 */}
      <TopToolbar
        repository={repository}
        currentView={appView}
        onViewChange={setAppView}
        marketSearchQuery={marketSearchQuery}
        onMarketSearchChange={setMarketSearchQuery}
        onMarketSearchSubmit={() => handleMarketSearchSubmit()}
        onRefreshAll={handleRefreshAll}
        onGitPull={handleGitPull}
        onOpenLogs={() => {
          api.queryLogs().then(setLogs);
          setIsLogsOpen(true);
        }}
        onOpenExport={handleExportConfig}
        onOpenImport={handleImportConfig}
        onAddProject={() => setIsAddProjectOpen(true)}
        onAddRepo={() => setIsRepoSettingsOpen(true)}
        loading={loading}
      />

      {/* 主体展示区：市场视图 vs 仓库3栏挂载视图 */}
      {appView === "market" ? (
        <MarketView
          searchQuery={marketActiveQuery}
          searchTrigger={marketSearchTrigger}
          onQuickSearch={(q) => handleMarketSearchSubmit(q)}
          onSyncRepo={async () => {
            if (repository) {
              const scanRes = await api.scanRepository();
              setSkills(scanRes.skills);
            }
            await loadInitialData();
          }}
          onShowFeedback={showFeedback}
        />
      ) : (
        <div className="flex-1 flex overflow-hidden">
          {/* 左侧栏：仓库与项目 */}
          <LeftSidebar
          repository={repository}
          projects={projects}
          activeProjectId={activeProjectId}
          diagnosticsMap={diagnosticsMap}
          onSelectProject={(id) => {
            setActiveProjectId(id);
            setViewMode("project");
          }}
          onRelocateProject={async (proj) => {
            const selected = await open({ directory: true, multiple: false });
            if (selected && typeof selected === "string") {
              await api.relocateProject(proj.id, selected);
              showFeedback(`项目路径已重定位至 ${selected}`, "success");
              await loadInitialData();
            }
          }}
          onRemoveProject={async (proj) => {
            if (confirm(`确定要从工具中移除项目 “${proj.name}” 的登记吗？`)) {
              await api.removeProject(proj.id, false);
              showFeedback(`已移除项目: ${proj.name}`, "success");
              await loadInitialData();
            }
          }}
          onOpenRepoSettings={() => setIsRepoSettingsOpen(true)}
          onAddProject={() => setIsAddProjectOpen(true)}
        />

        {/* 中间栏：Skill 检索与卡片 */}
        <MiddleSkills
          skills={skills}
          activeProject={activeProject}
          projectMounts={activeDiagnostic?.mounts ?? []}
          selectedSkillIds={selectedSkillIds}
          activeSkillId={selectedSkillDetail?.id}
          onToggleSkillSelect={(id) => {
            const next = new Set(selectedSkillIds);
            if (next.has(id)) next.delete(id);
            else next.add(id);
            setSelectedSkillIds(next);
          }}
          onSelectAllSkills={(ids) => setSelectedSkillIds(new Set(ids))}
          onClearSkillSelect={() => setSelectedSkillIds(new Set())}
          onSelectSkillDetail={(skill) => {
            setSelectedSkillDetail(skill);
            setViewMode("skill");
          }}
          onQuickMount={handleQuickMount}
          onCheckUpdates={() => handleCheckSkillUpdates()}
          isCheckingUpdates={isCheckingUpdates}
          onUpdateSkill={handleUpdateSkill}
          onBatchUpdate={handleBatchUpdate}
          updatingSkillNames={updatingSkillNames}
          isBatchUpdating={isBatchUpdating}
        />

        {/* 右侧栏：项目挂载看板 / Skill 详情看板 */}
        <RightDetails
          viewMode={viewMode}
          activeProject={activeProject}
          selectedSkill={selectedSkillDetail}
          projectDiagnostic={activeDiagnostic}
          onMountRepair={handleRepair}
          onUnmountSkill={handleUnmount}
          onBatchMountSelected={() => handleExecuteMount(Array.from(selectedSkillIds))}
          selectedSkillsCount={selectedSkillIds.size}
          onSetupEntryLink={handleSetupEntryLink}
          onRemoveEntryLink={handleRemoveEntryLink}
          onSwitchToProjectView={() => setViewMode("project")}
          allProjects={projects}
          mountMode={mountMode}
          onMountModeChange={setMountMode}
          onSwitchMountMode={handleSwitchMountMode}
          onUpdateSkill={handleUpdateSkill}
          onCheckSkillUpdate={(name) => handleCheckSkillUpdates(name)}
          isUpdatingSkill={selectedSkillDetail ? updatingSkillNames.has(selectedSkillDetail.name) : false}
          isCheckingUpdate={isCheckingUpdates}
          onSkillRestored={handleSkillRestored}
        />
      </div>
      )}

      {/* 底部状态条 */}
      <BottomStatusBar
        lastMessage={statusMessage}
        lastMessageType={statusType}
        recentBatchId={recentBatchId}
        onRollbackBatch={handleRollback}
        rollingBack={rollingBack}
      />

      {/* 全部模态交互弹窗 */}
      <ConflictDialog
        isOpen={conflictModalOpen}
        skillName={pendingConflictSkill?.name || ""}
        targetPath={
          activeProject && pendingConflictSkill
            ? `${activeProject.path}/.agents/skills/${pendingConflictSkill.name}`
            : ""
        }
        onConfirm={(strategy) => {
          setConflictModalOpen(false);
          if (pendingConflictSkill) {
            handleExecuteMount([pendingConflictSkill.id], strategy);
          }
        }}
        onClose={() => setConflictModalOpen(false)}
      />

      <DegradationDialog
        isOpen={degradationModalOpen}
        reason={degradationReason}
        onSelectMode={(mode) => {
          setDegradationModalOpen(false);
          handleExecuteMount(pendingDegradationSkills, "backup_and_replace", mode);
        }}
        onClose={() => setDegradationModalOpen(false)}
      />

      <GitPullConfirmModal
        isOpen={gitModalOpen}
        isDirtyWarning={isGitDirtyWarning}
        dirtyFiles={gitDirtyFiles}
        affectedSkills={affectedSkills}
        onConfirmCleanup={handleConfirmGitCleanup}
        onClose={() => setGitModalOpen(false)}
      />

      {importPreviewData && (
        <ImportPreviewModal
          isOpen={importModalOpen}
          filePath={importFilePath}
          previewData={importPreviewData}
          onConfirmApply={handleApplyImport}
          onClose={() => setImportModalOpen(false)}
        />
      )}

      <OnboardingWizard
        isOpen={isOnboardingOpen}
        onComplete={async (repoPath, repoName, projectPath, projectName) => {
          await api.addRepository(repoPath, repoName);
          if (projectPath) {
            await api.addProject(projectPath, projectName);
          }
          setIsOnboardingOpen(false);
          await loadInitialData();
          showFeedback("新手引导初始化完成，欢迎使用！", "success");
        }}
      />

      <OperationLogDrawer
        isOpen={isLogsOpen}
        logs={logs}
        onRollbackBatch={handleRollback}
        onRefreshLogs={() => api.queryLogs().then(setLogs)}
        onClose={() => setIsLogsOpen(false)}
        loading={loading}
      />

      <AddProjectModal
        isOpen={isAddProjectOpen}
        onAddProject={async (path, name) => {
          const proj = await api.addProject(path, name);
          showFeedback(`成功添加项目: ${proj.name}`, "success");
          await loadInitialData();
          setActiveProjectId(proj.id);
        }}
        onClose={() => setIsAddProjectOpen(false)}
      />

      <RepoSettingsModal
        isOpen={isRepoSettingsOpen}
        currentRepo={repository}
        onSaveRepo={async (path, name) => {
          const repo = await api.addRepository(path, name);
          showFeedback(`中央仓库已设置为: ${repo.name}`, "success");
          await loadInitialData();
        }}
        onClose={() => setIsRepoSettingsOpen(false)}
      />
    </div>
  );
};

export default App;
