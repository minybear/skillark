# skillark · 设计规范索引

> 长期稳定的产品与技术规范，约束“怎么做”。`v0.1/` 保存本次设计包的正式归档副本，仓库根目录的解压包保留为原始输入。

| 文档 | 状态 | 负责人 |
|---|---|---|
| [v0.1 设计包说明](v0.1/README.md) | 当前 | 待指定 |
| [01 产品需求](v0.1/01-PRD.md) | 已冻结 | 产品 |
| [02 总体架构](v0.1/02-ARCHITECTURE.md) | 已冻结 | 架构 |
| [03 数据模型](v0.1/03-DATA-MODEL.md) | 已冻结 | 后端 |
| [04 Agent 探测 POC](v0.1/04-AGENT-DETECTION-POC.md) | 待验证 | 后端 |
| [05 Skill 分发 POC](v0.1/05-DEPLOYMENT-POC.md) | 待验证 | 后端 |
| [06 UI 信息架构](v0.1/06-UI-IA.md) | 待评审 | 前端/产品 |
| [07 测试计划](v0.1/07-TEST-PLAN.md) | 待评审 | 测试 |
| [08 Roadmap](v0.1/08-ROADMAP.md) | 当前 | 项目 |
| [09 ADR 摘要](v0.1/09-DECISIONS.md) | 当前 | 架构 |
| [10 v0.1 设计冻结记录](v0.1/10-ADR-v0.1-FREEZE.md) | 已冻结 | 架构 |
| [JSON Contracts](v0.1/contracts/) | 已冻结 | 后端/前端 |
| [Rust 接口示例](v0.1/examples/) | 参考 | 后端 |
| [SQLite 初始模型](v0.1/sql/0001_init.sql) | 已迁移 | 后端 |

## 当前基线

- 桌面框架：Tauri 2
- 前端：React + TypeScript
- 后端：Rust
- 本地存储：SQLite
- 首发系统：Windows
- v0.1 来源：本地目录、ZIP
- v0.1 Agent：Claude Code、Cursor、Codex、WorkBuddy、Custom
- 默认分发：Copy；Junction 为高级选项

设计冻结结论见 [v0.1 冻结记录](v0.1/10-ADR-v0.1-FREEZE.md)。Rust/MSVC 工具链问题已
[关闭](../issues/closed/ISSUE-20260725-devex-rust-msvc-toolchain-missing.md)，当前进入 M1 Agent 探测 POC。
