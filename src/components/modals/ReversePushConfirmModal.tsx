import React from "react";
import {
  AlertTriangle,
  ArrowUpCircle,
  FileCode,
  FilePlus2,
  FileText,
  FileX2,
  Loader2,
  ShieldCheck,
  X,
} from "lucide-react";
import { SkillDiffResult } from "../../types";

interface ReversePushConfirmModalProps {
  isOpen: boolean;
  diffResult: SkillDiffResult | null;
  projectName: string;
  onConfirm: (force: boolean) => void;
  onClose: () => void;
  isLoading?: boolean;
}

export const ReversePushConfirmModal: React.FC<ReversePushConfirmModalProps> = ({
  isOpen,
  diffResult,
  projectName,
  onConfirm,
  onClose,
  isLoading = false,
}) => {
  if (!isOpen || !diffResult) return null;

  const addedCount = diffResult.files.filter((f) => f.change_type === "ADDED").length;
  const modifiedCount = diffResult.files.filter((f) => f.change_type === "MODIFIED").length;
  const deletedCount = diffResult.files.filter((f) => f.change_type === "DELETED").length;

  return (
    <div className="fixed inset-0 z-50 bg-black/75 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-lg w-full p-6 shadow-2xl space-y-4 max-h-[90vh] flex flex-col">
        {/* 顶部标题 */}
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2.5 text-teal-400">
            <ArrowUpCircle className="w-5 h-5" />
            <div>
              <h3 className="text-sm font-semibold text-slate-100">反向更新中央仓库 Skill</h3>
              <p className="text-[11px] text-slate-400 mt-0.5">
                将项目 <code className="text-teal-300 font-mono">[{projectName}]</code> 中的改动同步至中央仓库
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            disabled={isLoading}
            className="text-slate-400 hover:text-slate-200 transition-colors disabled:opacity-50"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* 技能标识 */}
        <div className="flex items-center justify-between bg-slate-950 border border-slate-800 rounded-lg p-3">
          <div className="flex items-center space-x-2">
            <FileCode className="w-4 h-4 text-slate-400" />
            <span className="text-xs font-semibold text-slate-200">{diffResult.skill_name}</span>
          </div>
          <div className="flex items-center space-x-1.5 text-[10px]">
            {addedCount > 0 && (
              <span className="px-1.5 py-0.5 rounded bg-emerald-950/60 border border-emerald-700/60 text-emerald-300">
                +{addedCount} 新增
              </span>
            )}
            {modifiedCount > 0 && (
              <span className="px-1.5 py-0.5 rounded bg-blue-950/60 border border-blue-700/60 text-blue-300">
                ~{modifiedCount} 修改
              </span>
            )}
            {deletedCount > 0 && (
              <span className="px-1.5 py-0.5 rounded bg-rose-950/60 border border-rose-700/60 text-rose-300">
                -{deletedCount} 删除
              </span>
            )}
          </div>
        </div>

        {/* 双向冲突警告 */}
        {diffResult.has_conflict && (
          <div className="bg-amber-950/30 border border-amber-700/70 rounded-lg p-3.5 flex items-start space-x-3">
            <AlertTriangle className="w-4 h-4 text-amber-400 shrink-0 mt-0.5" />
            <div className="space-y-1 text-xs">
              <h4 className="font-semibold text-amber-200">检测到双向修改冲突</h4>
              <p className="text-[11px] text-amber-300/80 leading-relaxed">
                中央仓库原件在外部也被更新过。继续推送将使用当前项目中的副本
                <span className="font-semibold text-amber-100">强制覆盖</span> 中央仓库原件。
              </p>
            </div>
          </div>
        )}

        {/* 文件差异列表 */}
        <div className="flex-1 overflow-hidden flex flex-col space-y-1.5 min-h-[160px]">
          <div className="flex items-center justify-between text-xs text-slate-400 font-medium px-0.5">
            <span>文件变更清单 ({diffResult.files.length})</span>
            <span className="text-[10px] text-slate-500 font-mono">
              本地哈希: {diffResult.local_hash.slice(0, 8)}...
            </span>
          </div>

          <div className="flex-1 overflow-y-auto border border-slate-800 bg-slate-950/80 rounded-lg divide-y divide-slate-850 p-1 max-h-56">
            {diffResult.files.length === 0 ? (
              <div className="py-8 text-center text-slate-500 text-xs flex flex-col items-center justify-center space-y-1">
                <FileText className="w-5 h-5 opacity-40" />
                <span>两端文件内容完全一致，无需更新</span>
              </div>
            ) : (
              diffResult.files.map((file) => (
                <div
                  key={file.path}
                  className="flex items-center justify-between p-2 hover:bg-slate-900/60 transition-colors text-xs font-mono"
                >
                  <div className="flex items-center space-x-2 truncate">
                    {file.change_type === "ADDED" ? (
                      <FilePlus2 className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                    ) : file.change_type === "MODIFIED" ? (
                      <FileText className="w-3.5 h-3.5 text-blue-400 shrink-0" />
                    ) : (
                      <FileX2 className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                    )}
                    <span className="text-slate-300 truncate" title={file.path}>
                      {file.path}
                    </span>
                  </div>

                  <span
                    className={`text-[10px] font-semibold px-1.5 py-0.5 rounded uppercase ${
                      file.change_type === "ADDED"
                        ? "text-emerald-400 bg-emerald-500/10"
                        : file.change_type === "MODIFIED"
                        ? "text-blue-400 bg-blue-500/10"
                        : "text-rose-400 bg-rose-500/10"
                    }`}
                  >
                    {file.change_type}
                  </span>
                </div>
              ))
            )}
          </div>
        </div>

        {/* 安全快照提示 */}
        <div className="bg-slate-950/60 border border-slate-800 rounded-lg p-2.5 flex items-center space-x-2 text-[11px] text-slate-400">
          <ShieldCheck className="w-4 h-4 text-emerald-400 shrink-0" />
          <span>
            系统将在覆盖前自动为中央仓库原件创建安全快照备份，支持随时一键回退。
          </span>
        </div>

        {/* 底部按钮 */}
        <div className="flex items-center justify-end space-x-2 pt-2 border-t border-slate-800">
          <button
            onClick={onClose}
            disabled={isLoading}
            className="px-3.5 py-1.5 text-xs font-medium rounded-md bg-slate-800 hover:bg-slate-700 text-slate-300 transition-colors disabled:opacity-50"
          >
            取消
          </button>

          <button
            onClick={() => onConfirm(diffResult.has_conflict)}
            disabled={isLoading || diffResult.files.length === 0}
            className={`inline-flex items-center space-x-1.5 px-4 py-1.5 text-xs font-semibold rounded-md shadow transition-colors disabled:opacity-50 text-white ${
              diffResult.has_conflict
                ? "bg-amber-600 hover:bg-amber-500"
                : "bg-teal-600 hover:bg-teal-500"
            }`}
          >
            {isLoading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <ArrowUpCircle className="w-3.5 h-3.5" />
            )}
            <span>
              {isLoading
                ? "正在安全同步..."
                : diffResult.has_conflict
                ? "强制覆盖中央原件"
                : "确认反向更新"}
            </span>
          </button>
        </div>
      </div>
    </div>
  );
};
