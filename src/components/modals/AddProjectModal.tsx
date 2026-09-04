import React, { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderPlus, X } from "lucide-react";

interface AddProjectModalProps {
  isOpen: boolean;
  onAddProject: (path: string, name?: string) => Promise<void>;
  onClose: () => void;
}

export const AddProjectModal: React.FC<AddProjectModalProps> = ({
  isOpen,
  onAddProject,
  onClose,
}) => {
  const [projectPath, setProjectPath] = useState("");
  const [projectName, setProjectName] = useState("");
  const [loading, setLoading] = useState(false);
  const [errorMsg, setErrorMsg] = useState("");

  if (!isOpen) return null;

  const handlePick = async () => {
    try {
      const res = await open({
        directory: true,
        multiple: false,
        title: "选择目标项目目录",
      });
      if (res && typeof res === "string") {
        setProjectPath(res);
        const parts = res.split(/[/\\]/);
        setProjectName(parts[parts.length - 1] || "My Project");
      }
    } catch (e) {
      console.error(e);
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!projectPath.trim()) {
      setErrorMsg("请指定项目路径");
      return;
    }
    setLoading(true);
    setErrorMsg("");
    try {
      await onAddProject(projectPath.trim(), projectName.trim() || undefined);
      setProjectPath("");
      setProjectName("");
      onClose();
    } catch (err: any) {
      setErrorMsg(err?.message || String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-md w-full p-6 shadow-2xl space-y-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 text-teal-400">
            <FolderPlus className="w-5 h-5" />
            <h3 className="text-sm font-semibold text-slate-100">添加目标项目</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        {errorMsg && (
          <div className="bg-rose-950/40 border border-rose-800/60 rounded-lg p-2.5 text-xs text-rose-300">
            {errorMsg}
          </div>
        )}

        <form onSubmit={handleSubmit} className="space-y-3.5">
          <div className="space-y-1">
            <label className="text-xs font-medium text-slate-300">项目本地路径</label>
            <div className="flex space-x-2">
              <input
                type="text"
                value={projectPath}
                onChange={(e) => setProjectPath(e.target.value)}
                placeholder="D:/workspace/my-app"
                className="flex-1 px-3 py-1.5 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 focus:outline-none focus:border-teal-500 font-mono"
              />
              <button
                type="button"
                onClick={handlePick}
                className="px-3 py-1.5 bg-slate-800 hover:bg-slate-700 border border-slate-700 rounded-md text-xs text-slate-200"
              >
                浏览
              </button>
            </div>
          </div>

          <div className="space-y-1">
            <label className="text-xs font-medium text-slate-300">项目显示名称</label>
            <input
              type="text"
              value={projectName}
              onChange={(e) => setProjectName(e.target.value)}
              placeholder="my-app"
              className="w-full px-3 py-1.5 bg-slate-800 border border-slate-700 rounded-md text-xs text-slate-200 focus:outline-none focus:border-teal-500"
            />
          </div>

          <p className="text-[11px] text-slate-400 leading-relaxed">
            登记后，系统将自动在项目内创建 <code className="text-teal-300 font-mono">.agents/skills</code> 作为唯一安装目录。
          </p>

          <div className="flex justify-end space-x-2 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200"
            >
              取消
            </button>
            <button
              type="submit"
              disabled={loading}
              className="px-4 py-1.5 text-xs font-semibold rounded-md bg-teal-600 hover:bg-teal-500 text-white disabled:opacity-50 transition-colors"
            >
              {loading ? "正在添加..." : "确认添加"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
