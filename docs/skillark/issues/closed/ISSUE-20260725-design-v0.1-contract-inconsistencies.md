# ISSUE-20260725-design-v0.1-contract-inconsistencies

状态：closed  
优先级：高  
登记日期：20260725

## 现象

v0.1 设计包已经覆盖产品、架构、数据、契约、POC 与测试，但存在会直接影响实现的跨文档不一致：

1. `design/v0.1/README.md` 将 GitHub 仓库或子目录链接列入“首期来源”，PRD 仅列本地目录和 ZIP，Roadmap 将 GitHub 放到 v0.2。
2. `agent-candidate.schema.json` 使用 `agentType`，Rust 示例和 Agent 探测 POC 输出使用 `kind`。
3. `deployment-plan.schema.json` 要求 `workspaceId` 为字符串，架构示例使用 `null`；数据模型建议创建固定 Global Workspace。
4. 分发 POC 使用 `synced_after_external_update`，PRD 的 Deployment 状态集合没有该状态。
5. SQLite 表结构已给出，但正式 Rust SQLite 库与 migration 执行方式尚未确定。

如果不先冻结，前后端 DTO、数据库 migration、状态机和测试夹具会出现重复返工。

## 复现

逐项比对以下设计文件中的字段、范围和状态定义即可复现：

- `docs/skillark/design/v0.1/01-PRD.md`
- `docs/skillark/design/v0.1/02-ARCHITECTURE.md`
- `docs/skillark/design/v0.1/03-DATA-MODEL.md`
- `docs/skillark/design/v0.1/04-AGENT-DETECTION-POC.md`
- `docs/skillark/design/v0.1/05-DEPLOYMENT-POC.md`
- `docs/skillark/design/v0.1/08-ROADMAP.md`
- `docs/skillark/design/v0.1/contracts/*.schema.json`

## 相关位置

- 产品边界：`docs/skillark/design/v0.1/01-PRD.md`
- 架构 JSON 示例：`docs/skillark/design/v0.1/02-ARCHITECTURE.md`
- Workspace 建议：`docs/skillark/design/v0.1/03-DATA-MODEL.md`
- 对外契约：`docs/skillark/design/v0.1/contracts/`
- Rust 参考接口：`docs/skillark/design/v0.1/examples/`
- 初始 SQL：`docs/skillark/design/v0.1/sql/0001_init.sql`

## 关联文档

- [v0.1 项目启动需求分析](../../plan/20260725-bootstrap-v0.1/01-需求分析.md)
- [v0.1 项目启动方案设计](../../plan/20260725-bootstrap-v0.1/02-方案设计.md)

## 约束/边界

- v0.1 不扩大到 Hub、云同步、AI 任意命令执行、macOS/Linux。
- 执行器只接受结构化安装计划，不接受自由文本 Shell。
- 用户手动配置的 Agent 路径优先于自动检测。
- 默认 Copy，Junction 仅作为高级选项。
- 合约冻结后再开始依赖这些字段的持久化和 UI 开发。

## 初步分析

建议以 PRD、ADR 与 Roadmap 作为范围权威，以 JSON Schema 作为进程边界的序列化权威：

- GitHub 延后到 v0.2。
- JSON 对外字段统一为 `agentType`，Rust 内部可保留类型安全的 `kind`，但必须显式映射。
- v0.1 创建固定 Global Workspace，使 `workspaceId` 始终非空。
- 不新增 `synced_after_external_update` 持久状态；映射为 `synced` 并在验证结果中附带来源变化提示。
- SQLite 优先验证 `sqlx + SQLite migrations`，若 POC 暴露明显复杂度再退回 `rusqlite`。

关闭条件：上述五项进入正式 ADR/Schema，契约校验测试通过，并在本 issue 中链接对应 worklog。

→ 已解决：五项结论已进入 ADR-008～012、冻结 Schema 和工程契约测试，见
[20260725 bootstrap M0 设计冻结与工程初始化](../../logs/202607/20260725-bootstrap-M0设计冻结与工程初始化.md)。
