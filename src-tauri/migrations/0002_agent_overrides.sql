CREATE TABLE IF NOT EXISTS agent_overrides (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL UNIQUE,
    display_name TEXT NOT NULL,
    cli_name TEXT,
    config_dir TEXT,
    skill_dir TEXT,
    skill_path_override TEXT,
    is_custom INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
