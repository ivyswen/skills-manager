# Skills Manager

基于 Tauri v2、Rust、React 19 和 SQLite 的跨平台桌面客户端，为多 Agent 环境集中管理、分发与挂载 Skills。

## 通用规则

- 所有文件使用 UTF-8 编码。
- 代码注释和外部文档使用中文。
- 前端只使用 `pnpm`，不要使用 npm 或 yarn；Rust 使用 `cargo`。

## 按需指引

- [开发与验证](docs/agent-instructions/development.md) — 修改源码、运行程序、构建或测试时阅读。
- [文件系统安全](docs/agent-instructions/filesystem-safety.md) — 修改扫描、挂载、卸载、同步、删除或回滚逻辑时阅读。
