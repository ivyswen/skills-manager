import React, { useState, useEffect, useCallback } from "react";
import {
  Folder,
  ExternalLink,
  AlertTriangle,
  CheckCircle2,
  XCircle,
  HelpCircle,
  Wrench,
  Trash2,
  Plus,
  ShieldAlert,
  Link2,
  Repeat,
  RefreshCw,
  ArrowUpCircle,
  History,
  Archive,
  GitFork,
  Loader2,
  RotateCcw,
} from "lucide-react";
import { api } from "../../services/api";
import { Project, Skill, Mount, ProjectDiagnostic, SkillBackupItem } from "../../types";

interface RightDetailsProps {
  viewMode: "project" | "skill";
  activeProject: Project | null;
  selectedSkill: Skill | null;
  projectDiagnostic: ProjectDiagnostic | null;
  onMountRepair: (mount: Mount) => void;
  onUnmountSkill: (mount: Mount) => void;
  onBatchMountSelected: () => void;
  selectedSkillsCount: number;
  onSetupEntryLink: (agentType: string, linkPath: string) => void;
  onRemoveEntryLink: (linkId: string, linkPath: string) => void;
  onSwitchToProjectView: () => void;
  allProjects?: Project[];
  mountMode?: "copy" | "junction" | "symlink";
  onMountModeChange?: (mode: "copy" | "junction" | "symlink") => void;
  onSwitchMountMode?: (mount: Mount, targetMode: "copy" | "junction" | "symlink") => void;
  onUpdateSkill?: (skill: Skill) => void;
  onCheckSkillUpdate?: (skillName: string) => void;
  isUpdatingSkill?: boolean;
  isCheckingUpdate?: boolean;
  onSkillRestored?: () => void;
}

