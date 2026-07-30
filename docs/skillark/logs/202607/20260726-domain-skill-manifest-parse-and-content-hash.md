# Task #5：SKILL.md 解析、路径规则和稳定目录 Hash

- 日期：2026-07-26
- 模块：domain（skill_manifest / content_hash / path_safety）
- 关联：SkillArk v0.1 开发计划 Task #5

## 做了什么

为 SkillArk domain 层新增三个纯净（无 Tauri 依赖）模块，为后续的 skill 扫描、去重和部署安全打基础：

1. `src-tauri/src/domain/skill_manifest.rs` — 手写 YAML 前置块解析器
   - `SkillManifest { name, version, description, entry, tags }`
   - `parse_skill_md(content)` 解析 `---` 分隔的 `key: value` 块
   - `ParseError` + `ParseErrorKind`（MissingFrontMatter / InvalidYaml / MissingName / MissingVersion），实现 `std::error::Error + Display`
2. `src-tauri/src/domain/content_hash.rs` — 稳定目录 SHA-256
   - `hash_directory(root)`：BTreeMap 按相对路径排序，仅哈希常规文件内容
   - 跳过所有 symlink 和 Windows reparse point（junction 等）
   - 路径分隔符归一为 `/`（Windows/Linux 同夹具同 hash）
3. `src-tauri/src/domain/path_safety.rs` — 路径安全三件套
   - `is_within`（canonicalize 后 starts_with）
   - `has_no_traversal`（拒绝 `..`，纯内存）
   - `symlink_escapes_root`（fail-closed，读不出链接视为危险）
4. `domain/mod.rs` 注册三个模块；`Cargo.toml` 新增 `sha2 = "0.10"`。

## 数据

- 测试结果：`cargo test` 全绿
  - 单元测试 **27 passed / 0 failed**
  - 契约测试 **4 passed / 0 failed**（含既有 `skill_manifest_matches_the_frozen_contract`）
  - 新增 domain 测试共 **14 个**：skill_manifest 9 + content_hash 5 + path_safety 6（其中 2 个 symlink 测试 `#[cfg(unix)]`，Windows 上不编译）
- 新增代码：skill_manifest 292 行、content_hash 225 行、path_safety 172 行。
- 依赖增量：仅 `sha2 = "0.10"`（任务明确要求；未引入 yaml crate）。
- 未改动既有契约：`commands::contracts::SkillManifestDto` 与新 `domain::SkillManifest` 是两个不同概念（前者是部署后清单含 files/content_hash/warnings，后者是 SKILL.md 作者声明的前置元数据），契约测试无需变更。

## 走过的弯路

- **`PathBuf::set_modified` 不存在**。第一版 `hash_ignores_mtime` 测试直接调 `file.set_modified(...)`，编译报 `E0599: no method named set_modified found for struct PathBuf`。
  - 修正：改用 `OpenOptions::new().write(true).open()` 拿到 `File`，再 `FileTimes::new().set_modified(t)` + `file.set_times(times)`（`FileTimes` 自 Rust 1.75 稳定）。skillark 工具链满足。
- **参考代码缺 `use std::fs;`**。任务给的 path_safety 参考片段在 `symlink_escapes_root` 用了 `fs::read_link` 但没 import，补上后才编译。
- **`Display` 实现误挂到错误的 impl 块**。草稿一度给 `SkillManifest` 加了无意义的 `kind_label` 方法（实际该属于 `ParseError`）。已在编译前删除，避免无意义的 `#[allow(dead_code)]`。

## 经验

- **手写 KV 解析优于拉 yaml crate**。SKILL.md manifest 只有 5 个标量字段，手写解析器 ~120 行（含错误分类），换来 domain 层零新依赖、错误类型可枚举分派（调用方能精确提示"缺 name"而非泛化字符串）。向前兼容靠"未知 key 静默忽略"一行实现。
- **稳定性 = 内容寻址 + 排序 + 分隔符归一**。三件事缺一不可：(1) 只读文件字节不读 mtime/perm；(2) BTreeMap 保证插入序无关；(3) `replace('\\', "/")` 保证跨平台一致。`hash_sorts_by_path` 和 `hash_ignores_mtime` 两个测试正是对这三条不变量的钉子。
- **symlink 安全走"fail-closed + 平台门控"**。hash 时一律跳过 symlink，不尝试解析；path_safety 里读不出链接直接判 `true`（危险）。symlink 相关测试用 `#[cfg(unix)]` 门控，避免 Windows CI 因缺管理员权限而 flaky。

## 优化建议（后续）

- `skill_manifest` 目前只支持 `key: value` 标量。若未来 manifest 需要 list/嵌套，考虑引入 `serde_yaml` 或扩展手写解析器（届时同步更新契约 schema 与 `SkillManifestDto` 的 metadata 映射）。
- `content_hash` 对超大目录一次性 `fs::read` 全部入内存。若 skill 体积增长，可改为流式 `hasher.update_reader`，或加文件大小上限 + 早退。
- `path_safety::is_within` 依赖 `canonicalize`（要求路径已存在）。部署管线常需要在"目标尚未创建"时预校验，后续可补一个 `is_within_lexical(root, target)` 基于 normalize 后的 `..` 折叠判断，覆盖 pre-create 场景。
- 下一 Task：把这三个模块接到 application 层（skill 扫描器 / 部署计划生成），届时为 domain→DTO 映射补 `From<&SkillManifest>`。
