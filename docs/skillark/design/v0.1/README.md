# SkillArk Design Kit v0.1

这是一套可直接放入 SkillArk 仓库的产品与技术设计资料，覆盖项目初始化后的第 4 步及后续工作：

1. 产品需求与 MVP 边界
2. 总体架构与模块边界
3. 数据模型与 SQLite 初始迁移
4. Agent 探测技术验证设计
5. Skill 分发技术验证设计
6. UI 信息架构与页面状态
7. 测试计划与验收标准
8. 版本路线图与两周 Sprint 计划

## 建议放置方式

将本目录中的 `docs/` 合并到项目根目录的 `docs/skillark/`，将 `design/` 保留为设计参考。初始 SQL 可在确认所用 Rust SQLite 库后迁移到正式 migrations 目录。

## 当前假设

- 桌面框架：Tauri 2
- 前端：React + TypeScript
- 后端：Rust
- 本地存储：SQLite
- 首发系统：Windows
- 首期 Agent：Claude Code、Cursor、Codex、WorkBuddy、自定义 Agent
- 首期 Skill 来源：本地目录、ZIP
- GitHub 仓库或子目录链接：v0.2

## 立即执行顺序

1. 评审 `docs/01-PRD.md`，冻结 v0.1 范围。
2. 评审 `docs/03-DATA-MODEL.md` 和 `design/sql/0001_init.sql`。
3. 按 `docs/04-AGENT-DETECTION-POC.md` 完成 Agent 探测 POC。
4. 按 `docs/05-DEPLOYMENT-POC.md` 完成复制和 Junction 分发 POC。
5. POC 达标后再实现 `docs/06-UI-IA.md` 中的完整 UI。