export const RightDetails: React.FC<RightDetailsProps> = ({
  viewMode,
  activeProject,
  selectedSkill,
  projectDiagnostic,
  onMountRepair,
  onUnmountSkill,
  onBatchMountSelected,
  selectedSkillsCount,
  onSetupEntryLink,
  onRemoveEntryLink,
  onSwitchToProjectView,
  mountMode = "copy",
  onMountModeChange,
  onSwitchMountMode,
  onUpdateSkill,
  onCheckSkillUpdate,
  isUpdatingSkill = false,
  isCheckingUpdate = false,
  onSkillRestored,
}) => {
  const [backups, setBackups] = useState<SkillBackupItem[]>([]);
  const [loadingBackups, setLoadingBackups] = useState(false);
  const [restoringBackupId, setRestoringBackupId] = useState<string | null>(null);

  const fetchBackups = useCallback(async (skillName: string) => {
    setLoadingBackups(true);
    try {
      const list = await api.listSkillBackups(skillName);
      setBackups(list);
    } catch (e) {
      console.error("Failed to fetch backups:", e);
    } finally {
      setLoadingBackups(false);
    }
  }, []);

  useEffect(() => {
    if (viewMode === "skill" && selectedSkill) {
      fetchBackups(selectedSkill.name);
    }
  }, [viewMode, selectedSkill?.name, selectedSkill?.content_hash, selectedSkill?.updated_at, fetchBackups]);

  const handleRestore = async (backup: SkillBackupItem) => {
    if (
      !window.confirm(
        `确定要从备份 ${backup.id} 恢复技能 "${backup.skill_name}" 吗？\n恢复前将自动为当前版本创建安全备份。`
      )
    ) {
      return;
    }
    setRestoringBackupId(backup.id);
    try {
      const res = await api.restoreSkillBackup(backup.id);
      if (res.success) {
        onSkillRestored?.();
        if (selectedSkill) {
          await fetchBackups(selectedSkill.name);
        }
      }
    } catch (e: any) {
      alert(`恢复备份失败: ${e?.message || e}`);
    } finally {
      setRestoringBackupId(null);
    }
  };

  const handleDeleteBackup = async (backupId: string) => {
    if (!window.confirm(`确定要删除此备份吗？删除后将无法恢复此历史快照。`)) {
      return;
    }
    try {
      await api.deleteSkillBackup(backupId);
      if (selectedSkill) {
        await fetchBackups(selectedSkill.name);
      }
    } catch (e: any) {
      alert(`删除备份失败: ${e?.message || e}`);
    }
  };

  const handleOpenFolder = async (path: string) => {
    try {
      await api.openFolder(path);
    } catch (e) {
      console.error("Failed to open folder:", e);
    }
  };

  const renderStatusBadge = (status: string, isOutdated?: boolean | null) => {
    switch (status) {
      case "NORMAL":
        return (
          <div className="flex items-center space-x-1.5">
            <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
              <CheckCircle2 className="w-3 h-3" />
              <span>正常</span>
            </span>
            {isOutdated && (
              <span className="inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20" title="原件内容已更新，副本处于陈旧状态">
                副本陈旧
              </span>
            )}
          </div>
        );
      case "BROKEN":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
            <XCircle className="w-3 h-3" />
            <span>断链</span>
          </span>
        );
      case "WRONG_TARGET":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-medium bg-amber-500/10 text-amber-400 border border-amber-500/20">
            <AlertTriangle className="w-3 h-3" />
            <span>指向错误</span>
          </span>
        );
      case "CONFLICT":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-medium bg-rose-500/10 text-rose-400 border border-rose-500/20">
            <ShieldAlert className="w-3 h-3" />
            <span>占用冲突</span>
          </span>
        );
      case "PERMISSION_DENIED":
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-medium bg-orange-500/10 text-orange-400 border border-orange-500/20">
            <ShieldAlert className="w-3 h-3" />
            <span>权限拒绝</span>
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center space-x-1 px-2 py-0.5 rounded text-[11px] font-medium bg-slate-800 text-slate-400 border border-slate-700">
            <HelpCircle className="w-3 h-3" />
            <span>{status}</span>
          </span>
        );
    }
  };

  // 模式 B: Skill 详情看板
  if (viewMode === "skill" && selectedSkill) {
    return (
      <main className="flex-1 bg-slate-950 flex flex-col h-full overflow-y-auto select-none p-6 space-y-6">
        {/* 顶部标题栏 */}
        <div className="flex items-start justify-between border-b border-slate-800 pb-4">
          <div>
            <div className="flex items-center space-x-2.5">
              <h2 className="text-xl font-bold text-slate-100">{selectedSkill.name}</h2>
              <span className={`text-xs px-2 py-0.5 rounded border ${
                selectedSkill.metadata_status === "valid"
                  ? "bg-teal-500/10 border-teal-500/30 text-teal-400"
                  : "bg-amber-500/10 border-amber-500/30 text-amber-400"
              }`}>
                {selectedSkill.metadata_status === "valid" ? "元数据完整" : "元数据不完整"}
              </span>
              {selectedSkill.has_update && (
                <span className="text-xs px-2 py-0.5 rounded bg-blue-500/20 border border-blue-500/40 text-blue-300 font-medium flex items-center space-x-1">
                  <ArrowUpCircle className="w-3.5 h-3.5" />
                  <span>新版本可用</span>
                </span>
              )}
            </div>
            <p className="text-xs text-slate-400 mt-1 font-mono">{selectedSkill.canonical_path}</p>
            {selectedSkill.source && (
              <div className="flex items-center space-x-2 mt-2 text-xs text-slate-400">
                <GitFork className="w-3.5 h-3.5 text-slate-500" />
                <span>上游开源仓库:</span>
                <button
                  onClick={() => api.openUrl(`https://github.com/${selectedSkill.source}`)}
                  className="text-blue-400 hover:text-blue-300 inline-flex items-center space-x-0.5 transition-colors"
                  title="在浏览器中查看 GitHub 源码库"
                >
                  <ExternalLink className="w-3 h-3" />
                </button>
              </div>
            )}
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={() => handleOpenFolder(selectedSkill.canonical_path)}
              className="inline-flex items-center space-x-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
            >
              <ExternalLink className="w-3.5 h-3.5" />
              <span>在文件管理器中打开</span>
            </button>
            <button
              onClick={onSwitchToProjectView}
              className="px-3 py-1.5 text-xs font-medium rounded-md bg-teal-600 hover:bg-teal-500 text-white transition-colors"
            >
              返回项目看板
            </button>
          </div>
        </div>

        {/* 版本更新比对状态卡片 */}
        <div className={`border rounded-lg p-4 transition-colors ${
          selectedSkill.has_update
            ? "bg-blue-950/20 border-blue-800/60"
            : "bg-slate-900 border-slate-800"
        }`}>
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <div className={`p-2.5 rounded-lg ${
                selectedSkill.has_update
                  ? "bg-blue-500/20 text-blue-400"
                  : "bg-slate-800 text-slate-400"
              }`}>
                <RefreshCw className={`w-5 h-5 ${isCheckingUpdate ? "animate-spin text-blue-400" : ""}`} />
              </div>
              <div>
                <h3 className="text-sm font-semibold text-slate-200">
                  {selectedSkill.has_update
                    ? "检测到上游仓库存在更新版本"
                    : selectedSkill.source
                    ? "已是最新版本"
                    : "本地技能 (未关联开源源)"}
                </h3>
                <p className="text-xs text-slate-400 mt-0.5">
                  {selectedSkill.has_update ? (
                    <span>
                      远端哈希: <span className="font-mono text-blue-300">{selectedSkill.remote_hash?.slice(0, 10)}...</span>
                      {" | "}
                      本地哈希: <span className="font-mono text-slate-300">{selectedSkill.content_hash.slice(0, 10)}...</span>
                    </span>
                  ) : selectedSkill.source ? (
                    <span>
                      当前版本哈希: <span className="font-mono text-slate-300">{selectedSkill.content_hash.slice(0, 10)}...</span>
                      {selectedSkill.last_checked_at && (
                        <> · 上次检查: {new Date(selectedSkill.last_checked_at).toLocaleTimeString()}</>
                      )}
                    </span>
                  ) : (
                    <span>在市场中安装的技能可自动在线接收上游更新</span>
                  )}
                </p>
              </div>
            </div>

            <div className="flex items-center space-x-2">
              {selectedSkill.source && (
                <button
                  onClick={() => onCheckSkillUpdate?.(selectedSkill.name)}
                  disabled={isCheckingUpdate || isUpdatingSkill}
                  className="inline-flex items-center space-x-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-300 transition-colors disabled:opacity-50"
                  title="检查远端 Git 源码库是否有更新"
                >
                  <RefreshCw className={`w-3.5 h-3.5 ${isCheckingUpdate ? "animate-spin text-blue-400" : ""}`} />
                  <span>{isCheckingUpdate ? "检查中..." : "检查更新"}</span>
                </button>
              )}

              {selectedSkill.has_update && (
                <button
                  onClick={() => onUpdateSkill?.(selectedSkill)}
                  disabled={isUpdatingSkill}
                  className="inline-flex items-center space-x-1.5 px-3.5 py-1.5 text-xs font-semibold rounded-md bg-blue-600 hover:bg-blue-500 text-white shadow transition-colors disabled:opacity-50"
                  title="执行安全自动备份并更新至上游最新代码"
                >
                  {isUpdatingSkill ? (
                    <Loader2 className="w-3.5 h-3.5 animate-spin" />
                  ) : (
                    <ArrowUpCircle className="w-3.5 h-3.5" />
                  )}
                  <span>{isUpdatingSkill ? "正在安全更新..." : "立即更新技能"}</span>
                </button>
              )}
            </div>
          </div>
        </div>

        {/* 描述与指标 */}
        <div className="grid grid-cols-3 gap-4">
          <div className="col-span-2 bg-slate-900 border border-slate-800 rounded-lg p-4">
            <h3 className="text-xs font-semibold text-slate-400 uppercase mb-2">Skill 描述</h3>
            <p className="text-xs text-slate-300 leading-relaxed whitespace-pre-wrap">
              {selectedSkill.description || "暂无描述"}
            </p>
          </div>

          <div className="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-3">
            <h3 className="text-xs font-semibold text-slate-400 uppercase mb-1">客观度量</h3>
            <div className="flex items-center justify-between text-xs">
              <span className="text-slate-400">内部文件数:</span>
              <span className="font-semibold text-slate-200">{selectedSkill.file_count}</span>
            </div>
            <div className="flex items-center justify-between text-xs">
              <span className="text-slate-400">总字符数:</span>
              <span className="font-semibold text-slate-200">{selectedSkill.char_count} 字符</span>
            </div>
            <div className="flex items-center justify-between text-xs">
              <span className="text-slate-400">内容哈希:</span>
              <span className="font-mono text-[10px] text-slate-400 truncate max-w-[120px]" title={selectedSkill.content_hash}>
                {selectedSkill.content_hash.slice(0, 12)}...
              </span>
            </div>
          </div>
        </div>

        {/* 安全备份历史与版本恢复看板 (参照 cc-switch 规范) */}
        <div className="bg-slate-900 border border-slate-800 rounded-lg p-4 space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-2">
              <History className="w-4 h-4 text-slate-400" />
              <h3 className="text-xs font-semibold text-slate-200 uppercase">
                安全备份与历史回退 ({backups.length})
              </h3>
            </div>
            <span className="text-[11px] text-slate-500">
              更新或修改前自动创建快照，最多保留 20 个历史副本
            </span>
          </div>

          {loadingBackups ? (
            <div className="py-8 text-center text-slate-500 text-xs flex items-center justify-center space-x-2">
              <Loader2 className="w-4 h-4 animate-spin text-teal-400" />
              <span>正在读取备份快照...</span>
            </div>
          ) : backups.length === 0 ? (
            <div className="py-8 text-center text-slate-500 text-xs border border-dashed border-slate-800 rounded-md">
              <Archive className="w-8 h-8 mx-auto mb-2 opacity-30 text-slate-400" />
              <p>暂无该技能的历史备份快照</p>
              <p className="text-[10px] text-slate-600 mt-1">
                在线更新或恢复时，系统会自动在此创建原件快照以保安全
              </p>
            </div>
          ) : (
            <div className="border border-slate-800 rounded-md divide-y divide-slate-800/80 max-h-60 overflow-y-auto">
              {backups.map((b) => {
                const isRestoring = restoringBackupId === b.id;
                return (
                  <div
                    key={b.id}
                    className="p-2.5 flex items-center justify-between hover:bg-slate-800/40 transition-colors text-xs"
                  >
                    <div className="space-y-0.5">
                      <div className="flex items-center space-x-2">
                        <span className="font-mono text-slate-300 font-medium">{b.id}</span>
                        {b.reason && (
                          <span className="text-[10px] px-1.5 py-0.2 rounded bg-slate-800 border border-slate-700 text-slate-400">
                            {b.reason === "pre-update"
                              ? "更新前快照"
                              : b.reason === "pre-restore"
                              ? "恢复前快照"
                              : b.reason}
                          </span>
                        )}
                      </div>
                      <div className="flex items-center space-x-2 text-[10px] text-slate-500">
                        <span>快照哈希: {b.content_hash ? b.content_hash.slice(0, 12) + "..." : "无"}</span>
                        <span>·</span>
                        <span>{new Date(b.created_at).toLocaleString()}</span>
                      </div>
                    </div>

                    <div className="flex items-center space-x-1.5">
                      <button
                        onClick={() => handleRestore(b)}
                        disabled={isRestoring}
                        className="inline-flex items-center space-x-1 px-2.5 py-1 rounded bg-teal-950 hover:bg-teal-900 border border-teal-800/60 text-teal-300 font-medium transition-colors disabled:opacity-50"
                        title="将此历史快照恢复为当前中央仓库原件"
                      >
                        {isRestoring ? (
                          <Loader2 className="w-3 h-3 animate-spin" />
                        ) : (
                          <RotateCcw className="w-3 h-3" />
                        )}
                        <span>{isRestoring ? "恢复中..." : "恢复"}</span>
                      </button>

                      <button
                        onClick={() => handleDeleteBackup(b.id)}
                        disabled={isRestoring}
                        className="p-1 rounded hover:bg-rose-950/40 text-slate-500 hover:text-rose-400 transition-colors disabled:opacity-50"
                        title="删除该备份快照"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </main>
    );
  }

  // 模式 A: 当前项目看板
  if (!activeProject) {
    return (
      <main className="flex-1 bg-slate-950 flex flex-col items-center justify-center text-slate-500 text-sm select-none">
        <Folder className="w-12 h-12 mb-3 opacity-30" />
        <p>请在左侧选择或添加一个项目</p>
      </main>
    );
  }

  const mounts = projectDiagnostic?.mounts ?? [];
  const entryLinks = projectDiagnostic?.entry_links ?? [];
  const unmanaged = projectDiagnostic?.unmanaged_dirs ?? [];

  return (
    <main className="flex-1 bg-slate-950 flex flex-col h-full overflow-y-auto select-none p-6 space-y-6">
      {/* 项目头部信息 */}
      <div className="flex items-start justify-between border-b border-slate-800 pb-4">
        <div>
          <div className="flex items-center space-x-2.5">
            <h2 className="text-xl font-bold text-slate-100">{activeProject.name}</h2>
            <span className="text-xs px-2 py-0.5 rounded bg-slate-800 border border-slate-700 text-slate-400">
              已挂载 {mounts.length} 个
            </span>
          </div>
          <p className="text-xs text-slate-400 mt-1 font-mono">{activeProject.path}</p>
        </div>

        <div className="flex items-center space-x-2">
          {/* 挂载模式选择器 */}
          <div className="flex items-center space-x-1.5 bg-slate-900 border border-slate-700/80 rounded-md px-2.5 py-1">
            <span className="text-[11px] text-slate-400 font-medium">挂载模式:</span>
            <select
              value={mountMode}
              onChange={(e) => onMountModeChange?.(e.target.value as any)}
              className="bg-transparent text-xs text-teal-300 font-medium outline-none cursor-pointer pr-1"
              title="选择新挂载时所采用的模式"
            >
              <option value="copy" className="bg-slate-900 text-slate-200">
                Copy 副本 (支持 AGY)
              </option>
              <option value="junction" className="bg-slate-900 text-slate-200">
                Junction 联接点 (实时同步)
              </option>
              <option value="symlink" className="bg-slate-900 text-slate-200">
                Symlink 软链接 (需开发者模式)
              </option>
            </select>
          </div>

          {selectedSkillsCount > 0 && (
            <button
              onClick={onBatchMountSelected}
              className="inline-flex items-center space-x-1.5 px-3 py-1.5 text-xs font-semibold rounded-md bg-teal-600 hover:bg-teal-500 text-white shadow transition-colors"
            >
              <Plus className="w-4 h-4" />
              <span>批量挂载已选 ({selectedSkillsCount})</span>
            </button>
          )}

          <button
            onClick={() => handleOpenFolder(activeProject.path)}
            className="inline-flex items-center space-x-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
          >
            <ExternalLink className="w-3.5 h-3.5" />
            <span>打开项目目录</span>
          </button>
        </div>
      </div>

      {/* Agent 入口软链接看板 */}
      <div className="bg-slate-900 border border-slate-800 rounded-lg p-4">
        <div className="flex items-center justify-between mb-3">
          <div className="flex items-center space-x-2">
            <Link2 className="w-4 h-4 text-teal-400" />
            <h3 className="text-xs font-semibold text-slate-200">Agent 入口软链接管理</h3>
          </div>
          <span className="text-[11px] text-slate-500">
            实际安装目录：<code className="text-slate-400">.agents/skills</code>
          </span>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
          {/* Claude Code 入口 */}
          {(() => {
            const claudeLink = entryLinks.find((l) => l.link_path.includes(".claude"));
            const hasLink = claudeLink && claudeLink.status === "valid";
            return (
              <div className="bg-slate-850 border border-slate-700/60 rounded-lg p-3 flex items-center justify-between">
                <div>
                  <div className="flex items-center space-x-2">
                    <span className="text-xs font-medium text-slate-200">Claude Code 入口</span>
                    <code className="text-[10px] text-slate-400">.claude/skills</code>
                  </div>
                  <p className="text-[11px] text-slate-500 mt-0.5">指向 .agents/skills</p>
                </div>
                {hasLink ? (
                  <div className="flex items-center space-x-2">
                    <span className="text-[10px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2 py-0.5 rounded">
                      已建立
                    </span>
                    <button
                      onClick={() => onRemoveEntryLink(claudeLink.id, claudeLink.link_path)}
                      className="p-1 hover:text-rose-400 text-slate-500"
                      title="移除入口链接"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => onSetupEntryLink("claude_code", ".claude/skills")}
                    className="px-2.5 py-1 text-xs font-medium rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-teal-400 transition-colors"
                  >
                    创建链接
                  </button>
                )}
              </div>
            );
          })()}

          {/* Codex 入口 */}
          {(() => {
            const codexLink = entryLinks.find((l) => l.link_path.includes(".codex"));
            const hasLink = codexLink && codexLink.status === "valid";
            return (
              <div className="bg-slate-850 border border-slate-700/60 rounded-lg p-3 flex items-center justify-between">
                <div>
                  <div className="flex items-center space-x-2">
                    <span className="text-xs font-medium text-slate-200">Codex 入口</span>
                    <code className="text-[10px] text-slate-400">.codex/skills</code>
                  </div>
                  <p className="text-[11px] text-slate-500 mt-0.5">指向 .agents/skills</p>
                </div>
                {hasLink ? (
                  <div className="flex items-center space-x-2">
                    <span className="text-[10px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2 py-0.5 rounded">
                      已建立
                    </span>
                    <button
                      onClick={() => onRemoveEntryLink(codexLink.id, codexLink.link_path)}
                      className="p-1 hover:text-rose-400 text-slate-500"
                      title="移除入口链接"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ) : (
                  <button
                    onClick={() => onSetupEntryLink("codex", ".codex/skills")}
                    className="px-2.5 py-1 text-xs font-medium rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-teal-400 transition-colors"
                  >
                    创建链接
                  </button>
                )}
              </div>
            );
          })()}
        </div>
      </div>

      {/* 未托管对象警告 */}
      {unmanaged.length > 0 && (
        <div className="bg-amber-950/30 border border-amber-800/60 rounded-lg p-3.5">
          <div className="flex items-start space-x-2.5">
            <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
            <div>
              <h4 className="text-xs font-semibold text-amber-300">
                发现 {unmanaged.length} 个未托管实体目录
              </h4>
              <p className="text-[11px] text-amber-400/80 mt-0.5">
                位于 <code className="font-mono">.agents/skills/</code> 中，但未在本工具中登记：
                {unmanaged.join(", ")}。
              </p>
            </div>
          </div>
        </div>
      )}

      {/* AGY 兼容性提示（当存在非 Copy 模式的挂载时显示） */}
      {mounts.some((m) => m.mount_mode !== "copy") && (
        <div className="bg-blue-950/20 border border-blue-800/40 rounded-lg p-3 flex items-start space-x-2.5 text-xs text-blue-300">
          <HelpCircle className="w-4 h-4 text-blue-400 shrink-0 mt-0.5" />
          <div className="flex-1 leading-relaxed">
            <span className="font-semibold text-blue-200">AGY 兼容性提示：</span>
            <span>
              检测到当前项目存在 Junction/Symlink 挂载。由于 Antigravity CLI (AGY) 扫描时不穿透 Windows 软链接，如需在 AGY 中使用，可点击操作列中的「转为 Copy」一键转换为物理副本。
            </span>
          </div>
        </div>
      )}

      {/* 已挂载 Skill 表格 */}
      <div className="bg-slate-900 border border-slate-800 rounded-lg overflow-hidden">
        <div className="p-3.5 border-b border-slate-800 flex items-center justify-between">
          <h3 className="text-xs font-semibold text-slate-200">项目已挂载 Skill 清单 ({mounts.length})</h3>
        </div>

        {mounts.length === 0 ? (
          <div className="text-center py-12 text-slate-500 text-xs">
            当前项目暂未挂载任何 Skill，可从中间列表勾选并点击“批量挂载”
          </div>
        ) : (
          <table className="w-full text-left border-collapse">
            <thead>
              <tr className="border-b border-slate-800 bg-slate-850/50 text-[11px] text-slate-400">
                <th className="py-2 px-3 font-medium">Skill 名称</th>
                <th className="py-2 px-3 font-medium">模式</th>
                <th className="py-2 px-3 font-medium">状态</th>
                <th className="py-2 px-3 font-medium">挂载路径</th>
                <th className="py-2 px-3 font-medium text-right">操作</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60 text-xs">
              {mounts.map((mount) => (
                <tr key={mount.id} className="hover:bg-slate-800/30 transition-colors">
                  <td className="py-2.5 px-3 font-medium text-slate-200">{mount.skill_name}</td>
                  <td className="py-2.5 px-3">
                    <span
                      className={`px-1.5 py-0.5 rounded text-[10px] font-medium border ${
                        mount.mount_mode === "copy"
                          ? "bg-emerald-950/40 border-emerald-700/60 text-emerald-300"
                          : mount.mount_mode === "junction"
                          ? "bg-blue-950/40 border-blue-700/60 text-blue-300"
                          : "bg-purple-950/40 border-purple-700/60 text-purple-300"
                      }`}
                    >
                      {mount.mount_mode.toUpperCase()}
                    </span>
                  </td>
                  <td className="py-2.5 px-3">{renderStatusBadge(mount.status, mount.is_outdated)}</td>
                  <td className="py-2.5 px-3 text-slate-500 font-mono text-[11px] truncate max-w-[200px]" title={mount.link_path}>
                    {mount.link_path}
                  </td>
                  <td className="py-2.5 px-3 text-right space-x-1.5">
                    {mount.is_outdated && (
                      <button
                        onClick={() => onMountRepair(mount)}
                        className="inline-flex items-center space-x-1 px-2 py-1 text-[11px] rounded bg-amber-950 hover:bg-amber-900 border border-amber-600/80 text-amber-300 transition-colors"
                        title="中央仓库原件已更新，点击同步最新原件到此副本"
                      >
                        <RefreshCw className="w-3 h-3" />
                        <span>同步最新</span>
                      </button>
                    )}

                    {mount.status !== "NORMAL" && !mount.is_outdated && (
                      <button
                        onClick={() => onMountRepair(mount)}
                        className="inline-flex items-center space-x-1 px-2 py-1 text-[11px] rounded bg-teal-950 hover:bg-teal-900 border border-teal-800/60 text-teal-300 transition-colors"
                        title="修复此项挂载"
                      >
                        <Wrench className="w-3 h-3" />
                        <span>修复</span>
                      </button>
                    )}

                    {/* 模式一键转换 */}
                    {mount.mount_mode !== "copy" ? (
                      <button
                        onClick={() => onSwitchMountMode?.(mount, "copy")}
                        className="inline-flex items-center space-x-1 px-2 py-1 text-[11px] rounded bg-emerald-950/60 hover:bg-emerald-900 border border-emerald-800/60 text-emerald-300 transition-colors"
                        title="转换为物理副本（解决 AGY 无法识别的问题）"
                      >
                        <Repeat className="w-3 h-3" />
                        <span>转为 Copy</span>
                      </button>
                    ) : (
                      <button
                        onClick={() => onSwitchMountMode?.(mount, "junction")}
                        className="inline-flex items-center space-x-1 px-2 py-1 text-[11px] rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-300 transition-colors"
                        title="转换为 Junction 联接点（可实现免手动同步，但 AGY 无法识别）"
                      >
                        <Repeat className="w-3 h-3" />
                        <span>转为 Junction</span>
                      </button>
                    )}

                    <button
                      onClick={() => onUnmountSkill(mount)}
                      className="inline-flex items-center space-x-1 px-2 py-1 text-[11px] rounded bg-slate-800 hover:bg-rose-950 hover:text-rose-300 border border-slate-700 hover:border-rose-800/60 text-slate-400 transition-colors"
                      title="卸载此挂载"
                    >
                      <Trash2 className="w-3 h-3" />
                      <span>卸载</span>
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </main>
  );
};
