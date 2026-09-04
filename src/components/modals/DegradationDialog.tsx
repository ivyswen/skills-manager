import React from "react";
import { ShieldAlert, GitBranch, Copy, X } from "lucide-react";

interface DegradationDialogProps {
  isOpen: boolean;
  reason: string;
  onSelectMode: (mode: "junction" | "copy") => void;
  onClose: () => void;
}

export const DegradationDialog: React.FC<DegradationDialogProps> = ({
  isOpen,
  reason,
  onSelectMode,
  onClose,
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-md w-full p-6 shadow-2xl space-y-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 text-amber-400">
            <ShieldAlert className="w-5 h-5" />
            <h3 className="text-sm font-semibold text-slate-100">软链接降级选择</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <p className="text-xs text-slate-300 leading-relaxed">
          {reason || "Windows 相对软链接创建受阻（未开启开发者模式或处于跨卷/跨盘符场景）。请选择降级挂载方案："}
        </p>

        <div className="space-y-2.5 pt-1">
          <button
            onClick={() => onSelectMode("junction")}
            className="w-full flex items-center justify-between p-3.5 rounded-lg bg-teal-950 hover:bg-teal-900 border border-teal-800/80 text-left transition-colors group"
          >
            <div>
              <div className="text-xs font-semibold text-teal-300">
                NTFS Junction 绝对联接点 (推荐)
              </div>
              <div className="text-[10px] text-teal-400/80 mt-1 leading-relaxed">
                无需管理员权限与开发者模式，Windows 原生底层原生支持跨卷挂载，原件更新即时同步。
              </div>
            </div>
            <GitBranch className="w-5 h-5 text-teal-400 shrink-0 ml-2 group-hover:scale-110 transition-transform" />
          </button>

          <button
            onClick={() => onSelectMode("copy")}
            className="w-full flex items-center justify-between p-3.5 rounded-lg bg-slate-800 hover:bg-slate-750 border border-slate-700 text-left transition-colors group"
          >
            <div>
              <div className="text-xs font-semibold text-slate-200">
                独立 Copy 副本模式
              </div>
              <div className="text-[10px] text-slate-400 mt-1 leading-relaxed">
                复制全量文件至目标项目。完全脱离中央原件，原件更新需手动在看板点击一键覆盖同步。
              </div>
            </div>
            <Copy className="w-5 h-5 text-slate-400 shrink-0 ml-2 group-hover:scale-110 transition-transform" />
          </button>
        </div>
      </div>
    </div>
  );
};
