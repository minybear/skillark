# SkillArk v0.2-v1.0 详细设计包

> 生成日期：2026-07-30  |  基线：SkillArk v0.1 设计包

本包为 v0.2 至 v1.0 的逐版本详细设计。每个版本保持与 v0.1 相同的设计层级：

1. PRD 与范围
2. 架构
3. 数据模型
4. POC/证伪实验
5. UI 信息架构
6. 测试计划
7. 实施路线
8. ADR 决策

## 方法约束

- 第一性原理：先定义不可约简的用户结果和不可违反约束。
- 事实/假设分离：使用 F/A/U/H 编号与证据等级。
- MECE：按输入、解析、信任、存储、执行、观察、恢复、治理检查。
- 逆向思维：测试文档先列失败方式和恢复策略。
- 可证伪：关键结论包含样本、指标、阈值与失败决策。

## 目录

```text
00-SHARED/
  DESIGN-METHOD.md
  FACTS-ASSUMPTIONS-REGISTER.md
  CROSS-VERSION-TRACEABILITY.md
  RELEASE-GATES.md
v0.2-link-bridge/ ... v1.0-stable/
  01-PRD.md
  02-ARCHITECTURE.md
  03-DATA-MODEL.md
  04-POC.md
  05-UI-IA.md
  06-TEST-PLAN.md
  07-ROADMAP.md
  08-DECISIONS.md
design/sql/
design/schemas/
MASTER-DETAILED-DESIGN.md
```

## 使用方式

- 先阅读 `00-SHARED/DESIGN-METHOD.md`。
- 开发某一版本前，只冻结该版本的 PRD、POC 和 ADR，不要一次冻结 v1.0 全部实现细节。
- POC 失败时必须执行文档中的失败决策，并更新 Facts/Assumptions Register。
- SQL 与 JSON Schema 是设计草案，编码前需对照实际 Domain 类型完成字段和外键冻结。
