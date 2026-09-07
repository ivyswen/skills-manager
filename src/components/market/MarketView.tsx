import React, { useState, useEffect, useCallback } from "react";
import { Search, Sparkles, AlertCircle } from "lucide-react";
import { MarketSkillItem, MarketUninstallCheckResult } from "../../types";
import { api } from "../../services/api";
import { MarketCard } from "./MarketCard";
import { MarketSkillDetailModal } from "./MarketSkillDetailModal";
import { MarketUninstallModal } from "../modals/MarketUninstallModal";

interface MarketViewProps {
  searchQuery: string;
  searchTrigger: number;
  onQuickSearch?: (query: string) => void;
  onSyncRepo: () => Promise<void>;
  onShowFeedback: (msg: string, type: "info" | "success" | "error") => void;
}

export const MarketView: React.FC<MarketViewProps> = ({
  searchQuery,
  searchTrigger,
  onQuickSearch,
  onSyncRepo,
  onShowFeedback,
}) => {
  const [skills, setSkills] = useState<MarketSkillItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);

  // Detail Modal State
  const [detailSkill, setDetailSkill] = useState<MarketSkillItem | null>(null);

  // Uninstall Modal State
  const [uninstallSkill, setUninstallSkill] = useState<MarketSkillItem | null>(null);
  const [uninstallCheckResult, setUninstallCheckResult] = useState<MarketUninstallCheckResult | null>(null);
  const [uninstallLoading, setUninstallLoading] = useState(false);

  // Operating skill id for button spinner
  const [actionSkillId, setActionSkillId] = useState<string | null>(null);

  // Execute Search
  const fetchMarketSkills = useCallback(
    async (query: string) => {
      setLoading(true);
      setErrorMessage(null);
      try {
        const res = await api.searchMarketSkills(query);
        setSkills(res.skills);
      } catch (err: any) {
        console.error("搜索 skills.sh 失败:", err);
        const msg = err?.message || String(err);
        setErrorMessage(msg);
        onShowFeedback(`获取市场技能失败: ${msg}`, "error");
      } finally {
        setLoading(false);
      }
    },
    [onShowFeedback]
  );

  // 仅在用户显式触发搜索且有关键词时执行，打开视图时不自动搜索
  useEffect(() => {
    if (searchTrigger > 0 && searchQuery.trim()) {
      setHasSearched(true);
      fetchMarketSkills(searchQuery.trim());
    }
  }, [searchTrigger, searchQuery, fetchMarketSkills]);

  // Handle Install Skill
  const handleInstall = async (skill: MarketSkillItem, forceOverwrite: boolean = false) => {
    setActionSkillId(skill.id);
    onShowFeedback(
      `正在从 GitHub (${skill.source}) 浅克隆并安装技能 "${skill.name}"，请稍候...`,
      "info"
    );
    try {
      await api.installMarketSkill({
        source: skill.source,
        skill_id: skill.skillId,
        skill_name: skill.name,
        force_overwrite: forceOverwrite,
      });

      onShowFeedback(`技能 "${skill.name}" 安装成功！`, "success");

      // Mark installed locally
      setSkills((prev) =>
        prev.map((s) => (s.id === skill.id ? { ...s, is_installed: true } : s))
      );
      setDetailSkill((prev) =>
        prev && prev.id === skill.id ? { ...prev, is_installed: true } : prev
      );

      // Trigger global sync with central repo
      await onSyncRepo();
    } catch (err: any) {
      const msg = err?.message || String(err);
      if (err?.code === "TARGET_CONFLICT" || msg.includes("已存在")) {
        // Confirm overwrite
        const shouldOverwrite = window.confirm(
          `中央仓库中已存在同名 Skill "${skill.name}"，是否覆盖重新安装？`
        );
        if (shouldOverwrite) {
          await handleInstall(skill, true);
          return;
        }
      } else {
        onShowFeedback(`安装失败: ${msg}`, "error");
      }
    } finally {
      setActionSkillId(null);
    }
  };

  // Handle Click Uninstall -> check mounts first
  const handleStartUninstall = async (skill: MarketSkillItem) => {
    setActionSkillId(skill.id);
    try {
      const checkRes = await api.checkMarketUninstall(skill.name);
      setUninstallSkill(skill);
      setUninstallCheckResult(checkRes);
    } catch (err: any) {
      onShowFeedback(`预检卸载状态失败: ${err?.message || err}`, "error");
    } finally {
      setActionSkillId(null);
    }
  };

  // Confirm Uninstall
  const handleConfirmUninstall = async (cascadeUnmount: boolean) => {
    if (!uninstallSkill) return;
    setUninstallLoading(true);
    try {
      const res = await api.uninstallMarketSkill({
        skill_name: uninstallSkill.name,
        cascade_unmount: cascadeUnmount,
      });

      onShowFeedback(
        `技能 "${uninstallSkill.name}" 已卸载${
          res.unmounted_count > 0 ? `，并级联清理了 ${res.unmounted_count} 个项目的软链接` : ""
        }`,
        "success"
      );

      // Update local card status
      setSkills((prev) =>
        prev.map((s) => (s.id === uninstallSkill.id ? { ...s, is_installed: false } : s))
      );
      setDetailSkill((prev) =>
        prev && prev.id === uninstallSkill.id ? { ...prev, is_installed: false } : prev
      );

      setUninstallSkill(null);
      setUninstallCheckResult(null);

      // Sync with central repo
      await onSyncRepo();
    } catch (err: any) {
      onShowFeedback(`卸载失败: ${err?.message || err}`, "error");
    } finally {
      setUninstallLoading(false);
    }
  };

  return (
    <div className="flex-1 overflow-y-auto bg-[#0f1013] text-slate-100 flex flex-col min-h-0">
      {/* Content Container */}
      <div className="flex-1 p-5 md:p-7 max-w-[1600px] w-full mx-auto">
        {/* Loading Skeletons */}
        {loading && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {Array.from({ length: 9 }).map((_, i) => (
              <div
                key={i}
                className="bg-[#18191e] border border-slate-800/80 rounded-xl p-4 animate-pulse space-y-4"
              >
                <div className="flex justify-between items-center">
                  <div className="h-5 bg-slate-800 rounded w-1/3"></div>
                  <div className="h-4 bg-slate-800 rounded w-16"></div>
                </div>
                <div className="flex items-center gap-3">
                  <div className="h-4 bg-slate-800 rounded w-28"></div>
                  <div className="h-4 bg-slate-800 rounded w-16"></div>
                </div>
                <div className="flex items-center gap-2 pt-2 border-t border-slate-800/40">
                  <div className="h-8 bg-slate-800 rounded-lg flex-1"></div>
                  <div className="h-8 bg-slate-800 rounded-lg flex-1"></div>
                </div>
              </div>
            ))}
          </div>
        )}

        {/* Error State */}
        {!loading && errorMessage && (
          <div className="py-16 flex flex-col items-center justify-center text-center max-w-md mx-auto">
            <div className="w-12 h-12 rounded-full bg-rose-500/10 border border-rose-500/20 flex items-center justify-center text-rose-400 mb-4">
              <AlertCircle className="w-6 h-6" />
            </div>
            <h3 className="text-sm font-semibold text-slate-200 mb-1">
              获取市场数据异常
            </h3>
            <p className="text-xs text-slate-400 mb-4 leading-relaxed">
              {errorMessage}
            </p>
            <button
              onClick={() => fetchMarketSkills(searchQuery)}
              className="px-4 py-1.5 text-xs font-medium text-white bg-indigo-600 hover:bg-indigo-500 rounded-lg shadow transition-colors"
            >
              重新尝试
            </button>
          </div>
        )}

        {/* Initial Guide State: 刚打开 skills.sh 视图未开始搜索时的引导页面 */}
        {!loading && !errorMessage && !hasSearched && (
          <div className="py-24 flex flex-col items-center justify-center text-center max-w-lg mx-auto">
            <div className="w-14 h-14 rounded-2xl bg-blue-500/10 border border-blue-500/20 flex items-center justify-center text-blue-400 mb-4 shadow-inner">
              <Search className="w-7 h-7" />
            </div>
            <h3 className="text-base font-semibold text-slate-100 mb-2">
              探索 skills.sh 技能市场
            </h3>
            <p className="text-xs text-slate-400 leading-relaxed mb-6">
              在上方搜索框输入技能名称或关键词，点击【搜索技能】或按 Enter 即可实时检索并一键安装到本地中央仓库。
            </p>
            <div className="flex flex-wrap items-center justify-center gap-2">
              <span className="text-xs text-slate-500 flex items-center gap-1">
                <Sparkles className="w-3.5 h-3.5 text-blue-400" />
                常用推荐：
              </span>
              {["ask", "git", "task", "agent", "code"].map((tag) => (
                <button
                  key={tag}
                  onClick={() => onQuickSearch?.(tag)}
                  className="px-2.5 py-1 text-xs font-medium text-slate-300 hover:text-white bg-slate-800/80 hover:bg-slate-700/80 rounded-lg border border-slate-700/60 transition-colors"
                >
                  {tag}
                </button>
              ))}
            </div>
          </div>
        )}

        {/* Empty Search State: 已执行搜索但未检索到匹配项 */}
        {!loading && !errorMessage && hasSearched && skills.length === 0 && (
          <div className="py-20 flex flex-col items-center justify-center text-center max-w-md mx-auto">
            <div className="w-12 h-12 rounded-full bg-slate-800/80 border border-slate-700/80 flex items-center justify-center text-slate-400 mb-4">
              <Search className="w-6 h-6" />
            </div>
            <h3 className="text-sm font-semibold text-slate-200 mb-1">
              未检索到匹配技能
            </h3>
            <p className="text-xs text-slate-400 leading-relaxed mb-4">
              未找到与 "{searchQuery}" 相关的开源 Skill。请尝试更简短的关键词或搜索常用词如 "git"、"agent"、"task"。
            </p>
            <button
              onClick={() => onQuickSearch?.("skills")}
              className="px-3.5 py-1.5 text-xs font-medium text-indigo-300 hover:text-white bg-indigo-950/50 hover:bg-indigo-900/60 rounded-lg border border-indigo-800/60 transition-colors inline-flex items-center gap-1.5"
            >
              <Sparkles className="w-3.5 h-3.5" />
              <span>探索推荐热门技能</span>
            </button>
          </div>
        )}

        {/* 3-Column Skills Grid */}
        {!loading && !errorMessage && skills.length > 0 && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            {skills.map((skill) => (
              <MarketCard
                key={skill.id}
                skill={skill}
                onViewDetail={setDetailSkill}
                onInstall={(s) => handleInstall(s, false)}
                onUninstall={handleStartUninstall}
                actionLoading={actionSkillId === skill.id}
              />
            ))}
          </div>
        )}
      </div>

      {/* Skill Detail Modal */}
      <MarketSkillDetailModal
        skill={detailSkill}
        isOpen={Boolean(detailSkill)}
        onClose={() => setDetailSkill(null)}
        onInstall={(s) => handleInstall(s, false)}
        onUninstall={handleStartUninstall}
        actionLoading={actionSkillId === detailSkill?.id}
      />

      {/* Uninstall Confirmation Modal */}
      <MarketUninstallModal
        skill={uninstallSkill}
        checkResult={uninstallCheckResult}
        isOpen={Boolean(uninstallSkill)}
        onClose={() => {
          setUninstallSkill(null);
          setUninstallCheckResult(null);
        }}
        onConfirm={handleConfirmUninstall}
        loading={uninstallLoading}
      />
    </div>
  );
};
