# skillark · 问题索引

> 发现的问题写到 `open/`，修复后移入 `closed/` 并留一行指向 logs。产出使用 write-issue skill。

## Open

| Issue | 模块 | 剩余条件 |
|---|---|---|
| [Junction 在 cargo test 父进程下被 EDR 拦截](open/ISSUE-20260728-junction-edr-block-under-cargo-test.md) | deployment | UI Copy 降级已完成；待独立 Windows 环境运行 2 个 ignored 测试 |

## 已关闭

| Issue | 模块 | 关闭依据 |
|---|---|---|
| [Rust/MSVC 工具链缺失](closed/ISSUE-20260725-devex-rust-msvc-toolchain-missing.md) | devex | Cargo tests、Clippy 与 Tauri debug build 已通过 |
| [v0.1 设计契约不一致与待冻结项](closed/ISSUE-20260725-design-v0.1-contract-inconsistencies.md) | design | ADR-008～012、Schema 和工程契约测试已落地 |
| [NSIS 安装包构建被网络环境阻塞](closed/ISSUE-20260729-nsis-bundle-blocked-by-network.md) | release | 官方资产哈希校验、NSIS 构建、全新安装和覆盖升级均通过 |
