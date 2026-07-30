# SkillArk v0.3 架构详细设计

## 1. 架构目标

把多个外部目录变成统一、可降级、可离线的本地 Registry。

## 2. 架构约束

- Domain Core 不依赖 Tauri、UI、具体 Hub、具体模型或平台命令。
- 新模块通过 Application Service 与 Port 接入既有 Skill/Version/Operation。
- 文件、网络、模型、扩展和环境操作都必须可取消并有资源预算。
- 任何远端或概率结果进入数据库前必须带来源和证据版本。

## 3. 组件关系

```mermaid
flowchart TB
  UI[React Feature UI] --> CMD[Tauri Commands]
  CMD --> APP[Application Services]
  APP --> CORE[SkillArk Domain Core]
  ConnectorHost[ConnectorHost] --> CORE
  RegistryNormalizer[RegistryNormalizer] --> CORE
  SyncScheduler[SyncScheduler] --> CORE
  SearchIndex[SearchIndex] --> CORE
  DedupeEngine[DedupeEngine] --> CORE
  CacheManager[CacheManager] --> CORE
  CORE --> REPO[Repositories]
  CORE --> OPS[Operation Plan/Execute/Verify/Rollback]
```

## 4. 模块边界

### ConnectorHost

**职责：** 统一调度 Connector，隔离认证、分页、游标和速率限制。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### RegistryNormalizer

**职责：** 把来源字段映射到最小公共模型，同时保留 raw_metadata。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### SyncScheduler

**职责：** 按来源独立增量同步，支持游标、退避、中断恢复和手动刷新。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### SearchIndex

**职责：** SQLite FTS5 + 结构化筛选，结果与来源状态解耦。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### DedupeEngine

**职责：** 通过 canonical repository、content hash、fork relation 和相似度建立候选簇。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### CacheManager

**职责：** 元数据、预览、包缓存三级存储与配额淘汰。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

## 5. 推荐目录

```text
src-tauri/src/
├── domain/
├── application/arkhub/
├── ports/
├── adapters/
├── infrastructure/
├── commands/
└── migrations/

src/features/arkhub/
├── api/
├── components/
├── pages/
├── state/
└── tests/
```

## 6. 应用服务

- `PlanV0.3ArkhubService`：生成纯计划和警告，不产生外部副作用。
- `ExecuteV0.3ArkhubService`：只接受计划 ID，重新校验前置条件。
- `VerifyV0.3ArkhubService`：用 Hash、revision、证据或实际目标验证结果。
- `RollbackV0.3ArkhubService`：使用补偿记录恢复前一可用状态。
- `DiagnoseV0.3ArkhubService`：返回结构化原因、证据和建议动作。

## 7. 状态机

```mermaid
stateDiagram-v2
  [*] --> planned
  planned --> running
  planned --> cancelled
  running --> verifying
  running --> failed
  verifying --> succeeded
  verifying --> failed
  failed --> rolling_back
  rolling_back --> rolled_back
  rolling_back --> recovery_required
  succeeded --> stale
  stale --> planned
```

## 8. 错误模型

每个错误必须包含：

```json
{
  "code": "STABLE_MACHINE_CODE",
  "stage": "plan|fetch|parse|scan|execute|verify|rollback",
  "retryable": false,
  "user_action": "可执行的下一步",
  "evidence": [],
  "cause_chain": []
}
```

禁止把原始异常文本作为唯一 UI 信息。

## 9. 资源与并发

- 网络、扫描、Hash、索引和环境操作放入后台任务队列。
- 同一 SkillVersion/目标路径的写操作串行化。
- 不同来源、不同环境、不同目标可并行，但结果独立提交。
- 所有任务支持取消；取消后不得把临时状态标为成功。

## 10. 安全边界

- 所有路径 canonicalize 后再比较允许根目录。
- 外部包、扩展、模型输出默认不可信。
- 凭据只通过引用传递，不进入 DTO、日志或诊断正文。
- 临时目录、缓存与派生版本使用内容 Hash 验证。
