# ISSUE-20260729 · NSIS 安装包构建被网络环境阻塞（GitHub 不可达）

> 状态：closed（20260729，官方资产直连存储域 + Tauri 内置哈希校验）
> 严重度：发布门禁缺口（G14「Windows 安装包全新安装 / 覆盖升级」未能自动证明）
> 关联节点：G14 安装、升级与迁移 / 发布门禁

## 现象

`npx tauri build --bundles nsis`：release 编译**成功**（`skillark.exe` 已生成，20m03s，
全优化 + 全部依赖通过），但打包阶段失败：

```
Info Verifying NSIS package
Downloading https://github.com/tauri-apps/binary-releases/releases/download/nsis-3.11/nsis-3.11.zip
failed to bundle project: `timeout: global`
```

## 根因（F2：环境，非代码）

- `curl https://github.com` → `http_code=000`（连接超时，**GitHub 直连在当前网络被阻断**）。
- Tauri v2 的 NSIS 打包器须从 GitHub `binary-releases` 下载 `nsis-3.11.zip`，无法完成。
- 代码、配置（`tauri.conf.json` bundle.active=true / targets=all）均正确；release 二进制本身已编译成功。

## 边界与约束

- 曾评估经第三方代理镜像下载 NSIS 放入 Tauri 缓存（`%LOCALAPPDATA%\Temp\.tauri`），
  但被安全策略正确拦截：**不应从用户未明确信任的第三方源下载并执行工具链**。
  此限制合理，不绕过。

## 已具备的替代证据

- release `skillark.exe` 编译成功（证明 release 构建链可用，仅缺 NSIS 打包工具下载）。
- debug `skillark.exe` 编译成功（7/27 起多次复跑）。
- 真实旧数据库迁移测试通过（`migration_over_real_existing_database_is_idempotent_and_preserves_data`）：
  拷贝 `.e2e-runtime` 7/28 真实 DB → 注入代表性旧数据 → 重跑迁移 → 幂等、数据保留、无重复应用。

## 剩余发布风险 / 待办（需用户决策或换网络）

1. **全新安装 / 覆盖升级**：需能访问 GitHub 的网络（或用户指定的可信 NSIS 来源/代理）完成
   `nsis-3.11.zip` 下载，再跑 `tauri build --bundles nsis` 产出安装包，随后在干净 Windows
   环境做全新安装 + 覆盖升级验证。
2. 可选：用户若能提供可信的 NSIS 分发镜像或离线 `nsis-3.11.zip`，放入 Tauri 缓存目录即可离线打包。
3. 若目标环境允许，配置 `TAURI` 下载代理或在公司内网镜像 `binary-releases`。

## 复现

```bash
curl -sI --max-time 15 https://github.com   # 当前网络：000（超时）
npx tauri build --bundles nsis              # release 编译过，打包在 NSIS 下载超时
```

## 关闭结果（20260729）

- 仅使用 Tauri 官方发布资产；通过 GitHub 返回的官方
  `release-assets.githubusercontent.com` 存储地址下载，不使用第三方代理。
- `nsis-3.11.zip` SHA-1：
  `EF7FF767E5CBD9EDD22ADD3A32C9B8F4500BB10D`，与 tauri-bundler 2.9.3 常量一致。
- `nsis_tauri_utils.dll` SHA-1：
  `75197FEE3C6A814FE035788D1C34EAD39349B860`，与 tauri-bundler 2.9.3 常量一致。
- `npx tauri build --bundles nsis` 成功，最终产物：
  `src-tauri/target/release/bundle/nsis/SkillArk_0.1.0_x64-setup.exe`。
- 最终安装包大小 3,632,568 bytes，SHA-256：
  `076B73142A332CB0E3B813CF3BAB3FA381FF2160D89D5CA88E6CDFD9B4D08AF2`。
- 以 0.0.9 基线安装包完成全新安装，再覆盖升级到 0.1.0：
  安装/升级均返回 0，ProductVersion 与卸载项升级到 0.1.0，
  隔离数据库升级前后 SHA-256 完全一致，升级后应用可启动。
- 最终 0.1.0 安装包再次完成静默安装、启动初始化数据库、静默卸载冒烟；
  安装目录和卸载注册表项均无残留。

关闭依据：
`logs/202607/20260729-release-Windows发布阻塞项闭环.md`。
