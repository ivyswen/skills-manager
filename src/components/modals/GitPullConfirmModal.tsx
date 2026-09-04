import React, { useState } from "react";
import { AlertCircle, GitPullRequest, Trash2, X } from "lucide-react";
import { AffectedSkill } from "../../types";

interface GitPullConfirmModalProps {
  isOpen: boolean;
  isDirtyWarning: boolean;
  dirtyFiles: string[];
  affectedSkills: AffectedSkill[];
  onConfirmCleanup: (selectedSkillNames: string[]) => void;
  onProceedPullAnyway?: () => void;
  onClose: () => void;
}

export const GitPullConfirmModal: React.FC<GitPullConfirmModalProps> = ({
  isOpen,
  isDirtyWarning,
  dirtyFiles,
  affectedSkills,
  onConfirmCleanup,
  onClose,
}) => {
  const [selectedToClean, setSelectedToClean] = useState<Set<string>>(
    new Set(affectedSkills.filter((s) => s.change_type === "deleted").map((s) => s.name))
  );

  if (!isOpen) return null;

  const toggleSelect = (name: string) => {
    const next = new Set(selectedToClean);
    if (next.has(name)) {
      next.delete(name);
    } else {
      next.add(name);
    }
    setSelectedToClean(next);
  };

  // 1. 脏工作区警告
  if (isDirtyWarning) {
    return (
      <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
        <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-lg w-full p-6 shadow-2xl space-y-4">
          <div className="flex items-start justify-between">
            <div className="flex items-center space-x-2 text-rose-400">
              <AlertCircle className="w-5 h-5" />
              <h3 className="text-sm font-semibold text-slate-100">中央仓库工作区存在未提交修改</h3>
            </div>
            <button onClick={onClose} className="text-slate-400 hover:text-slate-200">
              <X className="w-4 h-4" />
            </button>
          </div>

          <p className="text-xs text-slate-300 leading-relaxed">
            为防止覆盖或破坏您本地的未提交修改，Git 更新操作已被安全拦截。请在外部 Git 工具中提交或 stash 工作区后重试。
          </p>

          <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 max-h-40 overflow-y-auto space-y-1">
            <div className="text-[11px] font-semibold text-slate-400 mb-1">未提交修改文件：</div>
            {dirtyFiles.map((file, i) => (
              <div key={i} className="text-[11px] font-mono text-rose-300">
                {file}
              </div>
            ))}
          </div>

          <div className="flex justify-end pt-2">
            <button
              onClick={onClose}
              className="px-4 py-2 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 border border-slate-700 text-slate-200"
            >
              我知道了，取消更新
            </button>
          </div>
        </div>
      </div>
    );
  }

  // 2. 远端变更受影响项目批量复核清单
  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-lg w-full p-6 shadow-2xl space-y-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 text-teal-400">
            <GitPullRequest className="w-5 h-5" />
            <h3 className="text-sm font-semibold text-slate-100">Git 拉取完成：Skill 变动复核</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <p className="text-xs text-slate-300 leading-relaxed">
          中央仓库原件发生变动。以下被删除或重命名的 Skill 关联了项目挂载。系统严禁隐式静默删除，请勾选需要清理的失效挂载：
        </p>

        <div className="bg-slate-950 border border-slate-800 rounded-lg max-h-60 overflow-y-auto divide-y divide-slate-800">
          {affectedSkills.map((item) => (
            <div
              key={item.name}
              onClick={() => toggleSelect(item.name)}
              className="p-3 flex items-center justify-between cursor-pointer hover:bg-slate-900/60 transition-colors"
            >
              <div className="flex items-center space-x-2.5">
                <input
                  type="checkbox"
                  checked={selectedToClean.has(item.name)}
                  onChange={() => {}}
                  className="rounded border-slate-700 text-teal-500 focus:ring-teal-500"
                />
                <div>
                  <div className="text-xs font-medium text-slate-200">{item.name}</div>
                  <div className="text-[10px] text-slate-500">
                    影响 {item.affected_project_ids.length} 个项目的挂载
                  </div>
                </div>
              </div>

              <span
                className={`text-[10px] px-2 py-0.5 rounded font-medium ${
                  item.change_type === "deleted"
                    ? "bg-rose-500/10 text-rose-400 border border-rose-500/20"
                    : "bg-amber-500/10 text-amber-400 border border-amber-500/20"
                }`}
              >
                {item.change_type === "deleted" ? "原件已删除" : "原件已重命名"}
              </span>
            </div>
          ))}
        </div>

        <div className="flex items-center justify-end space-x-2 pt-2">
          <button
            onClick={onClose}
            className="px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200"
          >
            暂不处理
          </button>
          <button
            onClick={() => onConfirmCleanup(Array.from(selectedToClean))}
            className="inline-flex items-center space-x-1.5 px-4 py-1.5 text-xs font-semibold rounded-md bg-rose-600 hover:bg-rose-500 text-white transition-colors"
          >
            <Trash2 className="w-3.5 h-3.5" />
            <span>清理勾选项 ({selectedToClean.size})</span>
          </button>
        </div>
      </div>
    </div>
  );
};
