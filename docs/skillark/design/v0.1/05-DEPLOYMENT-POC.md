# Skill 分发 POC 设计

## 1. POC 目标

完成一个受管 Skill 从中央 Library 分发到多个临时目标目录，验证复制、Junction、冲突、状态校验和卸载流程。

POC 阶段优先使用测试目录，不直接写真实 Agent 目录。

## 2. 分发模式

### Copy

优点：

- 最稳定
- 不依赖链接权限
- 目标 Agent 读取行为可预测

缺点：

- 更新后需要重新分发
- 用户可能修改副本导致分叉

### Junction

优点：

- 中央 Library 更新后实时生效
- Windows 下通常比文件符号链接更容易使用

限制：

- 主要用于目录
- 跨卷、网络路径、UNC 和 WSL 场景需要单独验证
- 目标被手动替换后需要重新建立

v0.1 默认 Copy，Junction 标记为高级选项。

## 3. 安装算法

### Copy 模式

1. 验证源 Skill
2. 计算源 Hash
3. 检查目标路径
4. 若目标非受管，生成冲突并停止
5. 创建 `target.skillark-tmp-<operation-id>`
6. 复制所有文件，不跟随外部链接
7. 校验临时目录 Hash
8. 若目标存在，重命名为备份目录
9. 临时目录重命名为正式目标
10. 写 Deployment
11. 删除备份

### Junction 模式

1. 验证源目录为本地绝对路径
2. 检查目标父目录可写
3. 检查目标冲突
4. 创建临时 Junction
5. 验证 Junction 指向源路径
6. 原子替换目标
7. 写 Deployment

## 4. 冲突分类

- `none`：目标不存在
- `managed_same`：已由 SkillArk 管理且内容一致
- `managed_outdated`：已管理但不是当前版本
- `managed_modified`：目标被用户修改
- `unmanaged_skill`：目标存在有效 Skill，但不是 SkillArk 管理
- `unmanaged_directory`：目标存在普通目录
- `file_conflict`：目标是文件
- `permission_denied`

默认策略：除 `none` 和 `managed_same` 外都需要确认。

## 5. 卸载规则

卸载前：

- 确认 Deployment 存在
- 确认目标仍与部署记录关联
- Copy 模式下如果目标被修改，默认不直接删除
- Junction 模式确认链接目标正确后再删除链接

对修改过的 Copy 目录提供：

- 保留目录并解除管理
- 导回 Library 形成新版本
- 强制删除

## 6. 状态验证

### Copy

- 目标不存在：missing
- Hash 等于 deployed_hash：synced
- Hash 等于 Library 当前版本但不等于 deployed_hash：synced，并在 VerifyResult 中附加 Library 版本变化提示
- Hash 不同：modified 或 outdated，根据 Library 版本判断

### Junction

- 链接不存在：missing
- 链接目标等于 Library 路径：synced
- 链接目标不同：modified
- 链接损坏：failed

## 7. 安全边界

- 不跟随指向 Skill 根目录外部的目录链接
- ZIP 解压必须校验规范化路径仍位于目标根目录
- 拒绝目标为磁盘根目录、用户主目录或系统目录
- 删除前必须确认目标路径位于 Adapter 允许的 Skill 根目录下
- 所有操作都使用规范化绝对路径比较

## 8. POC 测试集

至少包含：

1. 纯 `SKILL.md`
2. 含 scripts、references、assets
3. 目录含中文名
4. 单文件大于 20 MB
5. 1000 个小文件
6. 源目录包含内部链接
7. 源目录包含逃逸链接
8. 目标已存在普通目录
9. 目标被修改
10. 安装过程中模拟失败

## 9. 验收标准

- Copy 和 Junction 均可安装、验证、卸载
- 安装失败后旧目标保持可用
- 无半成品目录残留，或可被下次启动自动清理
- 目标被手动修改时不会静默覆盖
- 路径含中文、空格时行为一致
