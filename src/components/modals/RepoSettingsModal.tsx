import React, { useState, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderGit2, X, Settings2, ShieldCheck } from "lucide-react";
import { Repository } from "../../types";
import { api } from "../../services/api";

interface RepoSettingsModalProps {
  isOpen: boolean;
  currentRepo: Repository | null;
  onSaveRepo: (path: string, name?: string) => Promise<void>;
  onClose: () => void;
}

export const RepoSettingsModal: React.FC<RepoSettingsModalProps> = ({
  isOpen,
  currentRepo,
  onSaveRepo,
  onClose,
}) => {
  const [repoPath, setRepoPath] = useState(currentRepo?.path || "");
  const [repoName, setRepoName] = useState(currentRepo?.name || "");
  const [closeToTray, setCloseToTray] = useState<boolean>(true);
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState("");
  const [trayFeedback, setTrayFeedback] = useState("");

  useEffect(() => {
    if (isOpen) {
      setRepoPath(currentRepo?.path || "");
      setRepoName(currentRepo?.name || "");
      setErrorMsg("");
      setTrayFeedback("");
      api.getAppSetting("close_to_tray")
        .then((val) => {
          // 默认为 true；仅当明确为 "false" 时才关闭
          setCloseToTray(val !== "false");
        })
        .catch((err) => {
          console.error("加载偏好设置失败:", err);
        });
    }
  }, [isOpen, currentRepo]);

  if (!isOpen) return null;

  const handlePick = async () => {
    try {
      const res = await open({
        directory: true,
        multiple: false,
        title: "选择中央 Skills 仓库目录 (需包含 skills/)",
      });
      if (res && typeof res === "string") {
        setRepoPath(res);
        const parts = res.split(/[/\\]/);
        setRepoName(parts[parts.length - 1] || "Central Repository");
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleToggleCloseToTray = async (checked: boolean) => {
    setCloseToTray(checked);
    try {
      await api.setAppSetting("close_to_tray", checked ? "true" : "false");
      setTrayFeedback(checked ? "已开启托盘常驻" : "已设为直接退出");
      setTimeout(() => setTrayFeedback(""), 2500);
    } catch (err: any) {
      setErrorMsg("保存托盘设置失败: " + (err?.message || String(err)));
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!repoPath.trim()) {
      setErrorMsg("请指定中央仓库路径");
      return;
    }
    setLoading(true);
    setErrorMsg("");
    try {
      await onSaveRepo(repoPath.trim(), repoName.trim() || undefined);
      onClose();
    } catch (err: any) {
      setErrorMsg(err?.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-lg w-full p-6 shadow-2xl space-y-5">
        <div className="flex items-start justify-between border-b border-slate-800 pb-3">
          <div className="flex items-center space-x-2 text-teal-400">
            <Settings2 className="w-5 h-5" />
            <h3 className="text-base font-semibold text-slate-100">应用与仓库设置</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-200 transition-colors">
            <X className="w-5 h-5" />
          </button>
        </div>

        {errorMsg && (
          <div className="bg-rose-950/40 border border-rose-800/60 rounded-lg p-2.5 text-xs text-rose-300">
            {errorMsg}
          </div>
        )}

        {/* 常规偏好设置 */}
        <div className="bg-slate-800/40 border border-slate-700/60 rounded-lg p-3.5 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-xs font-semibold text-slate-200">系统常规偏好</span>
            {trayFeedback && (
              <span className="text-[11px] text-teal-400 flex items-center space-x-1 animate-fade-in">
                <ShieldCheck className="w-3.5 h-3.5" />
                <span>{trayFeedback}</span>
              </span>
            )}
          </div>
          <label className="flex items-start space-x-3 cursor-pointer select-none pt-1">
            <input
              type="checkbox"
              checked={closeToTray}
              onChange={(e) => handleToggleCloseToTray(e.target.checked)}
              className="mt-0.5 w-4 h-4 text-teal-600 bg-slate-900 border-slate-600 rounded focus:ring-teal-500 focus:ring-offset-slate-900"
            />
            <div className="space-y-0.5">
              <div className="text-xs text-slate-200 font-medium">关闭窗口时最小化到系统托盘</div>
              <p className="text-[11px] text-slate-400 leading-relaxed">
                开启后，点击窗口关闭按钮 (×) 将最小化隐藏至系统托盘继续常驻后台，不中断状态。可通过托盘图标唤起或右键退出。
              </p>
            </div>
          </label>
        </div>

        {/* 中央仓库配置 */}
        <form onSubmit={handleSubmit} className="space-y-3.5 pt-1">
          <div className="flex items-center space-x-2 text-xs font-semibold text-slate-200">
            <FolderGit2 className="w-4 h-4 text-teal-400" />
            <span>配置中央 Skill 仓库</span>
          </div>

          <div className="space-y-1">
            <label className="text-xs font-medium text-slate-300">仓库目录绝对路径</label>
            <div className="flex space-x-2">
              <input
                type="text"
                value={repoPath}
                onChange={(e) => setRepoPath(e.target.value)}
                placeholder="F:/repos/my-skills"
                className="flex-1 px-3 py-1.5 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 focus:outline-none focus:border-teal-500 font-mono"
              />
              <button
                type="button"
                onClick={handlePick}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-md text-xs text-slate-200 transition-colors"
              >
                浏览
              </button>
            </div>
          </div>

          <div className="space-y-1">
            <label className="text-xs font-medium text-slate-300">仓库名称</label>
            <input
              type="text"
              value={repoName}
              onChange={(e) => setRepoName(e.target.value)}
              placeholder="我的公共 Skills 库"
              className="w-full px-3 py-1.5 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 focus:outline-none focus:border-teal-500"
            />
          </div>

          <p className="text-[11px] text-slate-400 leading-relaxed">
            注意：采用单中央来源设计，系统扫描该目录下 <code className="text-teal-300 font-mono">skills/</code> 子目录中的 Skill 原件。
          </p>

          <div className="flex justify-end space-x-2 pt-3 border-t border-slate-800">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200 transition-colors"
            >
              关闭
            </button>
            <button
              type="submit"
              disabled={loading}
              className="px-4 py-1.5 text-xs font-semibold rounded-md bg-teal-600 hover:bg-teal-500 text-white disabled:opacity-50 transition-colors"
            >
              {loading ? "正在扫描..." : "保存并扫描仓库"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
