# 20260725 bootstrap M0 设计冻结与工程初始化

## 1. 背景/问题现象

SkillArk 已完成设计包归档，但五项跨文档差异尚未冻结，仓库也没有可运行的 Tauri/React/Rust 工程。
如果直接进入 Agent 探测，外部字段、Workspace 语义、状态机和数据库访问方式会同步产生返工。

## 2. 方案/根因

本次完成 M0：

- 冻结 GitHub 来源、Agent DTO、Global Workspace、Deployment 状态和 SQLite 访问五项决策。
- 将结论写入 ADR-008～012、设计正文和三个 JSON Schema。
- 使用官方 `create-tauri-app 4.6.2` 生成 Tauri 2 + React/TypeScript 基线。
- 建立 Domain、Application、Ports、Adapters、Commands 分层。
- 后端采用 `sqlx 0.8` 管理 SQLite，不向前端暴露通用 SQL 权限。
- 将初始 SQL 迁入 `src-tauri/migrations/0001_init.sql`。
- 新增 Rust DTO 与 JSON Schema contract tests。
- 将默认欢迎页替换为 SkillArk 项目状态界面。

## 3. 走过的弯路

1. 本机未安装 Rust/Cargo 和 Visual C++ Build Tools，导致无法执行 Cargo 测试与 Tauri desktop build。该环境缺口已拆为独立高优先级 issue，没有通过未经授权的系统安装绕过。
2. Playwright 自带 Chromium 未下载；视觉检查改用本机 Microsoft Edge 可执行文件，避免额外下载浏览器。
3. npm 默认镜像不实现 audit API；改用 npm 官方 registry 后完成依赖审计。
4. 初版界面引用 Google Fonts，与“本地优先、离线可用”定位冲突；已改为 Windows 自带 Segoe UI Variable 和 Cascadia Mono 字体栈。

## 4. 效果

- `npm install` 安装 72 个包。
- `npm run check` 通过 TypeScript 检查和 Vite 生产构建，共转换 34 个模块。
- 生产输出：HTML 0.53 kB、CSS 6.47 kB、JavaScript 197.49 kB；JS gzip 62.10 kB。
- Edge/Playwright 在 1440×1000 视口完成视觉检查，页面标题正确，运行时错误数为 0。
- SQLite migration 在内存数据库生成 10 张预期业务表，重复执行通过。
- 3 个 JSON Schema 通过 Draft 2020-12 元校验。
- npm 官方 registry 审计结果为 0 个生产依赖漏洞。
- Rust contract tests 已落盘但未执行，原因是本机缺少 Rust/MSVC 工具链。

## 5. 经验沉淀

1. Domain 驱动的桌面应用不应因为官方存在 SQL 前端插件就直接开放数据库；数据访问仍需经过 Application Service 和 Repository Port。
2. M0 冻结必须同时修改 ADR、正文、Schema、DTO 和测试，只有文档决策没有工程约束仍会漂移。
3. “本地优先”不仅是数据存储策略，也约束字体、资源和启动路径，首屏不应依赖外部 CDN。
4. 工具链探测应在脚手架前执行，并将代码问题与环境问题分开报告。

## 6. 优化建议

1. 增加 `scripts/check-prerequisites.ps1`，一次检查 Node、Rust MSVC、C++ Build Tools、Windows SDK 和 WebView2。
2. CI 在 Windows runner 上执行 `npm run check`、Cargo tests、Clippy 和 Tauri debug build，补足本机环境差异。
3. M1 开始前先关闭 Rust/MSVC 工具链 issue，确保 Agent 路径 POC 能在真实 Windows 环境运行。
4. 后续将 Playwright 视觉检查固化为本地测试，并保存窄屏和桌面两种基线。
