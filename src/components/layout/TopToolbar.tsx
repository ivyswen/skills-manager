import React from "react";
import {
  RefreshCw,
  GitPullRequest,
  FileDown,
  FileUp,
  History,
  FolderPlus,
  ShieldAlert,
  Search,
  Settings,
  ArrowLeft,
  Loader2,
} from "lucide-react";
import { Repository } from "../../types";

interface TopToolbarProps {
  repository: Repository | null;
  currentView: "repo" | "market";
  onViewChange: (view: "repo" | "market") => void;
  marketSearchQuery: string;
  onMarketSearchChange: (query: string) => void;
  onMarketSearchSubmit: () => void;
  marketSearching?: boolean;
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
  currentView,
  onViewChange,
  marketSearchQuery,
  onMarketSearchChange,
  onMarketSearchSubmit,
  marketSearching = false,
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
    <header className="bg-[#121316] border-b border-slate-800/80 flex flex-col select-none shrink-0">
      {/* Row 1: System Title & Action Buttons */}
      <div className="h-12 flex items-center justify-between px-4 border-b border-slate-800/40">
        <div className="flex items-center space-x-3">
          {currentView === "market" ? (
            <button
              onClick={() => onViewChange("repo")}
              className="inline-flex items-center gap-1.5 text-slate-300 hover:text-white transition-colors group text-sm font-medium"
              title="返回仓库视图"
            >
              <ArrowLeft className="w-4 h-4 group-hover:-translate-x-0.5 transition-transform" />
              <span>Skills 管理</span>
            </button>
          ) : (
            <>
              <div className="flex items-center space-x-2">
                <img
                  src="/app-icon.png"
                  alt="Skills Manager"
                  className="w-6 h-6 rounded shadow-sm object-cover shrink-0"
                />
                <h1 className="font-semibold text-slate-100 text-sm leading-tight">
                  Skills 管理
                </h1>
              </div>

              {repository ? (
                <div className="hidden md:flex items-center ml-3 pl-3 border-l border-slate-800 text-xs text-slate-400 space-x-2">
                  <span className="inline-block w-2 h-2 rounded-full bg-emerald-500"></span>
                  <span className="font-medium text-slate-300 truncate max-w-[200px]">
                    {repository.name}
                  </span>
                  {repository.source_type === "git" && repository.current_branch && (
                    <span className="px-1.5 py-0.2 rounded bg-slate-800 text-slate-400 border border-slate-700 text-[10px]">
                      {repository.current_branch}
                    </span>
                  )}
                </div>
              ) : (
                <div className="hidden md:flex items-center ml-3 pl-3 border-l border-slate-800 text-xs text-amber-400 space-x-1">
                  <ShieldAlert className="w-3.5 h-3.5" />
                  <span>未配置中央仓库</span>
                </div>
              )}
            </>
          )}
        </div>

