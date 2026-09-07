import React, { useState, useMemo } from "react";
import {
  Search,
  CheckSquare,
  Square,
  FileCode2,
  Type,
  PlusCircle,
  Check,
  ArrowUpCircle,
  RefreshCw,
  Loader2,
  GitFork,
} from "lucide-react";
import { Skill, Mount, Project } from "../../types";

interface MiddleSkillsProps {
  skills: Skill[];
  activeProject: Project | null;
  projectMounts: Mount[];
  selectedSkillIds: Set<string>;
  onToggleSkillSelect: (skillId: string) => void;
  onSelectAllSkills: (skillIds: string[]) => void;
  onClearSkillSelect: () => void;
  onSelectSkillDetail: (skill: Skill) => void;
  onQuickMount: (skill: Skill) => void;
  activeSkillId?: string | null;
  onCheckUpdates?: () => void;
  isCheckingUpdates?: boolean;
  onUpdateSkill?: (skill: Skill) => void;
  onBatchUpdate?: (skillNames?: string[]) => void;
  updatingSkillNames?: Set<string>;
  isBatchUpdating?: boolean;
}

export const MiddleSkills: React.FC<MiddleSkillsProps> = ({
  skills,
  activeProject,
  projectMounts,
  selectedSkillIds,
  onToggleSkillSelect,
  onSelectAllSkills,
  onClearSkillSelect,
  onSelectSkillDetail,
  onQuickMount,
  activeSkillId,
  onCheckUpdates,
  isCheckingUpdates = false,
  onUpdateSkill,
  onBatchUpdate,
  updatingSkillNames = new Set(),
  isBatchUpdating = false,
}) => {
  const [searchQuery, setSearchQuery] = useState("");
  const [filterMode, setFilterMode] = useState<"all" | "mounted" | "unmounted" | "updatable">("all");

  const updatableSkills = useMemo(() => {
    return skills.filter((s) => s.has_update);
  }, [skills]);

  const mountMap = useMemo(() => {
    const map = new Map<string, Mount>();
    for (const m of projectMounts) {
      map.set(m.skill_id, m);
      map.set(m.skill_name, m);
    }
    return map;
  }, [projectMounts]);

  const filteredSkills = useMemo(() => {
    return skills.filter((skill) => {
      const matchQuery =
        skill.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
        skill.description.toLowerCase().includes(searchQuery.toLowerCase()) ||
        (skill.source && skill.source.toLowerCase().includes(searchQuery.toLowerCase()));
      if (!matchQuery) return false;

      const isMounted = mountMap.has(skill.id) || mountMap.has(skill.name);
      if (filterMode === "mounted") return isMounted;
      if (filterMode === "unmounted") return !isMounted;
      if (filterMode === "updatable") return !!skill.has_update;
      return true;
    });
  }, [skills, searchQuery, filterMode, mountMap]);

  const allFilteredSelected =
    filteredSkills.length > 0 &&
    filteredSkills.every((s) => selectedSkillIds.has(s.id));

  const handleSelectAll = () => {
    if (allFilteredSelected) {
      onClearSkillSelect();
    } else {
      onSelectAllSkills(filteredSkills.map((s) => s.id));
    }
  };

  return (
    <section className="w-80 lg:w-96 bg-slate-900 border-r border-slate-800 flex flex-col h-full shrink-0 select-none">
      {/* 搜索与过滤工具栏 */}
      <div className="p-3 border-b border-slate-800 space-y-2.5">
        <div className="relative">
          <Search className="w-4 h-4 text-slate-500 absolute left-2.5 top-2.5" />
          <input
            type="text"
            placeholder="搜索 Skill 名称或描述..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="w-full pl-8 pr-3 py-1.5 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 placeholder-slate-500 focus:outline-none focus:border-teal-500 focus:ring-1 focus:ring-teal-500 transition-colors"
          />
        </div>

        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-1 flex-wrap gap-y-1">
            <button
              onClick={() => setFilterMode("all")}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                filterMode === "all"
                  ? "bg-teal-500/20 text-teal-300 border border-teal-500/40"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              全部 ({skills.length})
            </button>
            <button
              onClick={() => setFilterMode("mounted")}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                filterMode === "mounted"
                  ? "bg-teal-500/20 text-teal-300 border border-teal-500/40"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              已挂载
            </button>
            <button
              onClick={() => setFilterMode("unmounted")}
              className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors ${
                filterMode === "unmounted"
                  ? "bg-teal-500/20 text-teal-300 border border-teal-500/40"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              未挂载
            </button>
            {updatableSkills.length > 0 && (
              <button
                onClick={() => setFilterMode("updatable")}
                className={`px-2 py-0.5 rounded text-[11px] font-medium transition-colors flex items-center gap-1 ${
                  filterMode === "updatable"
                    ? "bg-blue-500/20 text-blue-300 border border-blue-500/40"
                    : "text-blue-400/90 hover:text-blue-300"
                }`}
              >
                <ArrowUpCircle className="w-3 h-3 text-blue-400" />
                <span>可更新 ({updatableSkills.length})</span>
              </button>
            )}
          </div>

          <button
            onClick={handleSelectAll}
            className="text-[11px] text-slate-400 hover:text-teal-400 flex items-center space-x-1 shrink-0"
          >
            {allFilteredSelected ? (
              <>
                <CheckSquare className="w-3.5 h-3.5 text-teal-400" />
                <span>全选</span>
              </>
            ) : (
              <>
                <Square className="w-3.5 h-3.5" />
                <span>全选</span>
              </>
            )}
          </button>
        </div>

        {/* 技能更新操作栏 */}
        <div className="flex items-center justify-between pt-1 border-t border-slate-800/60 text-xs">
          <button
            onClick={onCheckUpdates}
            disabled={isCheckingUpdates || isBatchUpdating}
            className="inline-flex items-center space-x-1.5 px-2 py-1 text-[11px] font-medium rounded bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-300 transition-colors disabled:opacity-50"
            title="在线检查技能的 GitHub 上游是否有新版本"
          >
            <RefreshCw className={`w-3 h-3 ${isCheckingUpdates ? "animate-spin text-blue-400" : "text-slate-400"}`} />
            <span>{isCheckingUpdates ? "检查更新中..." : "检查更新"}</span>
          </button>

          {updatableSkills.length > 0 && (
            <button
              onClick={() => onBatchUpdate?.()}
              disabled={isCheckingUpdates || isBatchUpdating}
              className="inline-flex items-center space-x-1 px-2.5 py-1 text-[11px] font-semibold rounded bg-blue-600 hover:bg-blue-500 text-white shadow-sm transition-colors disabled:opacity-50"
              title="一键将所有可更新的技能自动安全备份并更新至最新版本"
            >
              {isBatchUpdating ? (
                <Loader2 className="w-3 h-3 animate-spin" />
              ) : (
                <ArrowUpCircle className="w-3 h-3" />
              )}
              <span>全部更新 ({updatableSkills.length})</span>
            </button>
          )}
        </div>
      </div>

      {/* Skill 卡片流列表 */}
      <div className="flex-1 overflow-y-auto p-2 space-y-2">
        {filteredSkills.length === 0 ? (
          <div className="text-center py-12 px-4 text-slate-500 text-xs">
            未检索到匹配的 Skill 原件
          </div>
        ) : (
          filteredSkills.map((skill) => {
            const isChecked = selectedSkillIds.has(skill.id);
            const isActive = activeSkillId === skill.id;
            const isSelected = isChecked || isActive;
            const mount = mountMap.get(skill.id) || mountMap.get(skill.name);
            const isMounted = !!mount;
            const isUpdatingThisSkill = updatingSkillNames.has(skill.name);

            return (
              <div
                key={skill.id}
                onClick={() => onSelectSkillDetail(skill)}
                className={`group border rounded-lg p-3 cursor-pointer transition-all hover:border-slate-600 ${
                  isSelected
                    ? "bg-teal-950/30 border-teal-500/60 ring-1 ring-teal-500/30"
                    : skill.has_update
                    ? "bg-blue-950/20 border-blue-800/40 hover:bg-blue-950/30"
                    : isMounted
                    ? "bg-slate-800/40 border-slate-700/50"
                    : "bg-slate-800/20 border-slate-800 hover:bg-slate-800/40"
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex items-center space-x-2">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onToggleSkillSelect(skill.id);
                      }}
                      className="text-slate-400 hover:text-teal-400"
                    >
                      {isChecked ? (
                        <CheckSquare className="w-4 h-4 text-teal-400" />
                      ) : (
                        <Square className="w-4 h-4" />
                      )}
                    </button>
                    <h3 className="text-xs font-semibold text-slate-200 group-hover:text-teal-300 transition-colors">
                      {skill.name}
                    </h3>
                  </div>

                  <div className="flex items-center space-x-1">
                    {skill.has_update && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-blue-500/15 border border-blue-500/30 text-blue-400 flex items-center space-x-0.5 font-medium">
                        <ArrowUpCircle className="w-2.5 h-2.5" />
                        <span>可更新</span>
                      </span>
                    )}
                    {skill.metadata_status === "incomplete" && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-500/10 border border-amber-500/30 text-amber-400">
                        元数据不完整
                      </span>
                    )}
                    {isMounted && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-emerald-500/10 border border-emerald-500/30 text-emerald-400 flex items-center space-x-0.5">
                        <Check className="w-2.5 h-2.5" />
                        <span>已挂载</span>
                      </span>
                    )}
                  </div>
                </div>

                {skill.source && (
                  <div className="flex items-center gap-1 text-[10px] text-slate-500 mt-1">
                    <GitFork className="w-2.5 h-2.5 text-slate-500" />
                    <span className="truncate max-w-[200px]" title={skill.source}>
                      {skill.source}
                    </span>
                  </div>
                )}

                <p className="text-[11px] text-slate-400 line-clamp-2 mt-1.5 leading-relaxed">
                  {skill.description || "暂无描述"}
                </p>

                <div className="flex items-center justify-between mt-2.5 pt-2 border-t border-slate-800/60 text-[10px] text-slate-500">
                  <div className="flex items-center space-x-2.5">
                    <span className="flex items-center space-x-1">
                      <FileCode2 className="w-3 h-3 text-slate-400" />
                      <span>{skill.file_count} 文件</span>
                    </span>
                    <span className="flex items-center space-x-1">
                      <Type className="w-3 h-3 text-slate-400" />
                      <span>{skill.char_count} 字符</span>
                    </span>
                  </div>

                  <div className="flex items-center space-x-2">
                    {skill.has_update && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onUpdateSkill?.(skill);
                        }}
                        disabled={isUpdatingThisSkill || isBatchUpdating}
                        className="flex items-center space-x-1 px-1.5 py-0.5 rounded bg-blue-500/20 hover:bg-blue-500/30 border border-blue-500/40 text-blue-300 font-medium transition-colors disabled:opacity-50"
                        title="安全备份并升级此技能"
                      >
                        {isUpdatingThisSkill ? (
                          <Loader2 className="w-3 h-3 animate-spin" />
                        ) : (
                          <ArrowUpCircle className="w-3 h-3" />
                        )}
                        <span>{isUpdatingThisSkill ? "更新中" : "更新"}</span>
                      </button>
                    )}

                    {activeProject && !isMounted && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          if (!selectedSkillIds.has(skill.id)) {
                            onToggleSkillSelect(skill.id);
                          }
                          onQuickMount(skill);
                        }}
                        className="opacity-0 group-hover:opacity-100 flex items-center space-x-1 text-teal-400 hover:text-teal-300 font-medium transition-opacity"
                        title={`快速挂载至当前项目: ${activeProject.name}`}
                      >
                        <PlusCircle className="w-3 h-3" />
                        <span>挂载</span>
                      </button>
                    )}
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </section>
  );
};
