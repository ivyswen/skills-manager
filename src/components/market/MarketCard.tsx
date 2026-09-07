import React from "react";
import { Download, ExternalLink, Trash2, Loader2 } from "lucide-react";
import { MarketSkillItem } from "../../types";

interface MarketCardProps {
  skill: MarketSkillItem;
  onViewDetail: (skill: MarketSkillItem) => void;
  onInstall: (skill: MarketSkillItem) => void;
  onUninstall: (skill: MarketSkillItem) => void;
  actionLoading?: boolean;
}

export const MarketCard: React.FC<MarketCardProps> = ({
  skill,
  onViewDetail,
  onInstall,
  onUninstall,
  actionLoading = false,
}) => {
  return (
    <div
      className={`bg-[#18191e] hover:bg-[#1f2026] rounded-xl p-4 flex flex-col justify-between transition-all duration-150 shadow-sm border ${
        actionLoading
          ? "border-emerald-500/70 ring-1 ring-emerald-500/30"
          : "border-slate-800/80 hover:border-slate-700/80"
      }`}
    >
      <div>
        {/* Top: Skill Name & Installed Badge */}
        <div className="flex items-start justify-between gap-2 mb-2.5">
          <h3
            className="font-semibold text-slate-100 text-sm md:text-base leading-snug truncate select-text cursor-default"
            title={skill.name}
          >
            {skill.name}
          </h3>
          {skill.is_installed && (
            <span className="px-2.5 py-0.5 rounded-full text-[11px] font-medium bg-[#2e7d32] text-white shrink-0 shadow-sm">
              已安装
            </span>
          )}
        </div>

        {/* Source Repo & Installs Count */}
        <div className="flex items-center gap-2.5 text-xs text-slate-400 mb-5 flex-wrap">
          <span
            className="px-2.5 py-0.5 rounded-full bg-[#202127] text-slate-300 font-mono text-[11px] border border-slate-700/50 truncate max-w-[190px]"
            title={skill.source}
          >
            {skill.source}
          </span>
          <span className="inline-flex items-center gap-1 text-slate-400 text-xs shrink-0">
            <Download className="w-3.5 h-3.5 text-slate-400" />
            <span>{skill.installs.toLocaleString()}</span>
          </span>
        </div>
      </div>

      {/* Action Buttons: [ 查看 ] [ 安装 / 卸载 ] */}
      <div className="flex items-center gap-2.5 pt-1">
        <button
          onClick={() => onViewDetail(skill)}
          disabled={actionLoading}
          className="flex-1 inline-flex items-center justify-center gap-1.5 py-2 px-3 text-xs font-medium rounded-lg bg-[#27282e] hover:bg-[#32333b] text-slate-200 hover:text-white border border-slate-700/60 transition-colors shadow-sm disabled:opacity-50"
          title="查看技能详情"
        >
          <ExternalLink className="w-3.5 h-3.5 text-slate-300" />
          <span>查看</span>
        </button>

        {skill.is_installed ? (
          <button
            onClick={() => onUninstall(skill)}
            disabled={actionLoading}
            className="flex-1 inline-flex items-center justify-center gap-1.5 py-2 px-3 text-xs font-medium rounded-lg bg-[#241315] hover:bg-[#32171a] text-red-400 hover:text-red-300 border border-red-900/60 hover:border-red-800/80 transition-colors disabled:opacity-75 disabled:cursor-wait"
            title="从中央仓库卸载此技能"
          >
            {actionLoading ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin text-red-400" />
                <span>正在卸载...</span>
              </>
            ) : (
              <>
                <Trash2 className="w-3.5 h-3.5" />
                <span>卸载</span>
              </>
            )}
          </button>
        ) : (
          <button
            onClick={() => onInstall(skill)}
            disabled={actionLoading}
            className="flex-1 inline-flex items-center justify-center gap-1.5 py-2 px-3 text-xs font-medium rounded-lg bg-[#2e7d32] hover:bg-[#388e3c] text-white border border-green-600/50 shadow-sm transition-colors disabled:opacity-85 disabled:cursor-wait"
            title="克隆并安装到中央仓库"
          >
            {actionLoading ? (
              <>
                <Loader2 className="w-3.5 h-3.5 animate-spin text-white" />
                <span>正在安装...</span>
              </>
            ) : (
              <>
                <Download className="w-3.5 h-3.5" />
                <span>安装</span>
              </>
            )}
          </button>
        )}
      </div>
    </div>
  );
};
