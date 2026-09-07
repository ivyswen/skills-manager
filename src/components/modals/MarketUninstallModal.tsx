import React, { useState } from "react";
import { AlertTriangle, Trash2, X, CheckSquare, Square, Folder, Loader2 } from "lucide-react";
import { MarketSkillItem, MarketUninstallCheckResult } from "../../types";

interface MarketUninstallModalProps {
  skill: MarketSkillItem | null;
  checkResult: MarketUninstallCheckResult | null;
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (cascadeUnmount: boolean) => Promise<void>;
  loading: boolean;
}

export const MarketUninstallModal: React.FC<MarketUninstallModalProps> = ({
  skill,
  checkResult,
  isOpen,
  onClose,
  onConfirm,
  loading,
}) => {
  const [cascadeUnmount, setCascadeUnmount] = useState(true);

  if (!isOpen || !skill || !checkResult) return null;

  const hasMountedProjects = checkResult.mounted_projects.length > 0;

  const handleConfirm = async () => {
    await onConfirm(cascadeUnmount);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm p-4 animate-fade-in">
      <div className="bg-slate-900 border border-slate-700/80 rounded-xl shadow-2xl w-full max-w-md overflow-hidden flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-slate-800 bg-slate-900/80">
          <div className="flex items-center space-x-2 text-rose-400">
            <AlertTriangle className="w-5 h-5 shrink-0" />
            <h3 className="font-semibold text-slate-100 text-sm">
              卸载 Skill: {skill.name}
            </h3>
          </div>
          <button
            onClick={onClose}
            disabled={loading}
            className="text-slate-400 hover:text-slate-200 p-1 rounded-md transition-colors"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 text-sm text-slate-300 space-y-4">
          {hasMountedProjects ? (
            <>
              <div className="p-3 bg-amber-950/30 border border-amber-800/40 rounded-lg text-amber-200/90 text-xs leading-relaxed">
                <p className="font-semibold text-amber-400 mb-1">
                  ⚠️ 检测到关联项目引用
                </p>
                该技能当前已被以下 <span className="text-white font-medium">{checkResult.mounted_projects.length}</span> 个项目挂载：
              </div>

              {/* Project list */}
              <div className="max-h-36 overflow-y-auto bg-slate-950/60 border border-slate-800 rounded-lg p-2.5 space-y-1.5">
                {checkResult.mounted_projects.map((proj, idx) => (
                  <div key={idx} className="flex items-center gap-2 text-xs text-slate-300">
                    <Folder className="w-3.5 h-3.5 text-teal-400 shrink-0" />
                    <span className="truncate">{proj}</span>
                  </div>
                ))}
              </div>

              {/* Cascade Checkbox */}
              <div
                onClick={() => !loading && setCascadeUnmount(!cascadeUnmount)}
                className="flex items-start gap-2.5 p-3 rounded-lg bg-slate-800/60 border border-slate-700/80 cursor-pointer hover:bg-slate-800 transition-colors select-none"
              >
                {cascadeUnmount ? (
                  <CheckSquare className="w-4 h-4 text-rose-400 mt-0.5 shrink-0" />
                ) : (
                  <Square className="w-4 h-4 text-slate-400 mt-0.5 shrink-0" />
                )}
                <div className="text-xs">
                  <span className="font-medium text-slate-200">
                    同时级联解除各项目的软链接挂载并安全清理
                  </span>
                  <p className="text-slate-400 text-[11px] mt-0.5">
                    勾选后将自动卸载各项目中的软链接，防止遗留断链孤儿。
                  </p>
                </div>
              </div>
            </>
          ) : (
            <p className="text-slate-300 text-xs leading-relaxed">
              确定要从中央仓库中卸载技能 <strong className="text-slate-100 font-mono">"{skill.name}"</strong> 吗？
              <br />
              <span className="text-slate-400 text-[11px] mt-1.5 block">
                该技能目前未被任何项目引用，将直接从中央仓库删除对应物理文件夹及索引记录。
              </span>
            </p>
          )}
        </div>

        {/* Footer */}
        <div className="flex items-center justify-end gap-2 px-5 py-3.5 bg-slate-950/60 border-t border-slate-800">
          <button
            onClick={onClose}
            disabled={loading}
            className="px-3.5 py-1.5 text-xs font-medium text-slate-300 hover:text-white bg-slate-800 hover:bg-slate-700 rounded-lg border border-slate-700 transition-colors disabled:opacity-50"
          >
            取消
          </button>
          <button
            onClick={handleConfirm}
            disabled={loading || (hasMountedProjects && !cascadeUnmount)}
            className="inline-flex items-center gap-1.5 px-3.5 py-1.5 text-xs font-medium text-white bg-rose-600 hover:bg-rose-500 rounded-lg shadow-sm border border-rose-500/50 transition-colors disabled:opacity-50 disabled:pointer-events-none"
          >
            {loading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Trash2 className="w-3.5 h-3.5" />
            )}
            <span>{hasMountedProjects ? "确认级联卸载" : "确认卸载"}</span>
          </button>
        </div>
      </div>
    </div>
  );
};
