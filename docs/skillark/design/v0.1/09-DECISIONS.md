# 已确定的架构决策（ADR 摘要）

## ADR-001：中央仓库是唯一受管源

Agent 目标目录不是主副本。Copy 模式的目标是快照，Junction 模式指向中央仓库。

## ADR-002：默认使用 Copy

Windows 首期默认 Copy，Junction 作为高级选项，降低权限和跨环境问题。

## ADR-003：安装必须先生成计划

UI 与 CLI 都先调用 Plan，再调用 Execute。执行器不接受自由文本命令。

## ADR-004：Agent 路径必须可配置

任何默认路径都只是候选。用户配置具有最高优先级。

## ADR-005：v0.1 只管理 Agent Skills 兼容包

WorkBuddy 等非完全一致格式先通过 Adapter 验证，不在 v0.1 承诺自动转换。

## ADR-006：内容版本使用 Hash，不依赖 mtime

修改时间只用于 UI 提示，不作为一致性依据。

## ADR-007：数据库和文件系统采用补偿式事务

所有文件写入必须通过临时路径、校验、原子替换和备份恢复完成。

## ADR-008：v0.1 Skill 来源只包含本地目录和 ZIP

GitHub 仓库或子目录链接延后到 v0.2。v0.1 可以保留 Source Adapter 扩展点，但不实现、不展示入口。

## ADR-009：全局部署使用固定 Workspace

应用首次初始化时创建稳定 ID 为 `global-default` 的 Global Workspace。Deployment 与 DeploymentPlan 的
`workspace_id` 始终非空。

## ADR-010：外部 Agent 字段统一为 agentType

Rust Domain 可使用 `AgentKind`，跨进程和持久化 JSON 使用 camelCase DTO 的 `agentType`，并由 JSON Schema
契约测试约束。

## ADR-011：不持久化 synced_after_external_update

目标 Hash 等于 Library 当前版本时状态为 `synced`。若它与部署时 Hash 不同，在 VerifyResult 中附加版本变化提示，
避免扩大持久状态机。

## ADR-012：SQLite 由 Rust 后端通过 sqlx 管理

v0.1 使用 `sqlx 0.8`、SQLite 和内嵌 migrations。前端不直接获得通用 SQL 执行权限，所有数据访问通过
Application Service 与 Repository Port。
