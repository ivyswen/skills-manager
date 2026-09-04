import React, { useState } from "react";
import { FileUp, CheckCircle2, AlertTriangle, X } from "lucide-react";
import { ImportPreviewResult } from "../../types";

interface ImportPreviewModalProps {
  isOpen: boolean;
  filePath?: string;
  previewData: ImportPreviewResult;
  onConfirmApply: (pathMappings: Record<string, string>) => void;
  onClose: () => void;
}

export const ImportPreviewModal: React.FC<ImportPreviewModalProps> = ({
  isOpen,
  previewData,
  onConfirmApply,
  onClose,
}) => {
  const [mappings, setMappings] = useState<Record<string, string>>(() => {
    const initial: Record<string, string> = {};
    if (previewData.repository) {
      initial[previewData.repository.path] = previewData.repository.path;
    }
    for (const p of previewData.projects) {
      initial[p.original_path] = p.original_path;
    }
    return initial;
  });

  if (!isOpen) return null;

  const handleRemapChange = (orig: string, val: string) => {
    setMappings((prev) => ({ ...prev, [orig]: val }));
  };

  const handleApply = () => {
    onConfirmApply(mappings);
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-slate-900 border border-slate-700 rounded-xl max-w-2xl w-full p-6 shadow-2xl space-y-4">
        <div className="flex items-start justify-between">
          <div className="flex items-center space-x-2 text-teal-400">
            <FileUp className="w-5 h-5" />
            <h3 className="text-sm font-semibold text-slate-100">导入配置预检与路径映射</h3>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-slate-200">
            <X className="w-4 h-4" />
          </button>
        </div>

        <p className="text-xs text-slate-300 leading-relaxed">
          正在读取配置文件。跨机器导入时路径可能发生变化，请检查并修正中央仓库与目标项目在本机的实际路径：
        </p>

        {previewData.repository && (
          <div className="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-2 text-xs">
            <div className="flex items-center justify-between">
              <div>
                <span className="text-slate-400">中央仓库：</span>
                <span className="font-semibold text-slate-200 ml-1">
                  {previewData.repository.name}
                </span>
                <div className="text-[11px] font-mono text-slate-500 mt-0.5">
                  原路径: {previewData.repository.path}
                </div>
              </div>
              {previewData.repository_exists ? (
                <span className="text-[10px] text-emerald-400 bg-emerald-500/10 border border-emerald-500/20 px-2 py-0.5 rounded">
                  本机路径有效
                </span>
              ) : (
                <span className="text-[10px] text-rose-400 bg-rose-500/10 border border-rose-500/20 px-2 py-0.5 rounded">
                  本机路径不存在
                </span>
              )}
            </div>
            <div className="flex items-center space-x-2">
              <span className="text-[11px] text-slate-400 shrink-0">映射到本机:</span>
              <input
                type="text"
                value={mappings[previewData.repository.path] ?? previewData.repository.path}
                onChange={(e) => handleRemapChange(previewData.repository!.path, e.target.value)}
                className="flex-1 px-2 py-1 bg-slate-900 border border-slate-700 rounded text-xs text-slate-200 focus:outline-none focus:border-teal-500 font-mono"
              />
            </div>
          </div>
        )}

        <div className="space-y-2">
          <div className="text-xs font-semibold text-slate-300">
            项目映射清单 ({previewData.projects.length})
          </div>
          <div className="max-h-60 overflow-y-auto space-y-2 pr-1">
            {previewData.projects.map((proj) => {
              const currentMapped = mappings[proj.original_path] ?? proj.original_path;
              return (
                <div
                  key={proj.original_path}
                  className="bg-slate-950 border border-slate-800 rounded-lg p-3 space-y-2"
                >
                  <div className="flex items-center justify-between text-xs">
                    <div className="flex items-center space-x-1.5">
                      {proj.exists ? (
                        <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                      ) : (
                        <AlertTriangle className="w-3.5 h-3.5 text-amber-400" />
                      )}
                      <span className="font-medium text-slate-200 truncate max-w-sm">
                        原路径: {proj.original_path}
                      </span>
                    </div>
                    <span className="text-[10px] text-slate-500">
                      包含 {proj.skill_names.length} 个挂载 Skill
                    </span>
                  </div>

                  <div className="flex items-center space-x-2">
                    <span className="text-[11px] text-slate-400 shrink-0">映射到本机:</span>
                    <input
                      type="text"
                      value={currentMapped}
                      onChange={(e) => handleRemapChange(proj.original_path, e.target.value)}
                      className="flex-1 px-2 py-1 bg-slate-900 border border-slate-700 rounded text-xs text-slate-200 focus:outline-none focus:border-teal-500"
                    />
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <div className="flex items-center justify-end space-x-2 pt-2">
          <button
            onClick={onClose}
            className="px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200"
          >
            取消
          </button>
          <button
            onClick={handleApply}
            className="px-4 py-1.5 text-xs font-semibold rounded-md bg-teal-600 hover:bg-teal-500 text-white transition-colors"
          >
            确认导入并应用
          </button>
        </div>
      </div>
    </div>
  );
};
