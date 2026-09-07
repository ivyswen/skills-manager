import React, { useEffect, useState } from "react";
import { X, ExternalLink, Download, Trash2, CheckCircle2, Folder, FileText, Loader2 } from "lucide-react";
import { MarketSkillItem, MarketSkillDetail } from "../../types";
import { api } from "../../services/api";

interface MarketSkillDetailModalProps {
  skill: MarketSkillItem | null;
  isOpen: boolean;
  onClose: () => void;
  onInstall: (skill: MarketSkillItem) => void;
  onUninstall: (skill: MarketSkillItem) => void;
  actionLoading?: boolean;
}

export const MarketSkillDetailModal: React.FC<MarketSkillDetailModalProps> = ({
  skill,
  isOpen,
  onClose,
  onInstall,
  onUninstall,
  actionLoading = false,
}) => {
  const [detail, setDetail] = useState<MarketSkillDetail | null>(null);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!isOpen || !skill) {
      setDetail(null);
      return;
    }

    let isMounted = true;
    setLoading(true);

    api
      .getMarketSkillDetail(skill.skillId, skill.name, skill.source, skill.installs)
      .then((res) => {
        if (isMounted) setDetail(res);
      })
      .catch((err) => {
        console.error("获取技能详情失败:", err);
      })
      .finally(() => {
        if (isMounted) setLoading(false);
      });

    return () => {
      isMounted = false;
    };
  }, [isOpen, skill]);

  if (!isOpen || !skill) return null;

  const handleOpenSource = async () => {
    const githubUrl = `https://github.com/${skill.source}`;
    await api.openUrl(githubUrl);
  };

  const handleOpenLocalPath = async () => {
    if (detail?.local_path) {
      await api.openFolder(detail.local_path);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/75 backdrop-blur-sm p-4 animate-fade-in">
      <div className="bg-[#18191e] border border-slate-700/80 rounded-xl shadow-2xl w-full max-w-2xl max-h-[85vh] flex flex-col overflow-hidden">
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-800 bg-[#14151a]">
          <div className="flex items-center gap-3">
            <h2 className="text-base font-semibold text-slate-100">{skill.name}</h2>
            {actionLoading ? (
              <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 animate-pulse">
                <Loader2 className="w-3 h-3 animate-spin" />
                正在安装...
              </span>
            ) : skill.is_installed ? (
              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[11px] font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/30">
                <CheckCircle2 className="w-3 h-3" />
                已安装
              </span>
            ) : (
              <span className="px-2 py-0.5 rounded-full text-[11px] font-medium bg-slate-800 text-slate-400 border border-slate-700">
                未安装
              </span>
            )}
          </div>
          <button
            onClick={onClose}
            disabled={actionLoading}
            className="text-slate-400 hover:text-slate-200 p-1 rounded-md transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <X className="w-4 h-4" />
          </button>
        </div>

        {/* Content Body */}
        <div className="p-6 overflow-y-auto space-y-5 flex-1">
          {/* Active installing indicator banner */}
          {actionLoading && (
            <div className="flex items-center gap-3 p-3.5 bg-emerald-950/40 border border-emerald-600/40 rounded-xl text-emerald-200 text-xs shadow-inner animate-pulse">
              <Loader2 className="w-4 h-4 animate-spin text-emerald-400 shrink-0" />
              <div className="flex-1">
                <div className="font-semibold text-emerald-300">正在安装技能到本地中央仓库</div>
                <div className="text-[11px] text-emerald-400/80 mt-0.5">
                  正在从 GitHub 浅克隆仓库并安全搬运至中央仓库，这通常需要数秒，请稍候...
                </div>
              </div>
            </div>
          )}

          {/* Metadata info cards */}
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
            <div className="bg-slate-900/70 border border-slate-800 rounded-lg p-3">
              <div className="text-[11px] text-slate-400 mb-1">开源源码仓库 (GitHub)</div>
              <button
                onClick={handleOpenSource}
                className="flex items-center gap-1 text-xs font-mono text-indigo-400 hover:text-indigo-300 transition-colors group text-left truncate w-full"
              >
                <span className="truncate">{skill.source}</span>
                <ExternalLink className="w-3 h-3 shrink-0 group-hover:translate-x-0.5 transition-transform" />
              </button>
            </div>

            <div className="bg-slate-900/70 border border-slate-800 rounded-lg p-3">
              <div className="text-[11px] text-slate-400 mb-1">skills.sh 全网安装热度</div>
              <div className="flex items-center gap-1 text-xs font-medium text-slate-200">
                <Download className="w-3.5 h-3.5 text-slate-400" />
                <span>{skill.installs.toLocaleString()} 次安装</span>
              </div>
            </div>
          </div>

          {/* Local Canonical Path if Installed */}
          {detail?.local_path && (
            <div className="bg-slate-900/70 border border-slate-800 rounded-lg p-3">
              <div className="text-[11px] text-slate-400 mb-1">中央仓库本地路径</div>
              <div className="flex items-center justify-between gap-2">
                <span className="text-xs font-mono text-slate-300 truncate select-all">
                  {detail.local_path}
                </span>
                <button
                  onClick={handleOpenLocalPath}
                  className="px-2 py-1 text-[11px] font-medium text-slate-300 bg-slate-800 hover:bg-slate-700 rounded border border-slate-700 shrink-0 inline-flex items-center gap-1"
                >
                  <Folder className="w-3 h-3 text-teal-400" />
                  打开目录
                </button>
              </div>
            </div>
          )}

          {/* SKILL.md Content Preview */}
          <div className="space-y-2">
            <div className="flex items-center gap-1.5 text-xs font-medium text-slate-300">
              <FileText className="w-3.5 h-3.5 text-indigo-400" />
              <span>SKILL.md 说明文档预览</span>
            </div>

            {loading ? (
              <div className="py-12 flex flex-col items-center justify-center text-slate-400 text-xs gap-2 bg-slate-950/40 rounded-lg border border-slate-800/80">
                <Loader2 className="w-5 h-5 animate-spin text-indigo-400" />
                <span>正在加载技能详情...</span>
              </div>
            ) : detail?.content ? (
              <div className="bg-[#121317] border border-slate-800 rounded-lg p-4 max-h-72 overflow-y-auto">
                <pre className="text-xs font-mono text-slate-300 whitespace-pre-wrap leading-relaxed select-text font-normal">
                  {detail.content}
                </pre>
              </div>
            ) : (
              <div className="py-8 px-4 text-center bg-slate-950/40 border border-slate-800/80 rounded-lg text-xs text-slate-400">
                {skill.is_installed ? (
                  <span>本地目录下暂未找到有效 SKILL.md 文档。</span>
                ) : (
                  <div className="space-y-1">
                    <p>该技能尚未安装到本地中央仓库。</p>
                    <p className="text-[11px] text-slate-500">
                      点击下方【立即安装】按钮后，系统将自动从 GitHub 浅克隆并安全搬运至中央仓库。
                    </p>
                  </div>
                )}
              </div>
            )}
          </div>
        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-between px-6 py-4 bg-[#14151a] border-t border-slate-800">
          <button
            onClick={handleOpenSource}
            className="text-xs text-slate-400 hover:text-slate-200 inline-flex items-center gap-1 transition-colors"
          >
            <ExternalLink className="w-3.5 h-3.5" />
            <span>在 GitHub 上查看源码</span>
          </button>

          <div className="flex items-center gap-2.5">
            <button
              onClick={onClose}
              disabled={actionLoading}
              className="px-3.5 py-1.5 text-xs font-medium text-slate-300 hover:text-white bg-slate-800 hover:bg-slate-700 rounded-lg border border-slate-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              关闭
            </button>

            {skill.is_installed ? (
              <button
                onClick={() => {
                  onClose();
                  onUninstall(skill);
                }}
                disabled={actionLoading}
                className="inline-flex items-center gap-1.5 px-3.5 py-1.5 text-xs font-medium text-rose-400 hover:text-rose-300 bg-rose-950/40 hover:bg-rose-900/60 rounded-lg border border-rose-900/60 transition-colors disabled:opacity-50"
              >
                <Trash2 className="w-3.5 h-3.5" />
                <span>卸载技能</span>
              </button>
            ) : (
              <button
                onClick={() => onInstall(skill)}
                disabled={actionLoading}
                className="inline-flex items-center gap-1.5 px-4 py-1.5 text-xs font-medium text-white bg-emerald-600 hover:bg-emerald-500 rounded-lg shadow-sm border border-emerald-500/50 transition-colors disabled:opacity-80 disabled:cursor-wait"
              >
                {actionLoading ? (
                  <>
                    <Loader2 className="w-3.5 h-3.5 animate-spin text-white" />
                    <span>正在安装中...</span>
                  </>
                ) : (
                  <>
                    <Download className="w-3.5 h-3.5" />
                    <span>立即安装</span>
                  </>
                )}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
