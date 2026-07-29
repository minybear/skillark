//! End-to-end service test: the v0.1 value loop.
//!
//! import directory → build plan → execute (copy) → verify synced → modify →
//! verify modified → uninstall. Exercises ImportSkillService, PlanDeployment,
//! ExecuteDeployment, VerifyDeployment, and UninstallDeployment against a real
//! temp vault and temp database.

use std::path::PathBuf;

use skillark_lib::{
    adapters::sqlite::connect,
    application::{
        execute_deployment::ExecuteDeploymentService,
        import_skill::ImportSkillService,
        plan_deployment::{PlanDeploymentService, PlanTargetSpec},
        state::AppState,
        uninstall_deployment::UninstallDeploymentService,
        verify_deployment::VerifyDeploymentService,
    },
    domain::{deployment::InstallMode, workspace::WorkspaceId},
    repositories::{AgentRepository, WorkspaceRepository},
};

async fn setup() -> AppState {
    let root = unique_dir();
    let db_url = format!("sqlite:{}", root.join("db.sqlite").to_string_lossy());
    let pool = connect(&db_url).await.unwrap();
    WorkspaceRepository::new(pool.clone())
        .ensure_global_default()
        .await
        .unwrap();
    let vault = root.join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    AppState::new(pool, vault)
}

fn unique_dir() -> PathBuf {
    // Base under the user home, not Temp: the host AV/EDR heavily instruments the
    // Temp dir (it also blocks reparse creation there), which makes fs-heavy
    // e2e tests flaky under parallel load. Real vaults live under the home too.
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let p = home
        .join(".skillark-e2e-test")
        .join(format!("{}-{}", std::process::id(), uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn make_source_skill(dir: &std::path::Path, name: &str) {
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\nversion: 1.0.0\ndescription: e2e\n---\n# {name}\nbody"),
    )
    .unwrap();
    std::fs::write(dir.join("scripts/run.sh"), "echo hi").unwrap();
}

#[tokio::test]
async fn import_plan_execute_verify_uninstall_loop() {
    let state = setup().await;
    let root = state.vault_path.parent().unwrap().to_path_buf();

    // 1. Import a directory skill.
    let source = root.join("source/my-skill");
    make_source_skill(&source, "my-skill");
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let outcome = importer.import_directory(source.clone()).await.unwrap();
    assert!(!outcome.reused_version, "first import creates a new version");

    // 2. Re-import identical content → version dedup.
    let dup = importer.import_directory(source.clone()).await.unwrap();
    assert!(dup.reused_version, "identical content reuses the version");
    assert_eq!(dup.version_id, outcome.version_id);
    let import_operations = state
        .operations()
        .list_recent(10)
        .await
        .unwrap()
        .into_iter()
        .filter(|operation| {
            operation.operation_type
                == skillark_lib::domain::operation::OperationType::Import
        })
        .collect::<Vec<_>>();
    assert_eq!(import_operations.len(), 2);
    assert!(import_operations.iter().all(|operation| {
        operation.status
            == skillark_lib::domain::operation::OperationStatus::Succeeded
            && operation.result_json.is_some()
    }));

    // 3. Load the version to confirm the snapshot was materialized.
    let version = state
        .skills()
        .get_version(outcome.version_id)
        .await
        .unwrap()
        .unwrap();
    assert!(std::path::Path::new(&version.library_snapshot_path).exists());

    // 4. Ensure an agent row exists and build a plan for its skill dir.
    AgentRepository::new(state.pool.clone())
        .upsert_agent("codex", "Codex", None, None, 0, false)
        .await
        .unwrap();
    let target_root = root.join("agents/codex/skills");
    std::fs::create_dir_all(&target_root).unwrap();
    let target_path = target_root.join("my-skill");

    let plan = PlanDeploymentService::new(&state)
        .build_plan(skillark_lib::application::plan_deployment::PlanRequest {
            skill_version_id: outcome.version_id.to_string(),
            targets: vec![PlanTargetSpec {
                agent_id: "codex".to_owned(),
                workspace_id: WorkspaceId("global-default".to_owned()),
                target_path: target_path.clone(),
                mode: InstallMode::Copy,
            }],
        })
        .await
        .unwrap();
    assert!(!plan.requires_confirmation, "fresh target needs no confirmation");
    assert_eq!(plan.targets[0].conflict.as_str(), "none");

    // 5. Execute → installed at target.
    let report = ExecuteDeploymentService::new(&state)
        .execute(plan)
        .await
        .unwrap();
    assert_eq!(report.succeeded, 1);
    assert_eq!(report.failed, 0);
    assert!(target_path.join("SKILL.md").is_file());

    // 6. Verify → synced.
    let verify = VerifyDeploymentService::new(&state).verify_all().await.unwrap();
    assert_eq!(verify.len(), 1);
    assert_eq!(verify[0].status, "synced");

    let deployment = state
        .deployments()
        .find_active_by_target(&target_path)
        .await
        .unwrap()
        .unwrap();

    // 7. Modify the target → verify reports modified.
    std::fs::write(
        target_path.join("SKILL.md"),
        "---\nname: my-skill\nversion: 1.0.0\n---\nEDITED",
    )
    .unwrap();
    let verify2 = VerifyDeploymentService::new(&state)
        .verify_one(&deployment.id)
        .await
        .unwrap();
    assert_eq!(verify2.status, "modified");

    // 8. Uninstall refuses the modified target without force, then succeeds with force.
    let uninstall_service = UninstallDeploymentService::new(&state);
    let refused = uninstall_service.uninstall(&deployment.id, false).await.unwrap();
    assert!(!refused.removed_target, "modified copy must not be silently deleted");
    assert!(target_path.exists());

    let removed = uninstall_service.uninstall(&deployment.id, true).await.unwrap();
    assert!(removed.removed_target);
    assert!(!target_path.exists());
}

#[tokio::test]
async fn import_zip_creates_version_and_rejects_slip() {
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    let state = setup().await;
    let root = state.vault_path.parent().unwrap().to_path_buf();

    // Build a valid skill zip.
    let zip_path = root.join("good.zip");
    {
        let f = std::fs::File::create(&zip_path).unwrap();
        let mut zw = ZipWriter::new(f);
        let opts = SimpleFileOptions::default();
        zw.start_file("good/SKILL.md", opts).unwrap();
        zw.write_all(b"---\nname: good\nversion: 1.0.0\n---\nbody").unwrap();
        zw.start_file("good/scripts/run.sh", opts).unwrap();
        zw.write_all(b"echo").unwrap();
        zw.finish().unwrap();
    }

    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let outcome = importer.import_zip(zip_path).await.unwrap();
    assert_eq!(outcome.canonical_name, "good");
    assert!(state.vault_path.join("good").exists());

    // A malicious zip must be rejected and produce no version.
    let bad_zip = root.join("bad.zip");
    {
        let f = std::fs::File::create(&bad_zip).unwrap();
        let mut zw = ZipWriter::new(f);
        zw.start_file("../escape.txt", SimpleFileOptions::default())
            .unwrap();
        zw.write_all(b"pwned").unwrap();
        zw.finish().unwrap();
    }
    let err = importer.import_zip(bad_zip).await.unwrap_err();
    assert!(err.contains("Zip Slip") || err.contains("escapes"), "got: {err}");
    let failed_import = state
        .operations()
        .list_recent(1)
        .await
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(
        failed_import.operation_type,
        skillark_lib::domain::operation::OperationType::Import
    );
    assert_eq!(
        failed_import.status,
        skillark_lib::domain::operation::OperationStatus::Failed
    );
    assert!(failed_import.error_message.is_some());
}
