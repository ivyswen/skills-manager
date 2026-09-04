import React, { useState, useMemo } from "react";
import {
  Folder,
  FolderGit,
  AlertCircle,
  FolderSync,
  Trash2,
  ChevronRight,
  Sparkles,
  ListFilter,
  FolderPlus,
  Check,
} from "lucide-react";
import { Repository, Project, ProjectDiagnostic } from "../../types";

type ProjectSortMode = "default" | "name_asc" | "name_desc" | "mounts_desc";

interface LeftSidebarProps {
  repository: Repository | null;
  projects: Project[];
  activeProjectId: string | null;
  diagnosticsMap: Record<string, ProjectDiagnostic>;
  onSelectProject: (projectId: string) => void;
  onRelocateProject: (project: Project) => void;
  onRemoveProject: (project: Project) => void;
  onOpenRepoSettings: () => void;
  onAddProject?: () => void;
}

export const LeftSidebar: React.FC<LeftSidebarProps> = ({
  repository,
  projects,
  activeProjectId,
  diagnosticsMap,
  onSelectProject,
  onRelocateProject,
  onRemoveProject,
  onOpenRepoSettings,
  onAddProject,
}) => {
  const [sortMode, setSortMode] = useState<ProjectSortMode>("default");
  const [sortMenuOpen, setSortMenuOpen] = useState(false);

  const sortedProjects = useMemo(() => {
    const list = [...projects];
    switch (sortMode) {
      case "name_asc":
        return list.sort((a, b) => a.name.localeCompare(b.name, "zh-CN"));
      case "name_desc":
        return list.sort((a, b) => b.name.localeCompare(a.name, "zh-CN"));
      case "mounts_desc":
        return list.sort((a, b) => {
          const countA = diagnosticsMap[a.id]?.mounts.length ?? 0;
          const countB = diagnosticsMap[b.id]?.mounts.length ?? 0;
          return countB - countA;
        });
      default:
        return list;
    }
  }, [projects, sortMode, diagnosticsMap]);
  return (
    <aside className="w-64 bg-slate-900 border-r border-slate-800 flex flex-col h-full shrink-0 select-none">
      {/* 中央仓库卡片 */}
      <div className="p-3 border-b border-slate-800">
        <div className="text-[11px] font-semibold text-slate-400 uppercase tracking-wider mb-2 flex items-center justify-between">
          <span>中央 Skill 来源</span>
          <button
            onClick={onOpenRepoSettings}
            className="text-teal-400 hover:text-teal-300 text-[11px] font-normal"
          >
            {repository ? "配置" : "设置"}
          </button>
        </div>

        {repository ? (
          <div className="bg-slate-800/60 border border-slate-700/60 rounded-lg p-2.5 hover:border-slate-600 transition-colors">
            <div className="flex items-center space-x-2">
              <FolderGit className="w-4 h-4 text-teal-400 shrink-0" />
              <span className="text-xs font-medium text-slate-200 truncate">{repository.name}</span>
            </div>
            <p className="text-[11px] text-slate-400 truncate mt-1" title={repository.path}>
              {repository.path}
            </p>
            <div className="flex items-center space-x-2 mt-2 text-[10px] text-slate-400">
              <span className="px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700">
                {repository.source_type.toUpperCase()}
              </span>
              {repository.current_branch && (
                <span className="px-1.5 py-0.5 rounded bg-slate-800 border border-slate-700 text-teal-300">
                  {repository.current_branch}
                </span>
              )}
            </div>
          </div>
        ) : (
          <div
            onClick={onOpenRepoSettings}
            className="border border-dashed border-slate-700 rounded-lg p-3 text-center cursor-pointer hover:border-teal-500/60 hover:bg-slate-800/40 transition-colors"
          >
            <Sparkles className="w-5 h-5 text-teal-400 mx-auto mb-1 opacity-80" />
            <p className="text-xs text-slate-300 font-medium">配置中央仓库</p>
            <p className="text-[10px] text-slate-500 mt-0.5">指定包含 skills/ 的本地或 Git 目录</p>
          </div>
        )}
      </div>

      {/* 项目列表 */}
      <div className="flex-1 overflow-y-auto p-2">
        <div className="text-[11px] font-semibold text-slate-400 uppercase tracking-wider px-2 py-1.5 flex items-center justify-between relative">
          <span>项目列表 ({projects.length})</span>
          <div className="flex items-center space-x-1">
            {/* 排序按钮 */}
            <div className="relative">
              <button
                onClick={() => setSortMenuOpen(!sortMenuOpen)}
                className={`p-1 rounded hover:bg-slate-800 transition-colors ${
                  sortMode !== "default" || sortMenuOpen
                    ? "text-teal-400 bg-slate-800/80"
                    : "text-slate-400 hover:text-slate-200"
                }`}
                title="项目排序"
              >
                <ListFilter className="w-4 h-4" />
              </button>

              {/* 排序菜单下拉 */}
              {sortMenuOpen && (
                <>
                  <div
                    className="fixed inset-0 z-20"
                    onClick={() => setSortMenuOpen(false)}
                  />
                  <div className="absolute right-0 top-full mt-1 w-36 bg-slate-900 border border-slate-700 rounded-lg shadow-2xl py-1.5 z-50 text-xs">
                    <button
                      onClick={() => {
                        setSortMode("default");
                        setSortMenuOpen(false);
                      }}
                      className={`w-full px-2.5 py-1.5 text-left flex items-center justify-between hover:bg-slate-800 transition-colors ${
                        sortMode === "default" ? "text-teal-400 font-medium" : "text-slate-300"
                      }`}
                    >
                      <span>默认顺序</span>
                      {sortMode === "default" && <Check className="w-3 h-3 text-teal-400" />}
                    </button>
                    <button
                      onClick={() => {
                        setSortMode("name_asc");
                        setSortMenuOpen(false);
                      }}
                      className={`w-full px-2.5 py-1.5 text-left flex items-center justify-between hover:bg-slate-800 transition-colors ${
                        sortMode === "name_asc" ? "text-teal-400 font-medium" : "text-slate-300"
                      }`}
                    >
                      <span>名称 (A → Z)</span>
                      {sortMode === "name_asc" && <Check className="w-3 h-3 text-teal-400" />}
                    </button>
                    <button
                      onClick={() => {
                        setSortMode("name_desc");
                        setSortMenuOpen(false);
                      }}
                      className={`w-full px-2.5 py-1.5 text-left flex items-center justify-between hover:bg-slate-800 transition-colors ${
                        sortMode === "name_desc" ? "text-teal-400 font-medium" : "text-slate-300"
                      }`}
                    >
                      <span>名称 (Z → A)</span>
                      {sortMode === "name_desc" && <Check className="w-3 h-3 text-teal-400" />}
                    </button>
                    <button
                      onClick={() => {
                        setSortMode("mounts_desc");
                        setSortMenuOpen(false);
                      }}
                      className={`w-full px-2.5 py-1.5 text-left flex items-center justify-between hover:bg-slate-800 transition-colors ${
                        sortMode === "mounts_desc" ? "text-teal-400 font-medium" : "text-slate-300"
                      }`}
                    >
                      <span>挂载数量 (多到少)</span>
                      {sortMode === "mounts_desc" && <Check className="w-3 h-3 text-teal-400" />}
                    </button>
                  </div>
                </>
              )}
            </div>

            {/* 新增项目按钮 */}
            {onAddProject && (
              <button
                onClick={onAddProject}
                className="p-1 rounded text-slate-400 hover:text-slate-200 hover:bg-slate-800 transition-colors"
                title="添加新项目"
              >
                <FolderPlus className="w-4 h-4" />
              </button>
            )}
          </div>
        </div>

        {sortedProjects.length === 0 ? (
          <div className="text-center py-8 px-4 text-slate-500 text-xs">
            暂无项目，请点击上方“添加项目”
          </div>
        ) : (
          <div className="space-y-1">
            {sortedProjects.map((project) => {
              const diag = diagnosticsMap[project.id];
              const isActive = activeProjectId === project.id;
              const mountsCount = diag?.mounts.length ?? 0;
              const anomaliesCount =
                diag?.mounts.filter(
                  (m) =>
                    m.status === "BROKEN" ||
                    m.status === "CONFLICT" ||
                    m.status === "WRONG_TARGET" ||
                    m.status === "PERMISSION_DENIED"
                ).length ?? 0;
              const isInvalidPath = diag ? !diag.is_valid : false;

              return (
                <div
                  key={project.id}
                  onClick={() => onSelectProject(project.id)}
                  className={`group relative rounded-lg p-2.5 cursor-pointer transition-all flex items-center justify-between border ${
                    isActive
                      ? "bg-teal-500/10 border-teal-500/40 text-slate-100"
                      : "bg-slate-850 hover:bg-slate-800/60 border-transparent text-slate-300"
                  }`}
                >
                  <div className="flex items-center space-x-2.5 min-w-0 flex-1 mr-2">
                    <Folder
                      className={`w-4 h-4 shrink-0 ${
                        isInvalidPath
                          ? "text-rose-400"
                          : isActive
                          ? "text-teal-400"
                          : "text-slate-400"
                      }`}
                    />
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center space-x-1.5">
                        <span className="text-xs font-medium truncate">{project.name}</span>
                        {isInvalidPath && (
                          <span className="text-[10px] text-rose-400 font-semibold px-1 rounded bg-rose-950/60 border border-rose-800/50">
                            路径失效
                          </span>
                        )}
                      </div>
                      <p className="text-[10px] text-slate-500 truncate" title={project.path}>
                        {project.path}
                      </p>
                    </div>
                  </div>

                  {/* 状态徽标与快捷按钮 */}
                  <div className="flex items-center space-x-1.5 shrink-0">
                    {anomaliesCount > 0 ? (
                      <span className="inline-flex items-center space-x-0.5 px-1.5 py-0.5 rounded-full text-[10px] font-semibold bg-rose-500/20 border border-rose-500/40 text-rose-300">
                        <AlertCircle className="w-2.5 h-2.5" />
                        <span>{anomaliesCount}</span>
                      </span>
                    ) : (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-slate-800 text-slate-400 border border-slate-700/60">
                        {mountsCount}
                      </span>
                    )}

                    {isInvalidPath && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onRelocateProject(project);
                        }}
                        className="p-1 hover:bg-slate-700 rounded text-amber-400"
                        title="项目目录已移动，点击重定位"
                      >
                        <FolderSync className="w-3.5 h-3.5" />
                      </button>
                    )}

                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onRemoveProject(project);
                      }}
                      className="opacity-0 group-hover:opacity-100 p-1 hover:bg-slate-700 hover:text-rose-400 rounded text-slate-500 transition-opacity"
                      title="移除项目"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>

                    <ChevronRight
                      className={`w-3 h-3 text-slate-500 transition-transform ${
                        isActive ? "rotate-90 text-teal-400" : ""
                      }`}
                    />
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </aside>
  );
};
