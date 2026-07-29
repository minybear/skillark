# CLAUDE.md


## 铁律：文档按项目沉淀 + 任务收尾必归档（无条件遵守）

所有设计、计划、日志、issue 按项目归档到 `docs/{项目}/` 下，**不再平铺**。

### 0. 项目文档骨架（开始一个项目先建）

开始一个新项目/新需求时，若 `docs/{项目}/` 不存在，先建骨架：**识别当前项目 → 列出将创建的目录给用户看 → 询问确认后创建**（对应 `init-project-docs` skill）。骨架：

```
docs/{项目}/design/   规范（长期稳定，约束怎么做）
docs/{项目}/plan/     需求计划（随需求推进）
docs/{项目}/logs/     开发日志（每个功能/修复必写，按月归档）
docs/{项目}/issues/   待处理问题（open/ 与 closed/）
```

### 1. 功能/修复完成 → 写开发日志

写到 `docs/{项目}/logs/{yyyyMM}/{yyyyMMdd}-{模块}-{标题}.md`（对应 `write-worklog` skill）。核心是沉淀"走过的弯路 + 经验 + 优化建议"三段，用数据说话，不只写"OK"。

### 2. 发现待处理问题 → 写 issue

写到 `docs/{项目}/issues/open/`（对应 `write-issue` skill），带完整上下文供后续会话冷启动。issue=事前待接手，log=事后已做完，别混。

### 3. 新需求 → 建 plan

写到 `docs/{项目}/plan/{yyyyMMdd}-{模块}-{需求名}/`（对应 `write-plan` skill），核心是 `01-需求分析`（澄清表）+ `02-方案设计`（多方案对比）。先文档后代码。

### 约束

- 日期一律用 `date +%Y%m%d` 取**当天真实日期**，不要用上下文里可能过期的日期。
- 归档操作说明（skill 文件）在 `.claude/skills/` 下（Claude Code 可直接调用；Codex 可阅读其中的 SKILL.md 作为操作规范）。
- 会话结束时有 Stop hook 兜底检查（Claude Code）：改了代码但当天没写日志会提醒，此时应补归档。
