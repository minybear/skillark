# 20260726 · M1 · 独立 Agents 页面与中英文切换

## 做了什么

1. **中英文切换**：新增 `src/shared/i18n.ts`，集中管理全部双语文案（导航、标题、按钮、Agent 状态、信号标签等）。左下角语言切换按钮一键切换。
2. **导航路由**：新增 `src/shared/useNavigation.ts`（轻量页面状态管理），sidebar 从 HomePage 提升到 App 层，Overview 和 Agents 可点击切换。
3. **独立 Agents 页面**：新增 `src/pages/AgentsPage.tsx`，每张 Agent 卡片支持展开信号明细表（信号类型、权重、匹配状态、详情），并展示 Skill 路径、可执行文件路径、可写权限。
4. **样式**：App.css 补充 agents-toolbar、agent-meta、signal-table、signal-dot、expand-toggle 等样式。
5. **重构**：HomePage 精简为纯内容组件（不含 sidebar），所有文案引用 i18n.ts。

## 走过的弯路

- **旧进程残留导致编译/启动失败**：开发期间多次遇到 `failed to remove skillark.exe (os error 5)` 和端口 1420 占用。根因是旧 exe 仍在运行，Cargo 无法覆盖二进制文件。解决方式：每次重启前先 `taskkill /F /IM skillark.exe`，再确认端口无 LISTENING。
- **HomePage 第一版内联了完整 i18n 逻辑**：导致文件膨胀到 450 行，且 `t()` 函数设计冗余（返回对象而非字符串）。重构时抽出 `i18n.ts` + `pick()` 辅助函数，HomePage 降到 184 行。
- **nav 按钮全 disabled 问题**：原始设计所有导航按钮 `cursor: default` 无交互。本次将 Overview 和 Agents 设为可点击，其余标记 M2 badge + disabled。

## 经验

- Tauri dev 热重载只作用于前端（Vite HMR）；Rust 代码变更需要 Cargo 重新编译，如果旧 exe 锁定文件会失败。Windows 上务必先杀进程。
- 轻量路由（useState page id）在 M1 阶段够用，无需引入 react-router。后续 M3 MVP UI 再评估是否升级。
- i18n 字符串集中管理比散落组件内更好维护，`pick(lang, pair)` 一行搞定。

## 数据

- TypeScript typecheck：`tsc --noEmit` 通过，零错误。
- Rust cargo build：通过，无 warning。
- 真实启动：`skillark.exe` 稳定运行，页面切换 + 中英文切换 + Discover agents 功能正常。

## 优化建议

1. AgentsPage 的 Discover 和 HomePage 的 Discover 是两份独立 state，切页面后不保留扫描结果。后续可提升 discovery state 到 App 层或用 Context 共享。
2. 信号明细表目前只有英文信号类型标签（path_executable 等），已在 SIGNAL_LABELS 映射了 5 种，后续 Custom Agent 需要扩展。
3. 考虑为 AgentsPage 加一个「上次扫描时间」显示，让用户知道数据新鲜度。
