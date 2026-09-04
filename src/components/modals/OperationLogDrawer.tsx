import React, { useState, useEffect, useCallback } from "react";
import {
  History,
  X,
  RotateCcw,
  CheckCircle,
  AlertCircle,
  RefreshCw,
  FolderOpen,
  FileText,
  Copy,
  Trash2,
  Check,
} from "lucide-react";
import { OperationLog, LogFileInfo } from "../../types";
import { api } from "../../services/api";

interface OperationLogDrawerProps {
  isOpen: boolean;
  logs: OperationLog[];
  onRollbackBatch: (batchId: string) => void;
  onRefreshLogs: () => void;
  onClose: () => void;
  loading: boolean;
}

export const OperationLogDrawer: React.FC<OperationLogDrawerProps> = ({
  isOpen,
  logs,
  onRollbackBatch,
  onRefreshLogs,
  onClose,
  loading,
}) => {
  const [activeTab, setActiveTab] = useState<"audit" | "raw">("audit");
  const [filterType, setFilterType] = useState<string>("all");
  const [rawText, setRawText] = useState<string>("");
  const [logInfo, setLogInfo] = useState<LogFileInfo | null>(null);
  const [copied, setCopied] = useState<boolean>(false);
  const [rawLoading, setRawLoading] = useState<boolean>(false);

  // 加载文本日志和文件信息
  const loadRawLogData = useCallback(async () => {
    setRawLoading(true);
    try {
      const [info, text] = await Promise.all([
        api.getLogFileInfo(),
        api.readLogText(300),
      ]);
      setLogInfo(info);
      setRawText(text);
    } catch (e) {
      console.error("加载文件日志异常:", e);
    } finally {
      setRawLoading(false);
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      loadRawLogData();
    }
  }, [isOpen, loadRawLogData]);

  if (!isOpen) return null;

  const filteredLogs = logs.filter((l) => {
    if (filterType === "all") return true;
    return l.operation_type === filterType;
  });

  const handleCopyText = async () => {
    if (rawText) {
      await navigator.clipboard.writeText(rawText);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleClearLog = async () => {
    if (window.confirm("确定要清空当前日志文件内容吗？")) {
      try {
        await api.clearLogFile();
        await loadRawLogData();
      } catch (e) {
        console.error("清空日志异常:", e);
      }
    }
  };

  const handleOpenDir = async () => {
    try {
      await api.openLogDir();
    } catch (e) {
      console.error("打开日志目录异常:", e);
    }
  };

  const handleOpenFile = async () => {
    try {
      await api.openLogFile();
    } catch (e) {
      console.error("打开日志文件异常:", e);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex justify-end">
      <div className="w-full max-w-2xl bg-slate-900 border-l border-slate-800 h-full flex flex-col shadow-2xl">
        {/* 抽屉头部 */}
        <div className="p-4 border-b border-slate-800 flex items-center justify-between">
          <div className="flex items-center space-x-2 text-slate-200">
            <History className="w-5 h-5 text-teal-400" />
            <h3 className="font-semibold text-sm">操作审计与系统日志</h3>
          </div>
          <div className="flex items-center space-x-1">
            <button
              onClick={() => {
                onRefreshLogs();
                loadRawLogData();
              }}
              className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded transition-colors"
              title="刷新全部日志"
            >
              <RefreshCw className={`w-4 h-4 ${loading || rawLoading ? "animate-spin" : ""}`} />
            </button>
            <button
              onClick={onClose}
              className="p-1.5 text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded transition-colors"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </div>

        {/* 模式选择 Tab 与 文件快捷入口 */}
        <div className="px-4 py-2.5 bg-slate-950/50 border-b border-slate-800 flex items-center justify-between text-xs">
          <div className="flex items-center space-x-1 bg-slate-800 p-0.5 rounded-md">
            <button
              onClick={() => setActiveTab("audit")}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                activeTab === "audit"
                  ? "bg-teal-600 text-white shadow-xs"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              操作审计记录 ({logs.length})
            </button>
            <button
              onClick={() => {
                setActiveTab("raw");
                loadRawLogData();
              }}
              className={`px-3 py-1 rounded text-xs font-medium transition-colors ${
                activeTab === "raw"
                  ? "bg-teal-600 text-white shadow-xs"
                  : "text-slate-400 hover:text-slate-200"
              }`}
            >
              文件文本日志 (skills-manager.log)
            </button>
          </div>

          <div className="flex items-center space-x-2">
            <button
              onClick={handleOpenFile}
              className="inline-flex items-center space-x-1 px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs border border-slate-700 transition-colors"
              title="使用系统默认记事本打开日志文件"
            >
              <FileText className="w-3.5 h-3.5 text-teal-400" />
              <span>打开文件</span>
            </button>
            <button
              onClick={handleOpenDir}
              className="inline-flex items-center space-x-1 px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs border border-slate-700 transition-colors"
              title="在 Windows 文件资源管理器中定位日志文件夹"
            >
              <FolderOpen className="w-3.5 h-3.5 text-amber-400" />
              <span>打开目录</span>
            </button>
          </div>
        </div>

        {/* 日志文件路径提示条 */}
        {logInfo && (
          <div className="px-4 py-1.5 bg-slate-900/90 border-b border-slate-800/80 flex items-center justify-between text-[11px] font-mono text-slate-400">
            <div className="truncate max-w-[80%]" title={logInfo.path}>
              <span className="text-slate-500">日志路径: </span>
              {logInfo.path}
            </div>
            <div>{(logInfo.size_bytes / 1024).toFixed(1)} KB</div>
          </div>
        )}

        {activeTab === "audit" ? (
          <>
            {/* 操作审计卡片筛选条 */}
            <div className="p-3 bg-slate-950/30 border-b border-slate-800 flex items-center space-x-2 text-xs">
              <span className="text-slate-400 text-[11px]">类型：</span>
              <select
                value={filterType}
                onChange={(e) => setFilterType(e.target.value)}
                className="bg-slate-800 border border-slate-700 rounded px-2 py-1 text-slate-300 text-xs focus:outline-none"
              >
                <option value="all">全部操作</option>
                <option value="mount">挂载 (mount)</option>
                <option value="unmount">卸载 (unmount)</option>
                <option value="git_pull">Git 更新 (git_pull)</option>
                <option value="project_add">项目登记 (project_add)</option>
              </select>
              <span className="text-slate-500 text-[11px] ml-auto">
                共 {filteredLogs.length} 条记录
              </span>
            </div>

            {/* 日志卡片列表 */}
            <div className="flex-1 overflow-y-auto p-4 space-y-3">
              {filteredLogs.length === 0 ? (
                <div className="text-center py-16 text-slate-500 text-xs">暂无审计日志</div>
              ) : (
                filteredLogs.map((log) => {
                  const isSuccess = log.status === "SUCCESS";
                  const isRolledBack = log.status === "ROLLED_BACK";

                  return (
                    <div
                      key={log.id}
                      className="bg-slate-850 border border-slate-800 rounded-lg p-3 space-y-2 text-xs"
                    >
                      <div className="flex items-center justify-between">
                        <div className="flex items-center space-x-2">
                          {isSuccess ? (
                            <CheckCircle className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                          ) : isRolledBack ? (
                            <RotateCcw className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                          ) : (
                            <AlertCircle className="w-3.5 h-3.5 text-rose-400 shrink-0" />
                          )}
                          <span className="font-semibold text-slate-200 uppercase text-[11px]">
                            {log.operation_type}
                          </span>
                          {log.skill_name && (
                            <span className="text-teal-300 font-mono text-[11px]">
                              {log.skill_name}
                            </span>
                          )}
                        </div>

                        <div className="flex items-center space-x-2">
                          <span className="text-[10px] text-slate-500">
                            {new Date(log.created_at).toLocaleTimeString()}
                          </span>
                          {log.batch_id && isSuccess && log.operation_type === "mount" && (
                            <button
                              onClick={() => onRollbackBatch(log.batch_id!)}
                              className="inline-flex items-center space-x-1 px-2 py-0.5 rounded bg-slate-800 hover:bg-slate-700 text-teal-400 text-[10px] border border-slate-700 transition-colors"
                              title="安全撤销本批次所有创建操作并还原备份"
                            >
                              <RotateCcw className="w-2.5 h-2.5" />
                              <span>回滚本批次</span>
                            </button>
                          )}
                        </div>
                      </div>

                      <p className="text-slate-300 leading-relaxed text-[11px]">{log.message}</p>

                      {log.target_path && (
                        <div
                          className="text-[10px] font-mono text-slate-500 truncate"
                          title={log.target_path}
                        >
                          目标: {log.target_path}
                        </div>
                      )}

                      {log.backup_path && (
                        <div
                          className="text-[10px] font-mono text-amber-500/80 truncate"
                          title={log.backup_path}
                        >
                          备份点: {log.backup_path}
                        </div>
                      )}
                    </div>
                  );
                })
              )}
            </div>
          </>
        ) : (
          /* 文本日志视图 */
          <div className="flex-1 flex flex-col overflow-hidden p-4 space-y-2.5">
            <div className="flex items-center justify-between text-xs text-slate-400">
              <span>展示最近 300 行实时日志记录：</span>
              <div className="flex items-center space-x-2">
                <button
                  onClick={handleCopyText}
                  className="inline-flex items-center space-x-1 px-2.5 py-1 rounded bg-slate-800 hover:bg-slate-700 text-slate-200 border border-slate-700 transition-colors"
                  title="复制全文"
                >
                  {copied ? <Check className="w-3.5 h-3.5 text-emerald-400" /> : <Copy className="w-3.5 h-3.5" />}
                  <span>{copied ? "已复制" : "复制"}</span>
                </button>
                <button
                  onClick={handleClearLog}
                  className="inline-flex items-center space-x-1 px-2.5 py-1 rounded bg-rose-950/40 hover:bg-rose-900/50 text-rose-300 border border-rose-800/50 transition-colors"
                  title="清空当前日志文件"
                >
                  <Trash2 className="w-3.5 h-3.5" />
                  <span>清空</span>
                </button>
              </div>
            </div>

            <div className="flex-1 overflow-auto bg-slate-950 border border-slate-800 rounded-lg p-3 font-mono text-xs text-slate-300 leading-relaxed whitespace-pre select-text">
              {rawLoading ? (
                <div className="text-slate-500 text-center py-10">正在加载日志文件...</div>
              ) : rawText ? (
                rawText
              ) : (
                <div className="text-slate-500 text-center py-10">暂无日志内容</div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};
