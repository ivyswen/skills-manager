import React from "react";
import { AlertTriangle, Archive, SkipForward, X } from "lucide-react";
import { ConflictStrategy } from "../../types";

interface ConflictDialogProps {
  isOpen: boolean;
  skillName: string;
  targetPath: string;
  onConfirm: (strategy: ConflictStrategy) => void;
  onClose: () => void;
}

export const ConflictDialog: React.FC<ConflictDialogProps> = ({
  isOpen,
  skillName,
  targetPath,
  onConfirm,
  onClose,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-md w-full p-6 shadow-2xl space-y-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 text-amber-400">
            <AlertTriangle className="w-5 h-5" />
            <h3 className="text-sm font-semibold text-slate-100">挂载目标路径已存在</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <p className="text-xs text-slate-300 leading-relaxed">
          目标项目中的 <code className="font-mono text-teal-300">{skillName}</code> 已存在普通文件或目录：
        </p>

        <div className="bg-slate-950 border border-slate-800 rounded p-2 text-[11px] font-mono text-slate-400 break-all">
          {targetPath}
        </div>

        <p className="text-[11px] text-slate-400 leading-relaxed">
          为防止误删用户原有代码，系统严禁隐式静默覆盖。请选择处理策略：
        </p>

        <div className="space-y-2 pt-1">
          <button
            onClick={() => onConfirm("backup_and_replace")}
            className="w-full flex items-center justify-between p-3 rounded-lg bg-teal-950 hover:bg-teal-900 border border-teal-800/80 text-left transition-colors group"
          >
            <div>
              <div className="text-xs font-semibold text-teal-300">备份后替换 (推荐)</div>
              <div className="text-[10px] text-teal-400/80 mt-0.5">
                将原对象安全移至 .backup/ 目录并建立新链接
              </div>
            </div>
            <Archive className="w-4 h-4 text-teal-400 group-hover:scale-110 transition-transform" />
          </button>

          <button
            onClick={() => onConfirm("skip")}
            className="w-full flex items-center justify-between p-3 rounded-lg bg-slate-800 hover:bg-slate-750 border border-slate-700 text-left transition-colors group"
          >
            <div>
              <div className="text-xs font-medium text-slate-200">跳过该项</div>
              <div className="text-[10px] text-slate-400 mt-0.5">
                保留现有目标对象，不进行修改
              </div>
            </div>
            <SkipForward className="w-4 h-4 text-slate-400 group-hover:scale-110 transition-transform" />
          </button>

          <button
            onClick={() => onConfirm("cancel")}
            className="w-full py-2 text-center text-xs text-slate-400 hover:text-slate-200"
          >
            取消整个操作
          </button>
        </div>
      </div>
    </div>
  );
};
