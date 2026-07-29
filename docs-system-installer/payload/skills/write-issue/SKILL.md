---
name: write-issue
description: 把开发中发现但暂不处理的问题，写成一份带完整上下文的 issue md 到 docs/{项目}/issues/open/。适用于：改 A 时发现 B 有隐患、测试撞见边角 bug、发现一个待办隐患但手头在忙别的。当用户说"记个 issue"、"这个问题先记下来"、"写个待处理问题"、"发现个隐患先存着"、"登记这个问题"时使用。issue = 事前待接手，log = 事后已做完，两者别混。
---

# 写问题（issue）

开发中岔出的新问题，当下上下文最全但手头在忙别的。与其记一句含糊 TODO（回头还要重新描述、AI 重新摸一遍代码），不如当场写成**自带完整上下文的 issue md**——后续任何新会话读它就能冷启动开工。

## 归档位置与命名

```
docs/{项目}/issues/open/ISSUE-{yyyyMMdd}-{模块}-{标题}.md
```

- **{项目}**：当前工作所属的项目目录（如 `ioh`、`websheet`）。目录不存在先走 `init-project-docs` 建骨架。
- 日期用 `date +%Y%m%d` 取当天。
- 模块代码小写：`orchestration` / `websheet` / `integration` / `device-interface` / `diameter-client` / `config` / `om` / `es-common-lib`。
- 修复完成后把文件从 `open/` 移到 `docs/{项目}/issues/closed/`，并在文件内加一行 `→ 已修复，见 logs/xxx`。

## 工作流程

1. `date +%Y%m%d` 取当天日期。
2. 按模板写 issue 文件到 `docs/{项目}/issues/open/`。**关键：把"冷启动所需的全部上下文"写进去**——现象、复现、相关代码位置、关联文档、约束边界。
3. 在 `docs/{项目}/issues/README.md` 的 open 状态表登记一行。
4. 回复用户 issue 路径 + 优先级。

## issue 模板

```markdown
# ISSUE-{yyyyMMdd}-{模块}-{标题}

状态： open
优先级： 高 / 中 / 低
登记日期： {yyyyMMdd}

## 现象
（问题表现，贴报错 / 对比预期）

## 复现
（触发条件、步骤、环境）

## 相关位置
（涉及代码 `文件:行号`、类、方法）

## 关联文档
（docs/{项目}/design 相关规范、相关 plan/log）

## 约束/边界
（修复时不能动的：对外契约、兼容、口径等）

## 初步分析（可选）
（已定位到的方向 / 怀疑点）
```

## 生命周期

`open` → 接手处理（大的升级成 plan 需求，走 `write-plan`）→ 修复（过程照常写 log，走 `write-worklog`）→ 移入 `closed/` 并留一行指向 log。

## 关键要求

- **上下文要全**：这是给"后续新会话冷启动"看的，缺了位置/约束，接手时还得重摸一遍，就失去意义。
- **issue ≠ log**：issue 说"要做什么"（事前），log 说"做了什么"（事后）。别混。
- **别堆僵尸**：open issues 要随周复盘一起过——处理 / 升级成 plan / 关掉 / 标过期。
- 与 Jira 等 tracker 不冲突：tracker 管流转，这份 md 管给 AI 看的完整上下文，可互相链接。
