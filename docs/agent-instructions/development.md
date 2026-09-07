# 开发与验证

适用范围：修改源码、运行程序、构建或测试时。

## 技术栈

- 桌面运行时：Tauri v2。
- 后端：Rust 2021 Edition、rusqlite（bundled SQLite）。
- 前端：React 19、TypeScript 5.x、Vite、Tailwind CSS。
- 系统集成：NTFS Junction/Symlink、Git CLI。

## 代码位置

- `src/`：前端 React/TypeScript 组件、服务和类型。
- `src-tauri/`：Rust 后端、Tauri IPC、数据库和核心引擎。
- `src-tauri/src/core/`：扫描、链接、状态诊断和 Git 服务。
- `src-tauri/src/commands/`：Tauri IPC 接口层。
- `src-tauri/src/db/`：SQLite schema 与事务抽象。
- `src-tauri/tests/`：Rust 集成测试。
- `docs/`：产品规格和项目文档。

## 命令

```powershell
# 前后端一体开发
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

## Rust 格式门禁

- Rust 代码必须通过 `cargo fmt`。
- 提交前运行 `cd src-tauri && cargo fmt --check`；不通过时禁止提交。
