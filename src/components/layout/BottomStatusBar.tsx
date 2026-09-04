import React from "react";
import { CheckCircle, AlertCircle, RotateCcw, Info } from "lucide-react";

interface BottomStatusBarProps {
  lastMessage: string | null;
  lastMessageType: "info" | "success" | "error";
  recentBatchId: string | null;
  onRollbackBatch: (batchId: string) => void;
  rollingBack: boolean;
}

export const BottomStatusBar: React.FC<BottomStatusBarProps> = ({
  lastMessage,
  lastMessageType,
  recentBatchId,
  onRollbackBatch,
  rollingBack,
}) => {
  return (
    <footer className="h-7 bg-slate-900 border-t border-slate-800 flex items-center justify-between px-3 text-[11px] text-slate-400 select-none shrink-0">
      <div className="flex items-center space-x-2 truncate">
        {lastMessageType === "success" && <CheckCircle className="w-3.5 h-3.5 text-emerald-400 shrink-0" />}
        {lastMessageType === "error" && <AlertCircle className="w-3.5 h-3.5 text-rose-400 shrink-0" />}
        {lastMessageType === "info" && <Info className="w-3.5 h-3.5 text-teal-400 shrink-0" />}
        <span className="truncate">{lastMessage || "就绪"}</span>
      </div>

      <div className="flex items-center space-x-3 shrink-0">
        {recentBatchId && (
          <button
            onClick={() => onRollbackBatch(recentBatchId)}
            disabled={rollingBack}
            className="inline-flex items-center space-x-1 text-teal-400 hover:text-teal-300 font-medium transition-colors disabled:opacity-50"
            title="撤销并回滚最近一次批量操作"
          >
            <RotateCcw className={`w-3 h-3 ${rollingBack ? "animate-spin" : ""}`} />
            <span>回滚上一批次</span>
          </button>
        )}
        <span className="text-slate-600">|</span>
        <span className="text-slate-500">Windows Native Reparse Point & Junction 支持</span>
      </div>
    </footer>
  );
};
