PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS sources (
    id TEXT PRIMARY KEY,
    source_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    base_url TEXT,
    enabled INTEGER NOT NULL DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS skills (
    id TEXT PRIMARY KEY,
    canonical_name TEXT NOT NULL,
    display_name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    format TEXT NOT NULL DEFAULT 'agent-skills',
    source_id TEXT,
    source_ref TEXT,
    library_path TEXT NOT NULL,
    current_version_id TEXT,
    status TEXT NOT NULL DEFAULT 'ready',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES sources(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_skills_library_path
ON skills(library_path);

CREATE INDEX IF NOT EXISTS idx_skills_canonical_name
ON skills(canonical_name);

CREATE TABLE IF NOT EXISTS skill_versions (
    id TEXT PRIMARY KEY,
    skill_id TEXT NOT NULL,
    version_label TEXT,
    source_revision TEXT,
    content_hash TEXT NOT NULL,
    manifest_json TEXT NOT NULL DEFAULT '{}',
    library_snapshot_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE,
    UNIQUE(skill_id, content_hash)
);

CREATE INDEX IF NOT EXISTS idx_skill_versions_skill_id
ON skill_versions(skill_id);

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    display_name TEXT NOT NULL,
    environment TEXT NOT NULL DEFAULT 'windows',
    executable_path TEXT,
    global_skill_path TEXT,
    status TEXT NOT NULL DEFAULT 'detected',
    confidence INTEGER NOT NULL DEFAULT 0,
    user_configured INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    config_json TEXT NOT NULL DEFAULT '{}',
    last_detected_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_agents_type
ON agents(agent_type);

CREATE TABLE IF NOT EXISTS workspaces (
    id TEXT PRIMARY KEY,
    workspace_type TEXT NOT NULL,
    name TEXT NOT NULL,
    root_path TEXT,
    status TEXT NOT NULL DEFAULT 'available',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_workspaces_root_path
ON workspaces(root_path)
WHERE root_path IS NOT NULL;

CREATE TABLE IF NOT EXISTS workspace_agents (
    workspace_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    target_path_override TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, agent_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT NOT NULL,
    status TEXT NOT NULL,
    plan_json TEXT NOT NULL DEFAULT '{}',
    result_json TEXT,
    error_code TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_operations_started_at
ON operations(started_at DESC);

CREATE TABLE IF NOT EXISTS deployments (
    id TEXT PRIMARY KEY,
    skill_version_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    operation_id TEXT,
    target_path TEXT NOT NULL,
    install_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    deployed_hash TEXT,
    source_path_at_install TEXT NOT NULL,
    installed_at TEXT,
    last_verified_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (skill_version_id) REFERENCES skill_versions(id),
    FOREIGN KEY (agent_id) REFERENCES agents(id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id),
    FOREIGN KEY (operation_id) REFERENCES operations(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_deployments_target_path
ON deployments(target_path)
WHERE status != 'uninstalled';

CREATE INDEX IF NOT EXISTS idx_deployments_skill_version
ON deployments(skill_version_id);

CREATE INDEX IF NOT EXISTS idx_deployments_workspace_agent
ON deployments(workspace_id, agent_id);

CREATE TABLE IF NOT EXISTS validation_reports (
    id TEXT PRIMARY KEY,
    skill_version_id TEXT NOT NULL,
    validator_version TEXT NOT NULL,
    valid INTEGER NOT NULL,
    risk_level TEXT NOT NULL DEFAULT 'unknown',
    findings_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    FOREIGN KEY (skill_version_id) REFERENCES skill_versions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_validation_reports_skill_version
ON validation_reports(skill_version_id, created_at DESC);

CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,
    value_json TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- SQLite cannot add this circular foreign key cleanly at table creation time.
-- Application code must verify that skills.current_version_id belongs to the same skill.
