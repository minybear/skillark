# 20260726 devex Windows 工具链与桌面构建验证

## 1. 背景/问题现象

SkillArk 的 Tauri/React/Rust 工程已经落盘，但本机缺少 Rust stable MSVC、Visual C++ Build Tools
和 Windows SDK，无法执行 Rust 测试、Clippy 或完整桌面构建。官方 Rust CDN 与 crates.io 在当前
网络下还出现 IPv6 连接长时间停滞，首次依赖同步无法稳定完成。

## 2. 方案/根因

- 安装 Visual Studio 2022 Build Tools 17.14.37516.0、MSVC 14.44.35207 和 Windows SDK
  10.0.26100.0。
- 安装并设为默认 `stable-x86_64-pc-windows-msvc`：`rustc 1.97.1`、`cargo 1.97.1`。
- Rustup 安装器和 Build Tools 引导程序分别完成 SHA-256/Authenticode 校验。
- Rust 组件从 TUNA 镜像补齐，但使用 Rust 官方 channel manifest 中的 SHA-256 复核。
- crates.io 先通过仅强制 IPv4 的本地 HTTPS CONNECT 通道同步索引；缺失 crate 从 USTC 镜像
  并发补齐后，逐个按 `Cargo.lock` 的 crates.io checksum 验证。
- 临时网络配置和通道在验证结束后移除，仓库不保留机器相关代理设置。

## 3. 走过的弯路

1. Rustup 官方 CDN 在本机优先走 IPv6，下载几乎无进展；直接无限等待不能形成可重复流程。
2. crates.io 首轮只建立 3 条低速连接，约 30 分钟仅缓存 342/516 个锁定包；改为并发镜像补齐后，
   约 17 秒完成 170 个有效包。
3. USTC 对 4 个版本号含 `+` 的非 Windows crate 返回错误内容；校验器全部拒绝写入，证明镜像下载
   不能绕过 checksum。
4. Tauri 初始标识 `com.skillark.app` 触发 `.app` 后缀提示；改为
   `com.skillark.desktop` 后复验无提示。
5. Rust 1.97 将中文 MSVC 的正常“正在创建库”进度输出识别为 `linker_messages`；仅在 MSVC
   目标局部豁免该已知提示，没有放宽其他 warning。

## 4. 效果

- Rust：`rustc 1.97.1 (8bab26f4f 2026-07-14)`，host 为
  `x86_64-pc-windows-msvc`。
- Rust tests：8/8 通过，0 失败。
- Clippy：`--all-targets -- -D warnings` 通过。
- 前端：35 个模块，JS 199.53 kB / gzip 62.72 kB。
- Tauri debug build 复验通过，生成 14,595,584 字节的
  `src-tauri/target/debug/skillark.exe`。
- 可执行文件 SHA-256：
  `C8E12B07AF45C98FC9D14A2101790CF894C79CF18C07560859BDBFB19F5CD356`。
- 对 512 个已缓存 crate 全量复核，checksum 异常数为 0。

## 5. 经验沉淀

1. Windows 桌面项目的环境验收不能止于 `cargo --version`，必须完成测试、Clippy 和 Tauri
   可执行文件链接，才能证明 MSVC、SDK、WebView2 与 Rust 真正连通。
2. 使用镜像时应把“下载位置”和“信任根”分开：镜像负责传输，官方 manifest/Cargo.lock
   checksum 负责决定内容是否可信。
3. 网络绕行配置必须是临时的，并在收尾时删除，避免把单机代理地址沉淀到项目配置。
4. 新版编译器与本地化工具链可能产生非代码告警，应精确豁免单个已知 lint，而不是关闭全部 warning。

## 6. 优化建议

1. 增加 `scripts/check-prerequisites.ps1`，输出 Rust host、MSVC、Windows SDK、WebView2 和
   实际链接测试结果。
2. 在 Windows CI 固化 `cargo test`、`cargo clippy -- -D warnings` 和 Tauri no-bundle build。
3. 为依赖缓存补充受 checksum 约束的预热流程，避免首次开发环境被网络质量阻塞。
4. 发布阶段再增加 NSIS/MSI 封装与签名验证；M1 阶段以 debug 应用本体为门禁。
