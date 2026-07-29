# SkillArk v0.1 产品需求文档

## 1. 产品定位

SkillArk 是一个 Windows 本地优先的跨 Agent Skill 管理与分发工具。用户只需将 Skill 导入一次，即可将其分发到多个 AI Agent 的用户级目录或指定项目目录。

产品长期定位是：

> 聚合不同 Skill 来源，统一管理、审计、版本化，并安全分发到不同 AI Agent。

v0.1 不建设公共市场，不执行任意 AI 生成命令，优先验证本地中央仓库与多 Agent 分发价值。

## 2. 目标用户

### 核心用户

- 同时使用 Claude Code、Cursor、Codex、WorkBuddy 的开发者
- 在多个项目中重复安装相同 Skill 的开发者
- 需要集中管理团队或个人 Skill 资产的高级用户

### 非目标用户

- 只使用单个 Agent 且不使用项目级 Skill 的轻度用户
- 期待在线社区、付费市场或企业权限体系的用户

## 3. 核心问题

1. 同一个 Skill 需要在多个 Agent 中重复安装。
2. 同一个项目 Skill 需要在多个项目中重复复制。
3. Skill 的来源、版本和安装位置缺少统一记录。
4. Skill 更新后，用户无法快速判断哪些目标仍是旧版本。
5. Windows 下软链接权限、路径差异和手动复制容易出错。

## 4. v0.1 核心价值

> 导入一次，选择目标，一次分发；随时查看每个目标是否同步。

## 5. v0.1 功能范围

### 5.1 本地 Skill Library

- 导入包含 `SKILL.md` 的本地目录
- 导入 ZIP 文件
- 校验 Skill 基本格式
- 解析名称、描述、目录结构和内容 Hash
- 将 Skill 保存为中央仓库中的受管副本
- 支持查看文件树和 `SKILL.md`
- 支持删除本地 Skill

### 5.2 Agent 管理

- 自动扫描已知 Agent
- 展示检测结果、可执行文件位置和 Skill 目录
- 允许用户修正或手动填写目录
- 支持禁用某个 Agent
- 支持添加自定义 Agent

首期内置类型：

- Claude Code
- Cursor
- Codex
- WorkBuddy
- Custom

### 5.3 Workspace 管理

- 全局 Workspace：面向用户级 Skill
- 项目 Workspace：用户选择一个项目根目录
- 一个项目可选择一个或多个 Agent
- 记录项目移动、丢失和不可访问状态

### 5.4 分发

- 支持复制模式
- 支持 Windows Junction 模式
- 安装前检查目标冲突
- 安装成功后保存目标 Hash
- 支持卸载
- 支持重新分发
- 支持验证目标是否仍与中央仓库一致

### 5.5 操作与错误

- 所有写操作生成操作记录
- 失败时保留错误原因
- 分发采用临时目录 + 原子替换，避免半安装状态
- 覆盖现有目录前要求用户确认

## 6. v0.1 明确不做

- 多 Hub 聚合市场
- 全量离线缓存
- 云端账号与同步
- AI 自动执行安装命令
- Bash 到 PowerShell 自动转换
- Skill 兼容评分
- macOS 和 Linux
- 企业组织、权限和付费
- 在线评论、收藏、排行

## 7. 用户主流程

### 首次启动

1. 启动 SkillArk
2. 自动扫描 Agent
3. 用户确认检测结果
4. 创建或选择中央仓库目录
5. 进入 Skill Library

### 导入并分发

1. 用户点击“添加 Skill”
2. 选择本地目录或 ZIP
3. SkillArk 校验并预览
4. 用户确认加入 Library
5. 用户选择全局或项目 Workspace
6. 用户勾选目标 Agent
7. 选择复制或 Junction
8. 查看安装计划
9. 执行分发
10. 查看每个目标状态

## 8. 状态定义

### Skill 状态

- `ready`：格式有效，可分发
- `invalid`：格式不完整
- `missing`：中央目录丢失
- `archived`：已归档

### Agent 状态

- `detected`：自动检测成功
- `configured`：用户手动配置
- `unavailable`：路径不存在或不可访问
- `disabled`：用户禁用

### Deployment 状态

- `planned`
- `installing`
- `synced`
- `outdated`
- `modified`
- `missing`
- `failed`
- `uninstalled`

## 9. 非功能要求

### 安全

- 默认不执行 Skill 内脚本
- 不跟随逃出 Skill 根目录的路径
- ZIP 解压防止 Zip Slip
- 禁止在未确认情况下覆盖非受管目录
- 日志不得记录 Token 或敏感环境变量

### 性能

- 500 个本地 Skill 的列表首次加载小于 2 秒
- 100 个 Agent/项目目标状态校验小于 5 秒
- 大目录 Hash 计算可取消并显示进度

### 可恢复性

- 数据库迁移必须可回滚或备份
- 分发失败不得破坏原目标
- 应用崩溃后可识别未完成操作

## 10. v0.1 发布验收

- 至少在两台 Windows 设备验证
- 至少覆盖含中文和空格的路径
- 至少验证 20 个结构不同的 Skill
- 复制模式安装、验证、覆盖、卸载全部通过
- Junction 模式在支持环境中全部通过
- 任意失败场景不得留下无法识别的半成品目录