        {/* Top Right Utilities */}
        <div className="flex items-center space-x-1.5">
          {currentView === "market" ? (
            /* 仓库管理 (Exact match with reference screenshot) */
            <button
              onClick={onAddRepo}
              className="inline-flex items-center space-x-1.5 px-3 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
              title="设置/管理中央仓库"
            >
              <Settings className="w-3.5 h-3.5 text-slate-300" />
              <span>仓库管理</span>
            </button>
          ) : (
            <>
              <button
                onClick={onRefreshAll}
                disabled={loading}
                className="inline-flex items-center space-x-1 px-2.5 py-1 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors disabled:opacity-50"
                title="刷新全部扫描与状态"
              >
                <RefreshCw className={`w-3.5 h-3.5 ${loading ? "animate-spin text-teal-400" : ""}`} />
                <span>刷新</span>
              </button>

              {repository?.source_type === "git" && (
                <button
                  onClick={onGitPull}
                  disabled={loading}
                  className="inline-flex items-center space-x-1 px-2.5 py-1 text-xs font-medium rounded-md bg-teal-950 hover:bg-teal-900 border border-teal-800/80 text-teal-300 transition-colors disabled:opacity-50"
                  title="拉取 Git 中央仓库更新"
                >
                  <GitPullRequest className="w-3.5 h-3.5" />
                  <span>Git 更新</span>
                </button>
              )}

              <button
                onClick={onAddProject}
                className="inline-flex items-center space-x-1 px-2.5 py-1 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
                title="添加本地项目"
              >
                <FolderPlus className="w-3.5 h-3.5 text-teal-400" />
                <span>添加项目</span>
              </button>

              <button
                onClick={onAddRepo}
                className="inline-flex items-center space-x-1.5 px-2.5 py-1 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200 transition-colors"
                title="设置/管理中央仓库"
              >
                <Settings className="w-3.5 h-3.5 text-slate-300" />
                <span>仓库管理</span>
              </button>

              <div className="h-4 w-px bg-slate-800 mx-1" />

              <button
                onClick={onOpenExport}
                className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors"
                title="导出配置 JSON"
              >
                <FileDown className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={onOpenImport}
                className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors"
                title="导入配置 JSON"
              >
                <FileUp className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={onOpenLogs}
                className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-md transition-colors"
                title="操作审计日志与回滚"
              >
                <History className="w-3.5 h-3.5" />
              </button>
            </>
          )}
        </div>
      </div>

      {/* Row 2: Pill Switcher & Search Bar (100% matched with screenshot) */}
      <div className="h-12 px-4 flex items-center gap-3">
        {/* Pill Switcher: [ 仓库 | skills.sh ] */}
        <div className="bg-[#191a20] p-0.5 rounded-full border border-slate-800 flex items-center shrink-0">
          <button
            onClick={() => onViewChange("repo")}
            className={`px-3.5 py-1 text-xs rounded-full transition-all duration-150 font-medium ${
              currentView === "repo"
                ? "bg-blue-600 text-white shadow-sm"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            仓库
          </button>
          <button
            onClick={() => onViewChange("market")}
            className={`px-3.5 py-1 text-xs rounded-full transition-all duration-150 font-medium ${
              currentView === "market"
                ? "bg-blue-600 text-white shadow-sm"
                : "text-slate-400 hover:text-slate-200"
            }`}
          >
            skills.sh
          </button>
        </div>

        {/* Search Input Field */}
        <div className="flex-1 relative flex items-center">
          <Search className="w-4 h-4 text-slate-400 absolute left-3 pointer-events-none" />
          <input
            type="text"
            value={marketSearchQuery}
            onChange={(e) => onMarketSearchChange(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                if (currentView !== "market") {
                  onViewChange("market");
                }
                onMarketSearchSubmit();
              }
            }}
            placeholder={
              currentView === "market"
                ? "搜索技能名称或关键词，例如 ask, git, task..."
                : "在 skills.sh 市场中发现开源技能..."
            }
            className="w-full bg-[#18191f] border border-slate-800 hover:border-slate-700 focus:border-blue-500 focus:bg-[#1b1d24] text-slate-100 placeholder-slate-500 text-xs rounded-lg pl-9 pr-4 py-1.5 outline-none transition-all"
          />
        </div>

        {/* 搜索技能 Button */}
        <button
          onClick={() => {
            if (currentView !== "market") {
              onViewChange("market");
            }
            onMarketSearchSubmit();
          }}
          disabled={marketSearching}
          className="inline-flex items-center gap-1.5 px-4 py-1.5 text-xs font-medium text-white bg-blue-600 hover:bg-blue-500 rounded-lg shadow-sm border border-blue-500/50 transition-colors shrink-0 disabled:opacity-50"
        >
          {marketSearching ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <Search className="w-3.5 h-3.5" />
          )}
          <span>搜索技能</span>
        </button>
      </div>
    </header>
  );
};
