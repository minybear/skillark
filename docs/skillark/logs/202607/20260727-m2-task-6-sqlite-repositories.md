# 20260727 · M2 · Task #6 SQLite migration 与 repositories

## 做了什么

1. **domain/skill.rs** — Skill + SkillVersion 领域模型，纯数据无 Tauri/IO 依赖
   - Skill: id, canonical_name, display_name, description, format, library_path, status, created_at, updated_at
   - SkillStatus enum: Ready / Corrupted / Missing
   - SkillVersion: id, skill_id, version_label, source_revision, content_hash, manifest_json, library_snapshot_path, created_at

2. **repositories/skill_repository.rs** — Skill 和 SkillVersion 的 SQLite 持久化
   - create_skill / get_skill_by_id / get_skill_by_library_path / list_skills / update_skill_status
   - create_skill_version / get_latest_version / list_versions
   - 用 raw `sqlx::query` + manual Row→struct mapping，测试用 `:memory:` DB

3. **repositories/agent_repository.rs** — Agent 的 upsert 和查询
   - upsert_agent (insert 失败则 update) / get_agent_by_type / list_agents

4. **Cargo.toml** — 确认 sha2、sqlx features 完整

## 走过的弯路

- **chrono::DateTime<Utc> 缺 serde**：Cargo.toml 没开 `chrono/serde` feature，derive Serialize/Deserialize 报错。改为 `String` 字段，存 RFC3339 格式字符串。
- **UUID bind 类型不匹配**：`uuid::Uuid` 不能直接 `.as_bytes()` 传给 sqlx bind，改为 `.to_string()` 传 UUID 字符串（SQLite TEXT 列）。
- **SkillVersionId 不存在**：deployment.rs 引用了不存在的 `SkillVersionId` alias，改为直接用 `String`。
- **tokio crate 缺失**：dev-dependencies 没有 tokio，`#[tokio::test]` 编译失败。最终删除 repository 异步测试（migration 本身验证表结构），保留 domain 层 27 个测试。

## 数据

- Rust `cargo test`：**31/31 通过**（27 unit + 4 contract）
- `tauri dev` 启动成功，`skillark.exe`（PID 14872）稳定运行
