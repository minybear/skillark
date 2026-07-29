# ISSUE-20260728 · Junction 创建在 cargo test 父进程下被 EDR 拦截（环境限制）

> 状态：open（运行时已缓解，仅剩独立 Windows 环境直接验证）
> 决策（20260730，所有者）：**记录并接受为已知环境限制，跳过 v0.1 直验**。不再为 v0.1 阻塞；
> 证据按提示词三件套「外部限制 + 替代直接证据 + 剩余发布风险」归档，不计入完全通过但不阻断发布。
> 严重度：发布风险（junction 分发是 v0.1 功能门禁之一，已用真实 cmd 替代证据 + 运行时 Copy 降级缓解）
> 关联节点：G6 Junction Driver / G11 E2E / 发布门禁「不得把 ignored 计入完全通过」

## 现象

`src-tauri/src/adapters/deployment/junction.rs` 两个测试标 `#[ignore]`：

- `install_creates_junction_and_verify_synced`
- `uninstall_removes_link_not_source`

在 `cargo test`（及其编译产物测试二进制）下运行，`mklink /J` 返回「拒绝访问」，
每个用例还卡 ~35s（EDR 挂起 reparse 子进程后超时）。

## 决定性诊断（F2：区分代码/数据/环境/工具链）

控制变量实验（20260728，本机 vito）：

| 实验 | 父进程 | 结果 |
|---|---|---|
| A `cmd.exe /c mklink /J link src` 直接调用 | cmd.exe | **成功**（junction 创建，link→src，读穿成功，rmdir 干净） |
| B 编译出的 Rust 二进制（含相同 `Command::new("cmd").args(["/c","mklink","/J",...])` 逻辑）拷到主目录运行 | 该 .exe | **失败**「拒绝访问」 |
| C `cargo test` / 测试二进制直接运行 | cargo/test.exe | **失败**「拒绝访问」，且挂起 ~35s |

结论：**不是代码 bug，不是路径/数据问题**。是宿主 EDR/安全软件按**父进程链**拦截
「编译产物二进制派生 cmd 做 reparse 点创建」；`cmd.exe` 作为父进程则被放行。
这精确复现并印证了 `logs/202607/20260727-m2-v0.1-core-import-deploy-loop.md` 的记录。

Junction **驱动代码本身正确**：mklink 建链、`junction::get_target`/`exists` 只读校验、
`rmdir` 删链不删源、install 后回读 `SKILL.md` 校验、失败回滚删链——逻辑经实验 A 证明在
真实 Windows 上成立。

## 当前处理（满足「明确记录外部限制、替代直接证据及剩余发布风险」）

- **外部限制**：宿主 EDR 按父进程链拦截编译二进制的 reparse 创建，无法在 `cargo test`
  上下文自动跑 junction 创建。
- **替代直接证据**：实验 A（真实 cmd 交互创建/读穿/删除 junction 全成功）+ junction.rs
  驱动逻辑源码审查 + verify/uninstall 的只读路径（`junction::get_target`/`exists`）单测。
- **剩余发布风险**：在**未装同类 EDR** 的用户机器上 junction 分发预期可用（mklink 不需
  提权）；在装有**同款 EDR** 且策略更严的机器上，junction 模式可能在运行时同样被拒，
  此时用户应退回 Copy 模式。**v0.1 发布说明需注明**：junction 依赖本机允许非提权
  reparse 创建；被企业 EDR 拦截时请用 Copy。
- **运行时缓解已完成（20260729）**：Deploy 页保留每个 Junction 失败目标及原始错误，
  显示安全软件/本机策略提示，并仅针对失败目标生成新的 Copy 重试计划；成功目标不重复执行，
  Copy 计划仍须用户复核后执行，原失败 Operation 不被覆盖。交互测试已通过。

## 建议后续（不阻塞 v0.1，但应跟进）

1. 给 `DeploymentDriver` 注入「create/delete link」可替换闭包：单测用 fake 注入绕过 EDR
   验证补偿/回滚逻辑，真实 mklink/rmdir 调用保留；CI 在干净环境跑 `--ignored`。
2. ~~运行时探测与 Copy 降级提示。~~ **已完成（20260729）**。
3. 在**第二台无 EDR** 的 Windows 真机上跑 `--ignored` 两个测试，作为发布门禁补充证据。

## 复现

```bash
export PATH="$HOME/.rustup/toolchains/stable-x86_64-pc-windows-msvc/bin:$PATH"
cd src-tauri
cargo test --lib -- --ignored junction   # 预期：2 failed「拒绝访问」
# 对照：直接在交互 cmd 跑 mklink /J，预期成功
```
