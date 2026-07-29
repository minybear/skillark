//! Integration tests for the SQLite repositories.
//!
//! Each test runs against a fresh temp-file database (migrations applied via
//! `adapters::sqlite::connect`) so the partial-unique-index and FK cascade
//! behaviour is exercised against the real schema.

use std::path::PathBuf;

use skillark_lib::{
    adapters::sqlite::connect,
    domain::{
        deployment::{DeploymentRecord, DeploymentStatus, InstallMode},
        operation::{OperationStatus, OperationType},
        skill::{Skill, SkillStatus, SkillVersion},
    },
    repositories::{
        AgentRepository, DeploymentRepository, OperationRepository, SkillRepository,
        WorkspaceRepository, GLOBAL_DEFAULT_ID,
    },
};
use sqlx::SqlitePool;
use uuid::Uuid;

async fn setup_pool() -> SqlitePool {
    // Base under the user home (not Temp) to avoid the host AV/EDR hotspot that
    // makes fs/IO-heavy tests flaky under parallel load.
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let mut p = home.join(".skillark-repotest");
    std::fs::create_dir_all(&p).unwrap();
    p.push(format!("{}-{}.sqlite", std::process::id(), Uuid::new_v4()));
    let url = format!("sqlite:{}", p.to_string_lossy());
    let pool = connect(&url).await.expect("pool + migrations");
    WorkspaceRepository::new(pool.clone())
        .ensure_global_default()
        .await
        .unwrap();
    pool
}

fn sample_skill(canonical: &str) -> Skill {
    let now = chrono::Utc::now().to_rfc3339();
    Skill {
        id: Uuid::new_v4(),
        canonical_name: canonical.to_owned(),
        display_name: canonical.to_owned(),
        description: String::new(),
        format: "agent-skills".to_owned(),
        library_path: format!("/vault/{canonical}"),
        status: SkillStatus::Ready,
        created_at: now.clone(),
        updated_at: now,
    }
}

fn sample_version(skill_id: Uuid, hash: &str) -> SkillVersion {
    SkillVersion {
        id: Uuid::new_v4(),
        skill_id,
        version_label: Some("1.0.0".to_owned()),
        source_revision: None,
        content_hash: hash.to_owned(),
        manifest_json: "{}".to_owned(),
        library_snapshot_path: format!("/vault/snapshot/{hash}"),
        created_at: chrono::Utc::now().to_rfc3339(),
    }
}

