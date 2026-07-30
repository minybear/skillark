# 20260726 · M1 · Custom Adapter + 持久化 + 验证矩阵

## 做了什么

1. **CustomAgentAdapter**（`src-tauri/src/adapters/custom_agent.rs`）
   - 动态 String 字段的 Adapter，实现 `AgentAdapter` trait
   - 支持 CLI / config / skill / process 多信号检测 + skill_path_override
   - `custom_adapters(overrides)` 从持久化配置构建 adapter 列表
   - 4 个单测：全信号匹配 confidence=100、零信号 confidence=0、override 路径、中文路径

2. **持久化**（`src-tauri/src/application/agent_overrides.rs`）
   - `AgentOverride` 结构体，存 JSON 文件 `~/.skillark/agent_overrides.json`
   - `load_overrides()` / `save_override()` / `delete_override()`
   - migration 0002 已建（`agent_overrides` 表，目前用 JSON 文件方案，后续 SQLite 接入）
   - roundtrip 单测通过

3. **discover_agents 集成**
   - 加载 overrides，将 `skill_path_override` 合并到 `manual_skill_paths`
   - built-in adapters 之后遍历 custom adapters
   - AgentAdapter trait `display_name()` 改为返回 `String`（支持动态名称）

4. **3 个新 Tauri 命令**
   - `get_agent_overrides` / `save_agent_override` / `delete_agent_override`
   - 已在 `lib.rs` 的 `invoke_handler` 中注册

5. **前端**
   - `src/api/overrides.ts`：Tauri IPC 封装
   - `AgentsPage.tsx`：新增 CustomAgentSection（表单 + 已保存列表 + 删除）
   - `i18n.ts`：新增 14 个双语 key
   - `App.css`：表单、按钮、删除按钮样式

## 走过的弯路

- **AgentAdapter trait display_name 签名**：原 trait 返回 `&'static str`，Custom Agent 动态名称无法满足。改为 `String`，BuiltInAgentAdapter 实现改为 `.to_owned()`。
- **writable 双层 Option**：`global_skill_path.map(path_writable)` 产生 `Option<Option<bool>>`，改为 `and_then` 展平。
- **global_skill_path move 后 borrow**：先取 writable 再 map normalize_path，调换顺序解决。
- **中文路径测试断言**：`normalize_path` 在 Windows 上返回 `\` 分隔，测试期望 `/` 分隔。改为忽略分隔符的跨平台断言（`replace('\\', "/")` 比较）。
- **子代理触达 25 轮上限**：deep-analysis 子代理创建了文件骨架但未完成 commands 和 lib.rs 注册，主会话补全了后端集成和全部前端工作。

## 数据

- Rust `cargo test`：**13/13 通过**（9 unit + 4 contract）
- TypeScript `tsc --noEmit`：零错误
- `tauri dev` 启动成功，`skillark.exe`（PID 9576）稳定运行

## 优化建议

1. `agent_overrides` 目前用 JSON 文件持久化，M6 SQLite repositories 接入后应迁移到数据库 `agent_overrides` 表。
2. AgentsPage 的 discovery 和 custom agent section 各自独立状态，保存自定义 Agent 后需手动 Rescan 才能看到新 Agent。后续可保存后自动触发一次 scan。
3. AgentAdapter trait 的 `display_name` 改为 String 后，BuiltInAgentAdapter 每次调用有一次 `to_owned()` 开销（可忽略）。
