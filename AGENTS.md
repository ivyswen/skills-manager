# Skills Manager

基于 Tauri v2 + Rust + React 19 + SQLite 的跨平台桌面客户端，为多 Agent 环境提供 Skills 集中管理与分发挂载。

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面运行时 | Tauri v2 |
| 后端 | Rust 2021 Edition, rusqlite (bundled SQLite) |
| 前端 | React 19, TypeScript 5.x, Vite, Tailwind CSS |
| 系统集成 | NTFS Junction/Symlink, Git CLI |

## 包管理器与命令

- 前端包管理器：`pnpm`（不要使用 npm 或 yarn）
- 后端包管理器：`cargo`

### 常用命令

```bash
# 开发环境启动（前后端一体）
pnpm tauri dev

# 前端构建
pnpm build

# 前端类型检查
npx tsc --noEmit

# 后端测试
cd src-tauri && cargo test -- --nocapture

# 生产构建
pnpm tauri build
```

## 项目结构

- `src/` — 前端 React 19 + TypeScript（组件、服务、类型）
- `src-tauri/` — Rust 后端（Tauri IPC 命令、核心引擎、数据库、模型）
- `src-tauri/src/core/` — 核心业务逻辑（扫描器、链接引擎、状态诊断、Git 服务）
- `src-tauri/src/commands/` — Tauri IPC 接口层
- `src-tauri/src/db/` — SQLite schema 与事务抽象
- `src-tauri/tests/` — Rust 集成测试
- `docs/` — PRD 规格文档

## 安全约束

- **Jail 路径检验**：写操作和递归删除严格限制在项目 `.agents/skills` 内，不触碰项目源码或系统路径。
- **中央仓库只读**：中央仓库为权威只读原件池，除 `git pull` 同步外禁止写入或删除。
- **禁止静默覆盖**：挂载路径被占用时必须弹窗确认，默认移入 `.backup`，禁止直接覆盖。
- **两阶段原子回滚**：批量操作失败时基于操作日志逆向恢复，杜绝残留孤儿链接。

## 编码约定

- 所有文件使用 UTF-8 编码。
- 代码注释和外部文档使用中文。
- Rust 代码必须通过 `cargo fmt`；提交前运行 `cd src-tauri && cargo fmt --check`，不通过则禁止提交。
