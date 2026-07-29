# SkillArk v0.1 测试计划

## 1. 测试层级

### Unit

- SKILL.md 解析
- 路径规范化
- Hash 稳定性
- 冲突分类
- Deployment 状态计算
- Agent 候选评分

### Integration

- SQLite Repository
- 本地目录导入
- ZIP 解压
- Copy Driver
- Junction Driver
- Operation 补偿流程

### End-to-End

- 首次启动
- Agent 扫描与修正
- 导入 Skill
- 创建项目 Workspace
- 多目标分发
- 修改目标后验证
- 卸载

## 2. Windows 环境矩阵

- Windows 11，普通用户
- Windows 11，开发者模式开启
- NTFS 同盘
- NTFS 跨盘
- 路径含中文
- 路径含空格和括号
- OneDrive 同步目录
- 只读或权限不足目录

## 3. 数据测试集

建议建立 `test-fixtures/skills/`：

- minimal-valid
- full-valid
- invalid-no-frontmatter
- invalid-no-name
- duplicate-name
- nested-assets
- unicode-paths
- many-small-files
- large-file
- internal-symlink
- escaping-symlink
- malicious-zip-slip

## 4. 关键不变量

1. 数据库中的 synced 不得对应一个不存在的目标。
2. 失败操作不得丢失原目标。
3. 删除 Deployment 不等于一定删除目标目录。
4. 非受管目录不得静默覆盖。
5. 同一个内容 Hash 的 SkillVersion 可以复用，不重复存储。
6. 用户手动配置的 Agent 路径不得被自动扫描覆盖。

## 5. 故障注入

在以下阶段模拟失败：

- 复制 20% 时
- 复制完成但 Hash 校验前
- 备份旧目录后
- 临时目录重命名前
- 文件完成但数据库提交失败
- 数据库完成但清理备份失败

每个故障都要定义恢复策略。

## 6. 发布门禁

- 单元测试全部通过
- 核心集成测试全部通过
- 无高等级路径穿越问题
- 无静默数据丢失问题
- Windows 安装包全新安装和覆盖升级通过
- 数据库迁移在一份真实旧数据副本上通过
