# SkillArk v0.2 → v1.0 详细设计（归档 + 分析）

> 归档日期：20260730（`date +%Y%m%d`）
> 来源：`docs/skillark/SkillArk-v0.2-v1.0-detailed-design.zip`（解压后归档于此，原 zip 保留）
> 状态：**设计草案（Draft）**——按包内说明，开发某版本前只冻结该版 PRD/POC/ADR，不一次冻结到 v1.0 全部实现细节。
> 基线：v0.1 已发布（见 `../v0.1/` 与证据账本）。
> 包原始说明见 `PACK-README.md`。

## 一、这是什么

v0.1 之后的逐版本详细设计，共 **9 个版本**，每版保持与 v0.1 相同的 8 份文档层级
（PRD / ARCHITECTURE / DATA-MODEL / POC / UI-IA / TEST-PLAN / ROADMAP / DECISIONS）。

| 版本 | 代号 | 用户结果（一句话） |
|---|---|---|
| v0.2 | Link Bridge 链接安装版 | 粘贴 Git 仓库/子目录链接 → 预览候选 → 固定修订 → 导入 → 检查更新 → 安全回滚 |
| v0.3 | ArkHub 多 Hub 聚合版 | 单入口跨多来源搜索、识别重复与原始来源、按需缓存安装包 |
| v0.4 | ArkGuard 安全审计版 | 安装/更新前给文件级发现、权限清单、风险级别与 allow/confirm/block 决策 |
| v0.5 | ArkPilot AI 安装助手版 | 规则解析不了的来源由 AI 生成结构化候选/计划，只执行通过 Schema+策略校验的步骤 |
| v0.6 | ArkCompat 兼容与可复现版 | 分发前兼容报告 + 派生版本转换 + manifest/lockfile 换机恢复相同 Hash |
| v0.7 | Multi-Environment 多环境版 | Windows/WSL/macOS Agent 独立发现、验证、分发，跨环境失败互不影响 |
| v0.8 | Extensions 扩展生态版 | 新 Hub/Agent/扫描规则经版本化、受限、可隔离的扩展接入 |
| v0.9 | Public Beta 公开测试版 | 10 分钟首值、升级/故障不丢数据、脱敏诊断定位问题 |
| v1.0 | Stable 稳定正式版 | Schema/Lockfile/扩展 API/支持矩阵/发布流程冻结稳定 |

## 二、目录结构（已对齐 v0.1 约定）

```
v0.2-v1.0/
├── README.md                      ← 本文件（归档说明 + 分析）
├── PACK-README.md                 ← 原 zip 自带 README（未改）
├── MASTER-DETAILED-DESIGN.md      ← 总设计：方法论 + 9 版摘要 + 8 条跨版本不变量
├── FILE-INVENTORY.txt             ← 原始文件清单
├── 00-SHARED/                     ← 跨版本共享方法论
│   ├── DESIGN-METHOD.md           ← 第一性原理 / F·A·U·H 编号 / MECE / 逆向 / 可证伪
│   ├── FACTS-ASSUMPTIONS-REGISTER.md
│   ├── CROSS-VERSION-TRACEABILITY.md  ← 需求-证据-验收追踪矩阵
│   └── RELEASE-GATES.md           ← 设计/工程/产品/发布回滚 四级门禁
├── v0.2-link-bridge/ … v1.0-stable/   ← 每版 8 份文档
├── sql/                           ← 0002~0010 增量迁移草案（对齐 v0.1 的 sql/）
└── contracts/                     ← 9 个 JSON Schema 草案（对齐 v0.1 的 contracts/）
```

> 对齐说明：源包把这两类放在 `design/sql/`、`design/schemas/`；为与 v0.1 的
> `v0.1/sql/`、`v0.1/contracts/` 命名一致，归档时改放 `sql/` 与 `contracts/`，内容未改。

## 三、设计方法论（贯穿全部版本，值得沉淀为项目规范）

- **第一性原理**：从不可约简的用户结果 + 不可违反约束推导，不复制既有方案。
- **事实/假设分离**：统一 `F-版本-序号` / `A-` / `U-` / `H-` 编号 + 证据等级 E0–E4（事实须 ≥E2，外部平台能力优先 E3）。
- **MECE 八维**：输入 / 解析 / 信任 / 存储 / 执行 / 观察 / 恢复 / 治理。
- **逆向思维**：测试文档先列失败方式与恢复策略。
- **可证伪**：关键结论（H）含样本、指标、阈值、截止点与失败决策。

这与本项目 v0.1 的「目标 DAG + 证据账本 + 失败扩图」方法一脉相承，可直接复用为各版本的开发门禁。

## 四、跨版本不变量（8 条，任何版本不得违反）

1. 中央 Library 是受管 Skill 主副本；目标目录不是隐式真源。
2. 所有写操作先 Plan 后 Execute，并落操作日志。
3. 原始版本 / 派生版本 / 部署快照分离。
4. AI 只生成结构化建议，不绕过策略层直接执行 Shell 或写文件。
5. 未受管目录不得静默覆盖；用户修改不得静默丢失。
6. 外部来源保留来源身份、版本/Commit、内容 Hash。
7. 安全判断可解释；「未发现风险」≠「绝对安全」。
8. 数据库迁移与发布更新须先备份且有恢复路径。

> 与 v0.1 已落地的不变量（中央 vault 不可变、Plan/Execute、Operation 审计、安全卸载 force 门）完全一致。

## 五、归档时发现的冲突 / 注意事项（重要）

1. **`sources` 表结构与 v0.1 已发布库不兼容（需手工迁移设计）**
   - v0.1 已上线 `../v0.1/sql/0001_init.sql` 的 `sources`：含 `source_type / display_name / base_url / enabled / config_json`。
   - v0.2 草案 `sql/0002_v0_2_link_bridge.sql` 的 `sources`：`id / schema_version / status / raw_json / created_at / updated_at` 的**通用骨架**，无 v0.1 那些列。
   - 包内 SQL 自注：「design skeleton… must be finalized against the implemented domain model」。
   - **结论**：v0.2 落地时**不能直接套该草案**，须对 v0.1 真实表做 `ALTER`/迁移设计，把 link-bridge 需要的来源字段（remote/subpath/revision 等）叠加到既有 `sources` 或新建关联表。已记入 issue 跟进：
     `../../issues/open/ISSUE-20260730-v0.2-sources-migration-conflict.md`。

2. **所有 SQL 与 JSON Schema 均为草案**：编码前须对照已实现 Domain 类型完成字段与外键冻结（包内明确要求）。

3. **版本依赖链**：v0.2 是后续多数版本的前置（来源/修订抽象被 v0.3 聚合、v0.4 审计、v0.5 AI、v0.6 lockfile 复用），建议按 v0.2→…→v1.0 顺序推进，不跳版。

## 六、下一步（建议）

- 为 **v0.2 Link Bridge** 单独建 plan（`docs/skillark/plan/{date}-v0.2-link-bridge/`），
  先冻结该版 PRD/POC/ADR，并优先解决上面第 1 条 `sources` 迁移冲突。
- 每版开发前将对应 H 假设转入 POC 实验，失败即执行文档内失败决策并更新 `00-SHARED/FACTS-ASSUMPTIONS-REGISTER.md`。
