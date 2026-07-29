# SkillArk v0.1 设计冻结记录

冻结日期：20260725  
状态：已冻结

## 冻结范围

本次冻结解决设计包中会影响 DTO、状态机、数据库和首期范围的五项差异。其他未经过 POC 验证的路径规则、
性能指标和 UI 细节仍保持“待验证”，不等同于实现完成。

## 冻结结论

| 项目 | 结论 | 落点 |
|---|---|---|
| 首期来源 | v0.1 仅本地目录与 ZIP；GitHub 延后到 v0.2 | PRD、Roadmap、ADR-008 |
| Agent 外部字段 | JSON/Tauri DTO 使用 `agentType`；Rust Domain 使用 `AgentKind` 并显式映射 | Agent Schema、ADR-010、Contract Test |
| 全局 Workspace | 固定 ID `global-default`；DeploymentPlan 不允许空 workspaceId | Data Model、DeploymentPlan Schema、ADR-009 |
| 外部更新状态 | 保持 `synced`，变化原因放入 VerifyResult，不新增持久状态 | Deployment POC、ADR-011 |
| SQLite | Rust 后端使用 `sqlx 0.8` + SQLite + 内嵌 migrations | ADR-012、`src-tauri/migrations/` |

## 工程约束

1. Domain Core 不依赖 Tauri、数据库、文件系统和网络。
2. 前端不得获得通用 SQL 执行权限。
3. 所有跨边界 DTO 使用 camelCase，并接受 JSON Schema 契约测试。
4. SQL migration 使用 LF，避免 Windows 与 CI 的 migration Hash 不一致。
5. 安装仍遵守 Plan → Execute，执行器不接受自由文本 Shell。

## 验证

- `src-tauri/tests/contracts.rs` 覆盖三个冻结 Schema。
- `src-tauri/migrations/0001_init.sql` 是初始数据模型的可执行副本。
- Rust 工具链就绪后运行 `cargo test --manifest-path src-tauri/Cargo.toml`。