fn sample_record(target: &str, skill_version_id: &str, agent_id: &str) -> DeploymentRecord {
    DeploymentRecord {
        id: Uuid::new_v4().to_string(),
        skill_version_id: skill_version_id.to_owned(),
        agent_id: agent_id.to_owned(),
        workspace_id: GLOBAL_DEFAULT_ID.to_owned(),
        operation_id: None,
        target_path: PathBuf::from(target),
        install_mode: InstallMode::Copy,
        status: DeploymentStatus::Synced,
        deployed_hash: Some("a".repeat(64)),
        source_path_at_install: PathBuf::from("/vault/snapshot/a"),
        installed_at: Some(chrono::Utc::now().to_rfc3339()),
        last_verified_at: None,
        error_message: None,
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// Insert a minimal agent row so the `deployments.agent_id` FK is satisfied.
async fn ensure_agent(pool: SqlitePool, agent_type: &str) -> String {
    AgentRepository::new(pool)
        .upsert_agent(agent_type, agent_type, None, None, 0, false)
        .await
        .unwrap()
        .to_string()
}

// ── OperationRepository ─────────────────────────────────────────────────

#[tokio::test]
async fn operation_lifecycle_runs_to_succeeded() {
    let pool = setup_pool().await;
    let repo = OperationRepository::new(pool);

    let id = Uuid::new_v4().to_string();
    repo.create(&id, OperationType::Install, "{}")
        .await
        .unwrap();

    let running = repo.get(&id).await.unwrap().unwrap();
    assert_eq!(running.status, OperationStatus::Running);
    assert!(running.completed_at.is_none());

    repo.complete(
        &id,
        OperationStatus::Succeeded,
        Some(r#"{"targets":2}"#),
        None,
    )
    .await
    .unwrap();

    let done = repo.get(&id).await.unwrap().unwrap();
    assert_eq!(done.status, OperationStatus::Succeeded);
    assert!(done.completed_at.is_some());

    let recent = repo.list_recent(10).await.unwrap();
    assert!(recent.iter().any(|o| o.id == id));
}

#[tokio::test]
async fn operation_records_failure_message() {
    let pool = setup_pool().await;
    let repo = OperationRepository::new(pool);
    let id = Uuid::new_v4().to_string();
    repo.create(&id, OperationType::Uninstall, "{}")
        .await
        .unwrap();
    repo.complete(
        &id,
        OperationStatus::Failed,
        None,
        Some("target modified"),
    )
    .await
    .unwrap();
    let op = repo.get(&id).await.unwrap().unwrap();
    assert_eq!(op.status, OperationStatus::Failed);
    assert_eq!(op.error_message.as_deref(), Some("target modified"));
}

#[tokio::test]
async fn running_operation_is_recovered_after_restart() {
    let pool = setup_pool().await;
    let repo = OperationRepository::new(pool);
    repo.create("op-interrupted", OperationType::Import, "{}")
        .await
        .unwrap();

    assert_eq!(repo.recover_interrupted().await.unwrap(), 1);
    let recovered = repo.get("op-interrupted").await.unwrap().unwrap();
    assert_eq!(recovered.status, OperationStatus::Failed);
    assert_eq!(recovered.error_code.as_deref(), Some("interrupted"));
    assert!(recovered.completed_at.is_some());
}

// ── WorkspaceRepository ─────────────────────────────────────────────────

#[tokio::test]
async fn global_default_is_idempotent_and_listed() {
    let pool = setup_pool().await;
    let repo = WorkspaceRepository::new(pool);
    repo.ensure_global_default().await.unwrap(); // second insert must not error
    repo.ensure_global_default().await.unwrap();

    let list = repo.list().await.unwrap();
    assert!(list.iter().any(|w| w.id.0 == GLOBAL_DEFAULT_ID));

    let fetched = repo.get(GLOBAL_DEFAULT_ID).await.unwrap().unwrap();
    assert_eq!(fetched.id.0, GLOBAL_DEFAULT_ID);
}

#[tokio::test]
async fn project_workspace_round_trip() {
    let pool = setup_pool().await;
    let repo = WorkspaceRepository::new(pool);
    repo.create_project("proj-1", "My Project", Some(std::path::Path::new("/code/app")))
        .await
        .unwrap();
    let ws = repo.get("proj-1").await.unwrap().unwrap();
    assert_eq!(ws.name, "My Project");
    assert_eq!(ws.root_path.as_deref(), Some(std::path::Path::new("/code/app")));
}

#[tokio::test]
async fn global_workspace_is_protected_from_delete() {
    let pool = setup_pool().await;
    let repo = WorkspaceRepository::new(pool);
    let err = repo.delete(GLOBAL_DEFAULT_ID).await.unwrap_err();
    assert!(err.contains("cannot be deleted"), "got: {err}");
    // Still present.
    assert!(repo.get(GLOBAL_DEFAULT_ID).await.unwrap().is_some());
}

#[tokio::test]
async fn project_workspace_can_be_deleted() {
    let pool = setup_pool().await;
    let repo = WorkspaceRepository::new(pool);
    repo.create_project("proj-2", "Tmp", None).await.unwrap();
    repo.delete("proj-2").await.unwrap();
    assert!(repo.get("proj-2").await.unwrap().is_none());
}

// ── SkillRepository v0.1 extensions ─────────────────────────────────────

#[tokio::test]
async fn skill_version_dedup_reuses_same_hash() {
    let pool = setup_pool().await;
    let repo = SkillRepository::new(pool);
    let skill = sample_skill("dedup");
    repo.create_skill(&skill).await.unwrap();

    assert!(repo.find_by_canonical_name("dedup").await.unwrap().is_some());

    let v1 = sample_version(skill.id, &"c".repeat(64));
    repo.create_skill_version(&v1).await.unwrap();
    repo.set_current_version(skill.id, v1.id).await.unwrap();

    // Same hash → reuse, not a second row.
    let found = repo
        .find_version_by_hash(skill.id, &"c".repeat(64))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found.id, v1.id);

    // A different hash → a separate version.
    let v2 = sample_version(skill.id, &"d".repeat(64));
    repo.create_skill_version(&v2).await.unwrap();
    let versions = repo.list_versions(skill.id).await.unwrap();
    assert_eq!(versions.len(), 2);

    // current_version pointer advances.
    repo.set_current_version(skill.id, v2.id).await.unwrap();
    let latest = repo.get_latest_version(skill.id).await.unwrap().unwrap();
    assert_eq!(latest.id, v2.id);
}

#[tokio::test]
async fn delete_skill_cascades_versions() {
    let pool = setup_pool().await;
    let repo = SkillRepository::new(pool);
    let skill = sample_skill("cascade");
    repo.create_skill(&skill).await.unwrap();
    repo.create_skill_version(&sample_version(skill.id, "e".repeat(64).as_str()))
        .await
        .unwrap();
    assert!(!repo.list_versions(skill.id).await.unwrap().is_empty());

    repo.delete_skill(skill.id).await.unwrap();
    assert!(repo.find_by_canonical_name("cascade").await.unwrap().is_none());
    assert!(repo.list_versions(skill.id).await.unwrap().is_empty());
}

// ── DeploymentRepository ────────────────────────────────────────────────

#[tokio::test]
async fn deployment_upsert_replaces_active_at_target() {
    let pool = setup_pool().await;
    let skill_repo = SkillRepository::new(pool.clone());
    let dep_repo = DeploymentRepository::new(pool.clone());
    let agent_id = ensure_agent(pool.clone(), "codex").await;

    let skill = sample_skill("deploy");
    skill_repo.create_skill(&skill).await.unwrap();
    let version = sample_version(skill.id, &"f".repeat(64));
    skill_repo.create_skill_version(&version).await.unwrap();

    let mut rec = sample_record("/t/a", &version.id.to_string(), &agent_id);
    dep_repo.upsert_active(&rec).await.unwrap();

    // Re-deploy: a new record id at the same target replaces the prior active row.
    rec.id = Uuid::new_v4().to_string();
    dep_repo.upsert_active(&rec).await.unwrap();

    let active = dep_repo
        .find_active_by_target(std::path::Path::new("/t/a"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.id, rec.id);
    assert_eq!(active.status, DeploymentStatus::Synced);

    let by_version = dep_repo.list_by_skill_version(&version.id.to_string()).await.unwrap();
    assert_eq!(by_version.len(), 1, "only one active deployment per target");
}

#[tokio::test]
async fn deployment_status_and_verify_updates() {
    let pool = setup_pool().await;
    let skill_repo = SkillRepository::new(pool.clone());
    let agent_id = ensure_agent(pool.clone(), "codex").await;
    let dep_repo = DeploymentRepository::new(pool);

    let skill = sample_skill("status");
    skill_repo.create_skill(&skill).await.unwrap();
    let version = sample_version(skill.id, &"b".repeat(64));
    skill_repo.create_skill_version(&version).await.unwrap();

    let rec = sample_record("/t/b", &version.id.to_string(), &agent_id);
    dep_repo.upsert_active(&rec).await.unwrap();

    dep_repo
        .set_status(&rec.id, DeploymentStatus::Modified, Some("user edit"))
        .await
        .unwrap();
    let after = dep_repo
        .find_active_by_target(std::path::Path::new("/t/b"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(after.status, DeploymentStatus::Modified);
    assert_eq!(after.error_message.as_deref(), Some("user edit"));

    dep_repo.mark_verified(&rec.id, "2026-01-01T00:00:00Z").await.unwrap();
    let verified = dep_repo
        .find_active_by_target(std::path::Path::new("/t/b"))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(verified.last_verified_at.as_deref(), Some("2026-01-01T00:00:00Z"));
}
