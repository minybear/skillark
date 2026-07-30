-- SkillArk v0.2 Link Bridge — git source provenance
--
-- 决策（20260730）：复用 v0.1 sources 表（足够通用：source_type/display_name/base_url/config_json），
-- 不重建 sources。本迁移只新增 git 溯源所需表。
--   - source_revisions：一次 git 拉取的不可变身份（resolved commit + subpath + content hash）。
-- repository_cache / update_checks 随 L6（第二增量）再加，保持首增量聚焦「粘贴链接→导入」。
--
-- 注意：设计包 design/v0.2-v1.0/sql/0002_v0_2_link_bridge.sql 使用并行编号（0002~0010），
-- 与本仓库已实现的迁移序列（0001_init / 0002_agent_overrides）不一致。v0.2 在本仓库为 0003。

CREATE TABLE IF NOT EXISTS source_revisions (
    id TEXT PRIMARY KEY,
    source_id TEXT NOT NULL,
    -- 解析后的不可变 commit SHA（版本身份），分支/标签只是请求时的 ref
    resolved_revision TEXT NOT NULL,
    -- 用户请求的 ref（分支/标签/commit）；空串表示默认分支
    requested_ref TEXT NOT NULL DEFAULT '',
    -- 仓库内子目录（多 Skill 仓库时定位单个 Skill）；空串表示仓库根
    subpath TEXT NOT NULL DEFAULT '',
    -- 扫描出的 Skill 内容 hash（与 skill_versions.content_hash 对齐）
    content_hash TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES sources(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_source_revisions_source
ON source_revisions(source_id);

-- 同一来源 + commit + 子目录 唯一：重复导入同一链接同一版本去重
CREATE UNIQUE INDEX IF NOT EXISTS idx_source_revisions_identity
ON source_revisions(source_id, resolved_revision, subpath);
