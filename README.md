# Skills 管理工具 (Skills Manager)

[![Tauri v2](https://img.shields.io/badge/Tauri-v2-blue.svg)](https://v2.tauri.app/)
[![Rust](https://img.shields.io/badge/Rust-1.77%2B-orange.svg)](https://www.rust-lang.org/)
[![React 19](https://img.shields.io/badge/React-19-cyan.svg)](https://react.dev/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.x-blue.svg)](https://www.typescriptlang.org/)
[![TailwindCSS](https://img.shields.io/badge/TailwindCSS-3.x-38bdf8.svg)](https://tailwindcss.com/)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

基于 **Tauri v2 + Rust + React 19 + SQLite** 构建的轻量级、跨平台桌面客户端（优先面向 Windows 深度优化），专为多 Agent 开发环境提供安全、可控、高韧性的 Agent Skills 集中化管理与分发挂载服务。

---

## 🌟 核心特性

### 1. 中央原件仓库与安全扫描
- **单中央仓库管理**：支持本地普通目录或 Git 仓库作为唯一中央 Skill 原件池。
- **Git 变更感知与脏工作区拦截**：
  - 更新前自动检测工作区未提交修改（`git status --porcelain`），拦截潜在冲突；
  - 拉取后通过 `git diff` 自动捕获被删除或重命名的 Skill 原件，检索所有受影响的项目挂载项，并在前端以确认清单形式展示，由用户复核批量清理，严禁隐式静默删除。
- **元数据与客观度量**：
  - 严格以目录名作为唯一标准标识（符合 PRD 6.1.2 规范）；
  - 支持跨平台换行符（CRLF / LF）规范化，解析 YAML Frontmatter，容错提取名称与描述；缺失时自动截取首段非标题正文（≤300 字）作为摘要；
  - 过滤系统杂项与非文本二进制文件（通过探测前 1KB 避开乱码），精准统计 UTF-8 字符数与文件总数。

### 2. Windows 深度适配与多模式挂载
- **智能适配与降级引擎**：
  - **同卷（同盘符）**：优先创建相对软链接（`symlink`）；若捕获 Win32 1314（权限不足），自动触发降级引导，推荐 **NTFS 绝对 Junction** 或 **Copy 独立副本**；
  - **跨卷（跨物理盘符）**：识别跨卷语义限制，直接推荐 **NTFS 绝对 Junction**（实测支持跨卷且无须提权）或 **Copy 模式**。
- **两阶段安全原子事务**：
  - **Pre-flight 预检**：挂载前先检测目标路径占用情况与系统权限；
  - **安全备份**：检测到目标冲突且用户选择“备份后替换”时，先将原对象原子重命名至 `.agents/skills/.backup/<timestamp>_<name>`；
  - **逆向恢复**：挂载过程若抛错，立即将备份原路移回，确保项目文件系统零损坏。
- **严密卸载分流**：
  - Reparse Point（软链接 / Junction）校验元数据后调用 `remove_dir`；
  - Copy 副本目录在经过严格 **Jail Check**（确保位于项目 `.agents/skills` 之内）后方调用 `remove_dir_all`，严防误删外部文件。

### 3. 双层解耦状态诊断机
彻底解决“未托管目录遮蔽系统异常”的死锁问题：
- **第一层（数据库登记挂载项）**：诊断 `NORMAL`、`BROKEN`（断链）、`WRONG_TARGET`（目标指向错误）、`CONFLICT`（被普通文件占用）、`PERMISSION_DENIED`；
- **第二层（物理文件系统发现）**：遍历 `.agents/skills`，将非 `.backup` 且未在系统登记的实体标记为 `UNMANAGED`（未托管）；
- **Copy 模式版本陈旧度感知**：比对原件与副本的 `content_hash`，原件更新时动态标记为黄色“副本陈旧”徽标，并提供一键覆盖同步。

### 4. Agent 预设与入口链接独立管理
- 目标项目以 `.agents/skills` 作为唯一物理挂载目录；
- 支持为 **Claude Code**、**Codex** 及自定义 Agent 独立创建入口软链接（如 `<project>/.claude/skills` -> `<project>/.agents/skills`）；
- 入口链接生命周期与物理挂载完全解耦，移除入口链接绝不影响物理 Skill。

### 5. 跨机器配置导入导出与路径重映射
- 导出标准化结构 JSON 文件，固化仓库、项目、Agent 目标与挂载关系；
- 导入时提供预览面板与预检，支持**中央仓库路径**与**目标项目路径**的双向重映射；路径不存在时标红提醒，映射确认后批量恢复所有配置与链接。

### 6. 审计日志与一键批次回滚
- 所有关键操作（挂载、卸载、修复、导入、Git 更新）统一录入 SQLite `operation_logs`；
- 每次批量挂载生成唯一 `batch_id`，底部状态栏提供常驻“回滚上一批次”按钮，可反向递归清除挂载并复原备份。

---

## 🛠️ 技术栈

| 层次 | 选型 | 说明 |
|---|---|---|
| **桌面运行时** | Tauri v2 (`@tauri-apps/cli 2.x`, `tauri 2.x`) | 极小内存占用，原生系统调用能力 |
| **Tauri 官方插件** | `@tauri-apps/plugin-dialog`, `@tauri-apps/plugin-opener` | 本地原生目录选取、在系统文件管理器中打开 |
| **后端核心** | Rust 2021 Edition | 内存安全、零成本抽象 |
| **嵌入式数据库** | `rusqlite` (`features = ["bundled"]`) | 内置 SQLite 3，无须外部安装数据库环境 |
| **Windows 链接** | `junction` (v2.0), `dunce`, `pathdiff` | 原生 NTFS 联接点生命周期管理，UNC 路径消除与相对寻址计算 |
| **前端界面** | React 19, TypeScript, Vite | 现代响应式单页架构 |
| **样式与图标** | Tailwind CSS, Lucide React | 极简、现代、高信息密度三栏看板 |

---

## 📂 项目结构

```text
skills-manager/
├── docs/
│   └── Skills 管理工具 PRD.md    # 业务规格与技术要求 (V1.1 确认稿)
├── src-tauri/
│   ├── Cargo.toml                # Rust 依赖声明
│   ├── tauri.conf.json           # Tauri 应用配置
│   ├── capabilities/
│   │   └── default.json          # 原生系统权限配置
│   ├── src/
│   │   ├── main.rs               # 应用主入口
│   │   ├── lib.rs                # Tauri Builder 注册与状态初始化
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs         # 8 张核心数据表 DDL 与迁移
│   │   │   └── repository.rs     # SQLite 事务抽象层
│   │   ├── core/
│   │   │   ├── scanner.rs        # Skill 识别、Frontmatter 解析与字符度量
│   │   │   ├── link_engine.rs    # NTFS Junction/软链接/Copy 与两阶段原子替换
│   │   │   ├── status_checker.rs # 双层解耦 7 状态诊断引擎
│   │   │   ├── git_service.rs    # Git 进程隔离、diff 变更感知与脏拦截
│   │   │   ├── entry_engine.rs   # Agent 入口链接生命周期管理
│   │   │   └── config_io.rs      # 配置 JSON 导出、导入预检与路径映射
│   │   ├── commands/             # Tauri IPC 接口分发层
│   │   │   ├── repo_cmd.rs
│   │   │   ├── project_cmd.rs
│   │   │   ├── mount_cmd.rs
│   │   │   ├── entry_cmd.rs
│   │   │   └── log_cmd.rs
│   │   └── models/
│   │       ├── error.rs          # 统一 AppError 与 10 大 PRD 错误码映射
│   │       └── dto.rs            # 前后端通信契约
│   └── tests/
│       └── test_full_suite.rs    # 端到端集成测试集
├── src/                          # 前端 UI (React 19 + TypeScript)
│   ├── components/
│   │   ├── layout/
│   │   │   ├── TopToolbar.tsx    # 顶栏操作区
│   │   │   ├── LeftSidebar.tsx   # 中央仓库卡片与项目列表
│   │   │   ├── MiddleSkills.tsx  # Skill 检索过滤与卡片流
│   │   │   ├── RightDetails.tsx  # 项目挂载看板 / Skill 详情看板双模切换
│   │   │   └── BottomStatusBar.tsx # 批次操作状态与回滚按钮
│   │   └── modals/               # 6 大交互弹窗
│   │       ├── ConflictDialog.tsx     # 冲突处理
│   │       ├── DegradationDialog.tsx  # 降级选择引导
│   │       ├── GitPullConfirmModal.tsx# 脏工作区提示 & 变更复核
│   │       ├── ImportPreviewModal.tsx # 导入路径重映射
│   │       ├── OnboardingWizard.tsx   # 首次引导向导
│   │       └── OperationLogDrawer.tsx # 审计日志抽屉
│   ├── services/                 # Tauri IPC 强类型封装
│   ├── types/                    # TypeScript 数据模型
│   ├── App.tsx                   # 主视图编排
│   └── main.tsx                  # 前端渲染入口
├── package.json
└── vite.config.ts
```

---

## 🚀 快速开始

### 1. 环境准备
- **操作系统**：Windows 10 / 11（推荐，全功能支持）；亦兼容 macOS / Linux。
- **Node.js**：`v18+`（推荐 Node `v20+` / `v24`）
- **包管理器**：`pnpm`（推荐）或 `npm`
- **Rust**：`1.77+`（需已安装 `cargo` 工具链）
- **Git**：`2.30+`（已添加至系统 `PATH`）

### 2. 安装依赖
```powershell
# 安装前端依赖
pnpm install
```

### 3. 开发环境运行
```powershell
# 启动 Tauri 桌面开发窗口（自动启动 Vite 前端与 Rust 编译）
pnpm tauri dev
```

### 4. 自动化测试
系统内置涵盖 13 项单元与端到端集成测试，严格验证核心文件系统逻辑：
```powershell
# 运行后端全量测试
cd src-tauri
cargo test -- --nocapture

# 运行前端类型检查与打包验证
cd ..
npx tsc --noEmit
npx vite build
```

---

## 📖 核心操作流程指南

```text
┌─────────────────────────────────────────────────────────────┐
│ 1. 首次启动向导 (OnboardingWizard)                          │
│    └─ 设定中央仓库（本地 skills/ 目录或输入 Git URL）        │
│    └─ 登记第一个项目目录（自动初始化 .agents/skills）        │
├─────────────────────────────────────────────────────────────┤
│ 2. 浏览与挂载 (MiddleSkills -> RightDetails)                │
│    └─ 中栏勾选所需 Skills，点击“批量挂载”                   │
│    └─ 系统自动预检（检测权限、同卷/跨卷、冲突路径）         │
│    └─ 若遇权限不足，自动引导降级为 Windows Junction         │
│    └─ 若遇文件冲突，由用户选择“备份后替换”或“跳过”         │
├─────────────────────────────────────────────────────────────┤
│ 3. Agent 入口链接配置 (RightDetails)                        │
│    └─ 针对 Claude Code 勾选一键创建 .claude/skills 入口链接  │
├─────────────────────────────────────────────────────────────┤
│ 4. Git 同步与变动清理 (TopToolbar)                          │
│    └─ 点击“Git 拉取”，拦截脏工作区                          │
│    └─ 自动比对 diff，弹窗提示已失效 Skill，勾选批量清理      │
├─────────────────────────────────────────────────────────────┤
│ 5. 跨机器迁移 (TopToolbar -> Settings)                     │
│    └─ 导出 JSON 配置文件并在新机器导入                      │
│    └─ 预览面板标红缺失路径，手动重定向后一键恢复全部挂载    │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔒 安全边界与设计原则

1. **Jail 路径安全检验**：应用执行写操作或递归删除操作时，严格限制在项目 `.agents/skills` 及其内部备份目录范围，绝不触碰项目源代码与宿主系统关键路径。
2. **中央仓库只读防护**：中央仓库为权威只读原件池，除执行 `git pull` 同步外，系统严禁对其执行任何写入或删除操作。
3. **禁止静默覆盖**：当挂载路径已被普通文件或未知目录占用时，必须弹出提示由用户显式确认，默认移入 `.backup`，严禁直接覆盖导致代码丢失。
4. **两阶段原子回滚**：批量挂载失败或用户主动点击“回滚”时，基于操作日志逆向恢复文件系统状态，杜绝残留孤儿软链接与状态脱节。
