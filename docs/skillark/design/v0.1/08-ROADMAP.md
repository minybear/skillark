# SkillArk 开发路线与 Sprint

## 阶段 0：设计冻结（2–3 天）

交付：

- PRD 评审完成
- 数据模型冻结
- Adapter 和 Driver 接口冻结
- POC 测试目录准备完成

退出条件：v0.1 不再增加 Hub、AI、云同步功能。

## Sprint 1：Agent 探测 POC（5 个工作日）

### Day 1

- 建立 domain、application、ports 基础目录
- 定义 Agent、AgentCandidate、DetectionSignal
- 实现 DetectionContext

### Day 2

- 实现 PATH、目录、进程信号采集
- 实现 Adapter 注册表

### Day 3

- Claude Code、Cursor Adapter 原型
- 手动配置和验证接口

### Day 4

- Codex、WorkBuddy、Custom Adapter 原型
- 扫描超时与取消

### Day 5

- POC 页面或 CLI 输出
- 在真实 Windows 环境跑验证矩阵
- 整理不确定路径，转为可配置规则

Sprint 1 验收：至少三个 Agent 能得到可确认的 Skill 目录。

## Sprint 2：导入与分发 POC（5 个工作日）

### Day 1

- SKILL.md 解析器
- 目录 Hash
- 本地目录导入

### Day 2

- SQLite 初始迁移和 repositories
- Skill、SkillVersion 持久化

### Day 3

- Copy Driver
- 冲突分类
- 临时目录与原子替换

### Day 4

- Junction Driver
- Verification Service
- Uninstall Service

### Day 5

- 多目标执行
- 操作日志
- 故障注入测试

Sprint 2 验收：一个 Skill 可分发到至少三个测试目标并正确验证、卸载。

## Sprint 3：完整 MVP UI（8–10 个工作日）

- 首次启动向导
- Skill Library
- Agent 配置
- Workspace
- 分发向导
- 操作记录
- 错误恢复

## Sprint 4：v0.1 发布准备（5–8 个工作日）

- ZIP 导入与 Zip Slip 防护
- 数据备份与迁移
- 安装包和自动更新基础设施
- Windows 真机回归
- 用户文档

## v0.2

- GitHub 仓库和子目录链接导入
- Git commit 与更新检查
- 更新 Diff

## v0.3

- Connector 插件体系
- skills.sh 和 ClawHub
- 本地元数据索引与按需缓存

## v0.4

- 安全扫描
- 权限报告
- 风险等级

## v0.5

- AI 解析非标准来源
- 结构化安装建议
- 不执行未经确认的任意命令
