# SkillArk v0.7 架构详细设计

## 1. 架构目标

把操作系统/运行环境差异显式建模，而不是继续堆路径 if/else。

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
  EnvironmentRegistry[EnvironmentRegistry] --> CORE
  EnvironmentAdapter[EnvironmentAdapter] --> CORE
  WSLAdapter[WSLAdapter] --> CORE
  PathMapper[PathMapper] --> CORE
  MacOSAdapter[MacOSAdapter] --> CORE
  CrossEnvironmentCoordinator[CrossEnvironmentCoordinator] --> CORE
  CORE --> REPO[Repositories]
  CORE --> OPS[Operation Plan/Execute/Verify/Rollback]
```

## 4. 模块边界

### EnvironmentRegistry

**职责：** 保存环境类型、身份、状态、能力和生命周期。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### EnvironmentAdapter

**职责：** 统一命令、文件、路径、权限和原子替换接口。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### WSLAdapter

**职责：** 枚举发行版，通过 wsl.exe 调用受控 helper，避免直接猜 Linux 语义。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### PathMapper

**职责：** Windows、UNC、WSL 与 POSIX 路径的显式转换和验证。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### MacOSAdapter

**职责：** 处理 POSIX 文件操作、Symlink、Keychain 和平台目录。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

### CrossEnvironmentCoordinator

**职责：** 为每个环境生成独立子计划，聚合但不共享事务。

**输入约束：** 只接受规范化 DTO 或 ID，不接收未经解析的任意字符串作为副作用参数。

**输出约束：** 返回结构化结果、证据、警告和可恢复错误；不得吞掉部分失败。

## 5. 推荐目录

```text
src-tauri/src/
├── domain/
├── application/multi_environment/
├── ports/
├── adapters/
├── infrastructure/
├── commands/
└── migrations/

src/features/multi-environment/
├── api/
├── components/
├── pages/
├── state/
└── tests/
```

## 6. 应用服务

- `PlanV0.7MultiEnvironmentService`：生成纯计划和警告，不产生外部副作用。
- `ExecuteV0.7MultiEnvironmentService`：只接受计划 ID，重新校验前置条件。
- `VerifyV0.7MultiEnvironmentService`：用 Hash、revision、证据或实际目标验证结果。
- `RollbackV0.7MultiEnvironmentService`：使用补偿记录恢复前一可用状态。
- `DiagnoseV0.7MultiEnvironmentService`：返回结构化原因、证据和建议动作。

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
