# 20260729 release Windows 发布阻塞项闭环

## 1. 目标/问题

处理 v0.1 Windows 发布剩余项：NSIS 因 GitHub 直连超时无法构建；Junction 在同款 EDR
环境可能运行失败但 UI 无恢复入口；手工中文 fixture 已被自动化覆盖；OneDrive/跨盘/只读
被笼统归为物理设备依赖，缺少可执行矩阵。

## 2. 方案/实现

- NSIS 仅使用 Tauri 官方发布资产。通过 GitHub 官方资产存储域获取
  `nsis-3.11.zip` 与 `nsis_tauri_utils.dll`，分别用 tauri-bundler 2.9.3 内置 SHA-1
  校验后写入工具缓存。
- Deploy 页保留 Junction 原始失败结果，只把失败目标映射为 Copy `PlanTargetSpec`，
  重新调用既有计划接口；用户复核计划后再执行，成功目标不重复分发。
- 将目标可写性探测提升为 Copy/Junction 共用逻辑：从目标向上找到最近存在目录并实写 marker；
  缺失目标且祖先不可写时计划分类为 `permission_denied`。
- 新增 Windows 环境测试：C 盘源到 D 盘目标的跨卷 Copy，以及 `icacls` 拒写父目录的可写性探测。
- 删除 `test-fixtures/skills/中文 Skill (E2E)` 两个手工文件，证据账本改指向运行时生成的路径矩阵。

## 3. 走过的弯路与根因

1. `github.com` 直接 TLS 握手仍超时；授权可信来源不能改变网络可达性。改为使用 GitHub
   返回的官方 `release-assets.githubusercontent.com` 资产地址，并以 Tauri 源码硬编码哈希
   作为完整性边界，没有使用第三方代理。
2. 第一次正式构建找不到 `cargo`。根因是当前进程 PATH 未包含真实 MSVC toolchain；
   将 `stable-x86_64-pc-windows-msvc/bin` 放到 PATH 前端后恢复。
3. “跨盘/只读必须依赖物理设备”的假设不成立。当前机器已有 C/D 两个 NTFS 卷，跨卷可直接测；
   只读/权限不足可用 ACL 确定性构造。只有 OneDrive 真同步和独立 Windows 环境仍是外部依赖。
4. ACL 测试揭示计划分类顺序缺口：`exists=false` 会掩盖 `writable=false`。同时立即父目录不存在
   不等于不可写，因此必须先探测最近存在祖先，再对缺失目标做权限分类。
5. `cargo fmt --check` 会改动大量历史未格式化文件，超出本轮范围；只格式化新增测试文件，
   以全量测试和 Clippy 零警告作为本轮质量证据。

## 4. 数据与验证

- NSIS 工具：
  - `nsis-3.11.zip` SHA-1
    `EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D`
  - `nsis_tauri_utils.dll` SHA-1
    `75197FEE3C6A814FE035788D1C34EAD39349B860`
- 最终安装包：
  - `SkillArk_0.1.0_x64-setup.exe`
  - 3,632,568 bytes
  - SHA-256
    `076B73142A332CB0E3B813CF3BAB3FA381FF2160D89D5CA88E6CDFD9B4D08AF2`
- 全新/覆盖升级：
  - 0.0.9 全新安装返回 0，ProductVersion/卸载项为 0.0.9
  - 覆盖升级 0.1.0 返回 0，ProductVersion/卸载项更新为 0.1.0
  - 隔离数据库升级前后 SHA-256 均为
    `817BD394FBB977584167269AFE3E8EF43E9D5462523B9121D426549251ACE127`
  - 升级后应用启动成功；最终安装包另做安装/启动/卸载冒烟，卸载目录与注册表无残留
- 前端：17/17 通过；`npm run check` 通过。
- Rust：117 通过、0 失败、2 ignored（Junction/EDR）；Clippy `-D warnings` 通过。
- Windows 环境：C→D 跨卷 Copy 哈希一致；ACL 拒写探测通过且测试恢复 ACL。

## 5. 经验与优化建议

- 下载工具链必须同时固定“来源身份 + 内容哈希”；只允许域名不足以形成供应链证据。
- 环境能力应按机制拆分：卷、ACL、同步客户端、独立系统不是同一种依赖，能本机模拟的应自动化。
- 降级不能静默改变语义；失败 Operation 与降级 Operation 分开，且只重试失败目标。
- 可写性判断要针对实际写入点的最近存在祖先，不能用路径是否存在代替权限判断。

## 6. 剩余项

- 当前用户没有可用 OneDrive 同步目录，无法验证真实同步扰动。
- 本机 EDR 继续拦截 cargo/test 父进程创建 Junction；仍需一台独立 Windows 环境运行
  `cargo test --lib -- --ignored junction`。
- 最终安装包未做 Authenticode 签名；代码签名不在本轮范围。
