# ISSUE-20260725-devex-rust-msvc-toolchain-missing

状态：closed  
优先级：高  
登记日期：20260725
关闭日期：20260726

## 现象

SkillArk 的 Tauri 2 + React 工程已经生成，前端 TypeScript/Vite 构建通过，但当前 Windows 开发环境找不到：

- `rustc`
- `cargo`
- `rustup`
- 包含 `Microsoft.VisualStudio.Component.VC.Tools.x86.x64` 的 Visual C++ Build Tools

因此无法在本机执行 Cargo contract tests、Rust 静态检查或 Tauri desktop build。

当前已确认存在：

- Node.js `v20.19.6`
- npm `10.8.2`
- Microsoft Edge WebView2

## 复现

在仓库根目录执行：

```text
rustc --version
cargo --version
rustup show active-toolchain
```

三个命令均无法识别。使用 Visual Studio Installer 的 `vswhere` 查询 C++ 工具组件也没有返回安装实例。

## 相关位置

- Rust manifest：`src-tauri/Cargo.toml`
- Cargo contract tests：`src-tauri/tests/contracts.rs`
- SQLite migration：`src-tauri/migrations/0001_init.sql`
- Tauri 配置：`src-tauri/tauri.conf.json`

## 关联文档

- [v0.1 设计冻结记录](../../design/v0.1/10-ADR-v0.1-FREEZE.md)
- [开发计划](../../plan/20260725-bootstrap-v0.1/03-开发计划.md)
- [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/)

## 约束/边界

- 工具链是用户级/系统级环境变更，不在未明确授权时自动安装。
- 应安装 Rust stable 的 MSVC toolchain，而不是 GNU toolchain。
- Visual Studio Build Tools 需要 C++ Desktop workload 和 Windows SDK。
- 安装后必须重新打开终端，让 PATH 与环境变量生效。

## 初步分析

安装顺序建议：

1. 安装 Microsoft Visual Studio C++ Build Tools 与 Windows SDK。
2. 通过 rustup 安装 Rust stable MSVC。
3. 重新打开终端并确认 `rustc`、`cargo`、`rustup` 版本。
4. 执行 `cargo test --manifest-path src-tauri/Cargo.toml`。
5. 执行 `npm run tauri build -- --debug`，验证完整 Windows 桌面链路。

关闭条件：Cargo contract tests、Rust 检查和 Tauri debug build 均通过，并链接对应 worklog。

## 关闭结果

- Rust stable MSVC：`rustc 1.97.1`、`cargo 1.97.1`。
- Visual Studio 2022 Build Tools：17.14.37516.0，MSVC 14.44.35207。
- Windows SDK：10.0.26100.0。
- Cargo tests：8/8 通过。
- Clippy：`--all-targets -- -D warnings` 通过。
- Tauri debug build：生成 `src-tauri/target/debug/skillark.exe`。

关闭依据：[Windows 工具链与桌面构建验证](../../logs/202607/20260726-devex-Windows工具链与桌面构建验证.md)。
