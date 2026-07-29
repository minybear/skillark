# SkillArk UI 信息架构

## 1. 导航结构

```text
首页
Skill Library
Agent
Workspace
操作记录
设置
```

v0.1 不单独设置 Marketplace 页面。

## 2. 首页

展示：

- Skill 总数
- 已配置 Agent 数
- 项目 Workspace 数
- synced / outdated / modified / failed 数量
- 最近操作
- 需要处理的问题

主要按钮：

- 添加 Skill
- 新建项目 Workspace
- 扫描 Agent

## 3. Skill Library

### 列表字段

- 名称
- 描述
- 当前版本或 Hash 短码
- 来源
- 已分发目标数
- 状态
- 最近更新

### 详情页

Tab：

- 概览
- 文件
- 部署
- 版本
- 校验报告

操作：

- 分发
- 更新元数据
- 创建新版本
- 删除

## 4. Agent 页面

每个 Agent 卡片：

- 名称和图标
- detected / configured / unavailable
- 可执行路径
- 全局 Skill 路径
- 可写状态
- 置信度与检测信号

操作：

- 重新扫描
- 编辑路径
- 验证
- 禁用
- 添加自定义 Agent

## 5. Workspace 页面

项目列表字段：

- 项目名
- 根目录
- 关联 Agent
- Skill 数量
- 异常数量
- 最近验证时间

项目详情：

- Agent 目标路径预览
- 已部署 Skill
- 批量添加或移除
- 验证全部

## 6. 分发向导

### Step 1：选择 Skill

支持单选或批量。

### Step 2：选择范围

- 全局
- 一个或多个项目 Workspace

### Step 3：选择 Agent

只显示已经配置并可写的 Agent；不可用 Agent 显示原因。

### Step 4：模式和冲突

- Copy（默认）
- Junction（高级）
- 展示每个目标路径和冲突状态

### Step 5：确认计划

明确展示：

- 将创建的目录
- 将覆盖或备份的目录
- 需要用户决定的冲突

### Step 6：结果

每个目标独立显示成功或失败，不因一个失败隐藏其他结果。

## 7. 关键交互原则

- 不用“同步”一个词覆盖所有动作；区分安装、重新分发、验证、更新源。
- 写文件前必须展示真实目标路径。
- 高风险操作必须展示影响范围，不只显示“确定吗”。
- 错误信息包含可行动建议。
- 批量操作允许部分成功，并提供重试失败项。

## 8. 页面状态

所有页面必须设计：

- loading
- empty
- ready
- partial error
- blocking error
- stale data

不得只设计理想成功状态。
