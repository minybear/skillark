# 20260726 agent M1 Agent 探测 POC

## 1. 背景/问题现象

M0 仅冻结了 Agent DTO 与 Adapter 合约，应用还不能在 Windows 上发现本机 Agent。M1 需要在
不依赖单一信号、不因某个 Agent 未安装而报错的前提下，发现 Claude Code、Cursor、Codex 和
WorkBuddy，并向前端返回冻结的 `agentType` camelCase DTO。

## 2. 方案/根因

- 实现 `DetectionContext`，采集用户目录、环境目录、PATH、运行进程与人工 Skill 路径覆盖。
- 建立四个内置 Agent Adapter 注册表，按 CLI 40、配置目录 25、Skill 目录 25、运行进程 10
  的权重评分。
- 人工路径以 `user_override` 信号最高优先，并保留中文路径。
- 使用 Windows ToolHelp 原生进程快照，避免启动外部 Shell。
- 扫描放入 Tauri blocking task，并提供 `cancel_agent_discovery` 命令。
- 规范化所有输出路径，移除 Windows `\\?\` 长路径前缀后再映射到 DTO。
- React 首页增加主动扫描、取消、Detected/Probable/Possible 分级与四 Agent 结果卡片。

## 3. 走过的弯路

1. 初版使用 `tasklist` 采集进程，真实测试耗时 6.10 秒，超过 3 秒预算；改为 ToolHelp 快照后，
   整个系统扫描降到 0.19 秒。
2. `Path::canonicalize` 在 Windows 返回 `\\?\` 前缀，导致 DTO 可读性和两条路径断言不一致；
   规范化函数现在同时处理本地盘与 UNC 前缀。
3. 初版将路线图中的 M1 标记完成，但 Custom Adapter、持久化人工校正和完整验证矩阵尚未完成；
   已恢复为“进行中”，避免把 POC 首批能力误报为里程碑关闭。

## 4. 效果

- Agent 单测 4/4 通过：多信号评分、人工覆盖、未安装降级、真实系统扫描性能。
- 冻结契约测试 4/4 通过，外部字段仍为 `agentType`，未泄漏 Domain 的 `kind`。
- 真实 Windows 扫描返回 4 个内置候选，总耗时 0.19 秒，低于 3 秒预算约 15 倍。
- 未安装 Agent 返回 0 置信度候选，不抛错。
- TypeScript 类型检查和 Vite 生产构建通过；桌面/窄屏预览均无横向溢出，运行时错误数为 0。
- 生产前端共 35 个模块，JavaScript gzip 62.72 kB。

## 5. 经验沉淀

1. Agent 探测应把“可执行文件、配置、Skill 目录、运行状态”作为独立证据，UI 只解释结果，
   不持有路径规则。
2. Windows 性能预算内的系统扫描应优先使用原生 API，外部命令不仅慢，也带来编码和取消问题。
3. 路径规范化必须把 Windows 长路径前缀、UNC 与中文用户名纳入测试，不能只比较字符串拼接结果。
4. POC 的完成状态应按验收矩阵判断；代码可运行不等同于 Custom/人工校正/多实例均已验证。

## 6. 优化建议

1. 下一批补 Custom Adapter 与持久化人工设置，并验证“人工设置优先于自动探测”的重启行为。
2. 为四个内置 Agent 建立默认安装、自定义安装、只读目录、中文用户名和多实例夹具矩阵。
3. 将探测状态从首页 POC 区迁入独立 Agents 页面，并提供信号明细与路径校正表单。
4. M2 前补全 Adapter 路径规则的版本化说明，避免客户端升级后静默改变目标目录。
