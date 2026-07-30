# v0.2 Link Bridge（链接安装版）开发计划

> 建立日期：20260730  |  状态：开发中  |  详细设计：`docs/skillark/design/v0.2-v1.0/v0.2-link-bridge/` 与总 PRD `docs/skillark/design/SkillArk_V0.2-V1.0_PRD.md`
> 关联 issue：`docs/skillark/issues/open/ISSUE-20260730-v0.2-sources-migration-conflict.md`

## 1. 用户结果（不可约简）

用户粘贴 Git 仓库或子目录链接 → 预览候选 Skill → 固定来源修订 → 导入中央 Library → 可追溯。失败不破坏既有 Skill/部署/凭据。

## 2. 本轮冻结决策（20260730，所有者确认）

| 决策点 | 选择 | 理由 |
|---|---|---|
| Git 取数机制 | **git2 crate（libgit2）** | 自带 git 能力，不依赖用户装 git、不走子进程（避开 junction 的 EDR 子进程拦截前科）。嵌入式 git 标准做法。代价：原生依赖、包略大。 |
| 首增量范围 | **核心导入链路 L1–L5** | 先打通「粘贴链接即可安装」主路径；更新检查/Diff/LRU 缓存（L6）拆第二增量。P0 优先。 |
| `sources` 迁移冲突 | **复用 v0.1 sources + 新增关联表** | v0.1 `sources(source_type/display_name/base_url/config_json)` 足够通用；Git 来源用 `source_type='git'`、细节进 `config_json`，不重建表。新增 `source_revisions/repository_cache/update_checks`。 |

## 3. 目标 DAG（第一增量）

```
L0 设计冻结 + sources 迁移（0003）
├─ L1 LinkResolver：URL/子目录 → RepositoryLocator（纯函数，离线可测）
├─ L2 GitSourceAdapter：git2 浅克隆/归档 → 隔离目录（本地 git 夹具单测；真实 GitHub 待 POC）
├─ L3 RepositoryScanner：多 Skill 仓库 → 候选列表
├─ L4 溯源落库：source + resolved_revision + content_hash（接 import_scanned）
├─ L5 UI：粘贴链接 → 预览候选 → 导入
└─ L7 门禁：test/clippy/npm check + 真实 GitHub 拉取 POC（待网络）+ 归档
```

> L6 UpdateService/DiffService/缓存拆为第二增量，不在本轮。

## 4. 网络现实（关键风险）

本机对 GitHub 出网**不稳定**（推送/git ls-remote 反复超时）。影响：
- L2 真实 GitHub 拉取 POC 暂无法验，须等网络恢复。
- 开发策略：L1/L3 纯函数离线测；L2 用**本地 git 仓库夹具**（`git init` 出来的本地 remote）单测 git2 浅克隆逻辑，绕开公网；真实 GitHub 走 L7 POC 兜底。

## 5. 架构落点（复用 v0.1）

- `SkillSourceAdapter` port 已存在（`ports/mod.rs`）：`scan() -> ScannedSource { manifest, source_root }`。
- v0.2 `GitSourceAdapter` 实现 `SkillSourceAdapter`，fetch 后产出 `ScannedSource`，直接接 `ImportSkillService::import_scanned`（hash→vault 快照→落库）。
- `skill_versions.source_revision` 列已存在 → 存 resolved commit。

## 6. 证据账本（20260731 更新：增量 1 L0–L5 完成）

| 节点 | 要求 | 当前证据 | 结论 | 下一动作 |
|---|---|---|---|---|
| L0 | sources 迁移幂等+数据保留 | 0003 source_revisions + 2 集成测试幂等去重 | **passed** | — |
| L1 | 解析覆盖+0 路径越界 | link_bridge 14 单测（含 `..`/绝对/反斜杠拒绝） | **passed** | 200 真实样本（待网络随 L2） |
| L2 | git2 克隆+checkout+SHA | git2 编译过；本地 git 夹具 3 单测（默认/指定分支/roundtrip） | **passed(逻辑)** | 真实 GitHub POC 待网络（open issue） |
| L3 | 多 Skill 候选 | scan_repository 6 单测（根/多子目录/hint/`.git` 忽略/空/接 LocalDirectorySource） | **passed** | — |
| L4 | 溯源 revision/hash 落库 | SourceRepository + 2 集成测试（provenance + 幂等去重） | **passed** | — |
| L5 | 链接导入 UI 可用 | preview_link/import_link_candidate 命令 + LibraryPage UI + 1 前端测试 | **passed** | — |
| L7 | 门禁全绿 | cargo test 142过/0败/2ign；clippy 绿；npm check 绿；vitest 18过 | **passed(除真实 POC)** | GitHub 真实拉取 POC 待网络 |
| L6 | 更新/Diff/缓存 | —（第二增量） | deferred | 增量 2 |

> 门禁（20260731 真实复跑）：`cargo test --all-targets` **142 通过 / 0 失败 / 2 ignored**；`clippy -D warnings` 全绿；
> `npm run check` 绿；`vitest` **18/18**。2 ignored = junction（环境豁免）。
> 唯一未结：真实 GitHub HTTPS 克隆 POC 被本机网络阻塞（open issue），代码逻辑已用本地 git 夹具证明。
