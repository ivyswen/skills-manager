import React from "react";
import {
  RefreshCw,
  GitPullRequest,
  FileDown,
  FileUp,
  History,
  FolderGit2,
  FolderPlus,
  ShieldAlert,
} from "lucide-react";
import { Repository } from "../../types";

interface TopToolbarProps {
  repository: Repository | null;
  onRefreshAll: () => void;
  onGitPull: () => void;
  onOpenLogs: () => void;
  onOpenExport: () => void;
  onOpenImport: () => void;
  onAddProject: () => void;
  onAddRepo: () => void;
  loading: boolean;
}

export const TopToolbar: React.FC<TopToolbarProps> = ({
  repository,
  onRefreshAll,
  onGitPull,
  onOpenLogs,
  onOpenExport,
  onOpenImport,
  onAddProject,
  onAddRepo,
  loading,
}) => {
  return (
    <header className="h-14 bg-slate-900 border-b border-slate-800 flex items-center justify-between px-4 select-none shrink-0">
      <div className="flex items-center space-x-3">
        <div className="flex items-center space-x-2.5">
          <img
            src="/app-icon.png"
            alt="Skills Manager"
            className="w-8 h-8 rounded-lg shadow-sm object-cover shrink-0"
          />
          <div>
            <h1 className="font-semibold text-slate-100 text-sm leading-tight">Skills 管理工具</h1>
            <span className="text-[11px] text-slate-400">Desktop Client v1.1</span>
          </div>
        </div>

        {repository ? (
          <div className="hidden md:flex items-center ml-4 pl-4 border-l border-slate-800 text-xs text-slate-400 space-x-2">
            <span className="inline-block w-2 h-2 rounded-full bg-emerald-500"></span>
            <span className="font-medium text-slate-300 truncate max-w-[200px]">{repository.name}</span>
            {repository.source_type === "git" && repository.current_branch && (
              <span className="px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700 text-[10px]">
                {repository.current_branch}
              </span>
            )}
          </div>
        ) : (
          <div className="hidden md:flex items-center ml-4 pl-4 border-l border-slate-800 text-xs text-amber-400 space-x-1">
            <ShieldAlert className="w-3.5 h-3.5" />
            <span>未配置中央仓库</span>
          </div>
        )}
      </div>

      <div className="flex items-center space-x-1.5">
        <button
          onClick={onRefreshAll}
          disabled={loading}
          className="inline-flex items-center space-x-1.5 px-2.5 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors disabled:opacity-50"
          title="刷新全部扫描与状态"
        >
          <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin text-teal-400" : ""}`} />
          <span>刷新</span>
        </button>

        {repository?.source_type === "git" && (
          <button
            onClick={onGitPull}
            disabled={loading}
            className="inline-flex items-center space-x-1.5 px-2.5 py-1.5 text-xs font-medium rounded-md bg-teal-950 hover:bg-teal-900 border border-teal-800/80 text-teal-300 transition-colors disabled:opacity-50"
            title="拉取 Git 中央仓库更新"
          >
            <GitPullRequest className="w-3.5 h-3.5" />
            <span>Git 更新</span>
          </button>
        )}

        <div className="h-4 w-px bg-slate-800 mx-1" />

        <button
          onClick={onAddProject}
          className="inline-flex items-center space-x-1.5 px-2.5 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
          title="添加本地项目"
        >
          <FolderPlus className="w-3.5 h-3.5 text-teal-400" />
          <span>添加项目</span>
        </button>

        <button
          onClick={onAddRepo}
          className="inline-flex items-center space-x-1.5 px-2.5 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
          title="设置/更换中央仓库"
        >
          <FolderGit2 className="w-3.5 h-3.5 text-indigo-400" />
          <span>中央仓库</span>
        </button>

        <div className="h-4 w-px bg-slate-800 mx-1" />

        <button
          onClick={onOpenExport}
          className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors"
          title="导出配置 JSON"
        >
          <FileDown className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenImport}
          className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors"
          title="导入配置 JSON"
        >
          <FileUp className="w-4 h-4" />
        </button>

        <button
          onClick={onOpenLogs}
          className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors"
          title="操作审计日志与回滚"
        >
          <History className="w-4 h-4" />
        </button>
      </div>
    </header>
  );
};
