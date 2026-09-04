import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { Sparkles, FolderGit2, FolderPlus, ArrowRight, Check } from "lucide-react";

interface OnboardingWizardProps {
  isOpen: boolean;
  onComplete: (repoPath: string, repoName: string, projectPath?: string, projectName?: string) => Promise<void>;
}

export const OnboardingWizard: React.FC<OnboardingWizardProps> = ({
  isOpen,
  onComplete,
}) => {
  const [step, setStep] = useState<1 | 2>(1);
  const [repoPath, setRepoPath] = useState("");
  const [repoName, setRepoName] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [projectName, setProjectName] = useState("");
  const [submitting, setSubmitting] = useState(false);
  const [errorMsg, setErrorMsg] = useState("");

  if (!isOpen) return null;

  const handlePickRepo = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择中央 Skills 仓库目录 (包含 skills/)",
      });
      if (selected && typeof selected === "string") {
        setRepoPath(selected);
        const parts = selected.split(/[/\\]/);
        setRepoName(parts[parts.length - 1] || "Central Repository");
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handlePickProject = async () => {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择目标项目目录",
      });
      if (selected && typeof selected === "string") {
        setProjectPath(selected);
        const parts = selected.split(/[/\\]/);
        setProjectName(parts[parts.length - 1] || "My Project");
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleFinish = async () => {
    if (!repoPath.trim()) {
      setErrorMsg("请指定有效的中央仓库目录");
      return;
    }
    setSubmitting(true);
    setErrorMsg("");
    try {
      await onComplete(
        repoPath.trim(),
        repoName.trim() || "Central Repository",
        projectPath.trim() || undefined,
        projectName.trim() || undefined
      );
    } catch (err: any) {
      setErrorMsg(err?.message || String(err));
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-slate-950/85 backdrop-blur-md flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-2xl max-w-lg w-full p-8 shadow-2xl space-y-6">
        <div className="text-center space-y-2">
          <div className="w-12 h-12 rounded-xl bg-teal-500/20 border border-teal-500/40 text-teal-400 flex items-center justify-center mx-auto mb-3">
            <Sparkles className="w-6 h-6" />
          </div>
          <h2 className="text-lg font-bold text-slate-100">欢迎使用 Skills 管理工具</h2>
          <p className="text-xs text-slate-400 leading-relaxed max-w-sm mx-auto">
            仅需两步，即可将统一的 Skill 仓库安全、快捷地挂载到您的各类 AI Coding 项目中。
          </p>
        </div>

        {errorMsg && (
          <div className="bg-rose-950/40 border border-rose-800/60 rounded-lg p-2.5 text-xs text-rose-300">
            {errorMsg}
          </div>
        )}

        {step === 1 ? (
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-300 flex items-center justify-between">
                <span>第一步：选择中央 Skills 来源目录</span>
                <span className="text-[10px] text-teal-400">必须包含 skills/ 目录</span>
              </label>
              <div className="flex space-x-2">
                <input
                  type="text"
                  placeholder="例如：D:/my-skills 或 F:/repos/central-skills"
                  value={repoPath}
                  onChange={(e) => setRepoPath(e.target.value)}
                  className="flex-1 px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-xs text-slate-200 focus:outline-none focus:border-teal-500 font-mono"
                />
                <button
                  type="button"
                  onClick={handlePickRepo}
                  className="px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-xs font-medium text-slate-200 flex items-center space-x-1"
                >
                  <FolderGit2 className="w-4 h-4 text-teal-400" />
                  <span>浏览</span>
                </button>
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-300">来源名称 (可选)</label>
              <input
                type="text"
                placeholder="我的中央仓库"
                value={repoName}
                onChange={(e) => setRepoName(e.target.value)}
                className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-xs text-slate-200 focus:outline-none focus:border-teal-500"
              />
            </div>

            <div className="pt-2 flex justify-end">
              <button
                type="button"
                disabled={!repoPath.trim()}
                onClick={() => setStep(2)}
                className="inline-flex items-center space-x-2 px-5 py-2 rounded-lg bg-teal-600 hover:bg-teal-500 disabled:opacity-40 text-white font-medium text-xs transition-colors"
              >
                <span>下一步：添加初始项目</span>
                <ArrowRight className="w-4 h-4" />
              </button>
            </div>
          </div>
        ) : (
          <div className="space-y-4">
            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-300 flex items-center justify-between">
                <span>第二步：选择目标项目 (可选，后续可随时添加)</span>
              </label>
              <div className="flex space-x-2">
                <input
                  type="text"
                  placeholder="例如：C:/Users/dev/my-react-app"
                  value={projectPath}
                  onChange={(e) => setProjectPath(e.target.value)}
                  className="flex-1 px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-xs text-slate-200 focus:outline-none focus:border-teal-500 font-mono"
                />
                <button
                  type="button"
                  onClick={handlePickProject}
                  className="px-3 py-2 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-lg text-xs font-medium text-slate-200 flex items-center space-x-1"
                >
                  <FolderPlus className="w-4 h-4 text-teal-400" />
                  <span>浏览</span>
                </button>
              </div>
            </div>

            <div className="space-y-1.5">
              <label className="text-xs font-semibold text-slate-300">项目显示名称</label>
              <input
                type="text"
                placeholder="我的初始项目"
                value={projectName}
                onChange={(e) => setProjectName(e.target.value)}
                className="w-full px-3 py-2 bg-slate-800 border border-slate-700 rounded-lg text-xs text-slate-200 focus:outline-none focus:border-teal-500"
              />
            </div>

            <div className="pt-2 flex items-center justify-between">
              <button
                type="button"
                onClick={() => setStep(1)}
                className="text-xs text-slate-400 hover:text-slate-200"
              >
                返回上一步
              </button>
              <button
                type="button"
                disabled={submitting}
                onClick={handleFinish}
                className="inline-flex items-center space-x-2 px-5 py-2 rounded-lg bg-teal-600 hover:bg-teal-500 disabled:opacity-50 text-white font-medium text-xs transition-colors"
              >
                <Check className="w-4 h-4" />
                <span>{submitting ? "正在初始化..." : "完成并进入主看板"}</span>
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
