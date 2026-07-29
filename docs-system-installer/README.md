# AI 协作文档沉淀体系 · 一键安装包（v1）

> ⚠️ **v1 版本说明**：本包**暂不含 hook**（hook 仍在调试，v2 发布时恢复）。4 个 skill + 铁律 + 安装脚本一切正常；安装时会自动跳过 hook 步骤，不会报错。

把「**AI 辅助软件开发实操手册 V1.1**」的文档体系（1.4 文档类型速查 + 3.7 开发日志）落成可一键安装的工程化设施：**4 个 skill + Stop hook + 铁律**，Claude Code 与 Codex 通用。

装进任何 git 项目后，AI 会按项目把日志 / issue / plan 自动归档到 `docs/{项目}/` 下，不再平铺、不再遗漏。

---

## 这套体系做什么

| 能力 | 由什么实现 | 触发方式 |
|---|---|---|
| **新项目自动建目录** | `init-project-docs` skill | AI 识别你在做某项目 → **列出将创建的目录给你看 → 询问确认后创建**（方案一）<br>或你直接说「在 docs 下给 XX 建目录」 |
| **功能/修复完成写日志** | `write-worklog` skill | AI 完成后自动沉淀 → `docs/{项目}/logs/{yyyyMM}/{日期}-{模块}-{标题}.md` |
| **发现问题登记 issue** | `write-issue` skill | AI 发现待处理问题 → `docs/{项目}/issues/open/` |
| **新需求建 plan** | `write-plan` skill | AI 开新需求 → `docs/{项目}/plan/{日期}-{模块}-{需求}/` |
| **每次必触发兜底** | ~~Stop hook~~（v1 暂不含） | v2 发布后：Claude Code 每次停下检查未归档改动 → 提醒 |

**三层机制缺一不可**：Hook 管"每次必触发"，Skill 管"怎么按规范写"，铁律管"让 AI 心里有数"。

---

## 目录结构（安装后会用到的）

```
docs/{项目}/
    design/              # 规范：长期稳定，约束"怎么做"
    plan/                # 计划：随需求推进，记录"做什么/做到哪了"
    logs/{yyyyMM}/       # 开发日志：每个功能/修复完成后必写，按月归档
    issues/
        open/            # 未解决问题（事前，待接手）
        closed/          # 已修复（移入并指向 logs）
```

> issue = 事前"要做什么"，log = 事后"做了什么"，别混。

---

## 一键安装

### Windows（PowerShell）

在**目标项目根目录**或任意位置执行：

```powershell
powershell -ExecutionPolicy Bypass -File "完整路径\install.ps1"
```

如需指定项目根（不在 git 仓库里时）：

```powershell
powershell -ExecutionPolicy Bypass -File "完整路径\install.ps1" -ProjectRoot D:\work\myproj
```

> 若提示执行策略受限，加 `-ExecutionPolicy Bypass` 即可（仅本次）。

### Mac / Linux（bash）

```bash
bash install.sh
# 或指定项目根：
bash install.sh /path/to/your/project
```

---

## 安装会做什么

安装脚本**幂等**，可重复执行，已存在的文件自动跳过：

1. **装 4 个 skill** → `.claude/skills/`（`init-project-docs` / `write-worklog` / `write-issue` / `write-plan`）
2. **装 Stop hook** → `.claude/hooks/archive-reminder.sh`，并在 `.claude/settings.json` 注册 `Stop` 事件（**保留你已有的 permissions**）
3. **写铁律** → 追加到 `CLAUDE.md`（Claude Code 读）和 `AGENTS.md`（Codex 读），二者通用

> Codex 不支持 hook 与 skill 自动调用机制，但会读 `AGENTS.md` 铁律 + `.claude/skills/*/SKILL.md` 操作规范，按铁律归档。Claude Code 则 skill + hook + 铁律全生效。

---

## 卸载 / 移除

安装只做"加法"，没有全局副作用，手动反向操作即可：

- 删 `.claude/skills/{init-project-docs,write-worklog,write-issue,write-plan}` 四个目录
- 删 `.claude/hooks/archive-reminder.sh`，并从 `.claude/settings.json` 的 `hooks.Stop` 移除 `bash .claude/hooks/archive-reminder.sh` 一项（保留其余）
- 从 `CLAUDE.md` / `AGENTS.md` 删除「铁律：文档按项目沉淀 + 任务收尾必归档」一节

---

## 验证安装是否成功

装完后：

1. 在项目里随便改一个 `.java` / `.xml` 源码文件（不写日志），让 Claude Code 停下 → 应看到"📋 归档提醒"。
2. 对 Claude Code 说「开始 XX 项目」→ 它应列出将创建的 `docs/XX/{design,plan,logs,issues}` 目录并询问确认。
3. 让它做一个功能并说「写个开发日志」→ 应在 `docs/XX/logs/{yyyyMM}/` 生成按规范命名的日志。

---

## 常见问题

**Q: hook 提醒里路径是 `docs/{项目}/logs/`，没具体项目名？**
A: 正常。hook 只判断"今天任意项目目录下有没有日志"，具体项目名在 AI 写日志时确定。新项目先用 `init-project-docs` 建目录。

**Q: Codex 上 hook 会触发吗？**
A: 不会。hook 是 Claude Code 专属机制。Codex 靠 `AGENTS.md` 铁律 + 手动说"写个日志/记个 issue"触发（对应它读 SKILL.md 的规范来归档）。

**Q: 已经有 `.claude/settings.json` 会被覆盖吗？**
A: 不会。脚本读取后只**合并** `hooks.Stop`，已有的 `permissions` 等全部保留。

**Q: 安装包里 `payload/` 是什么？**
A: skill 文件、hook 脚本、铁律文本的母本（单一事实来源）。安装脚本把 payload 内容复制进目标项目。

---

## 体系来源

设计依据：《AI 辅助软件开发 · 工程师实操手册 V1.1》
- 1.4 文档类型速查（design / plan / logs / issues 分层）
- 1.7 问题即文档（issue 模板）
- 2.2~2.5 Skill 标准结构与沉淀方法
- 3.7 开发日志（六段模板 + 集中按月归档）
- 3.8 日志定期汇总反哺流程
