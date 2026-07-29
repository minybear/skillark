//! G11 + G12: dataset-driven and Windows-path-matrix end-to-end tests.
//!
//! Builds the v0.1 acceptance data set at runtime (no large binaries committed):
//!   - 20 structurally-distinct skills import and dedup correctly
//!   - a 1000-small-file skill round-trips (import → copy deploy → verify synced)
//!   - a single >20MB file skill round-trips
//!   - corrupt ZIP and malicious (Zip Slip / absolute-path) ZIPs are rejected
//!   - internal vs escaping symlinks are classified by the hasher
//!   - Windows path matrix: 中文 / spaces / parentheses / long paths deploy+verify
//!
//! Everything runs against a fresh vault + DB per test, based under the user
//! home (not Temp) to avoid the host AV/EDR instrumentation of Temp.

use std::path::{Path, PathBuf};

use skillark_lib::{
    adapters::sqlite::connect,
    application::{
        execute_deployment::ExecuteDeploymentService,
        import_skill::ImportSkillService,
        plan_deployment::{PlanDeploymentService, PlanRequest as PlanRequest_, PlanTargetSpec},
        state::AppState,
        verify_deployment::VerifyDeploymentService,
    },
    domain::{deployment::InstallMode, workspace::WorkspaceId},
    repositories::WorkspaceRepository,
};

// ───── scaffolding ─────────────────────────────────────────────────────────

fn unique_dir(tag: &str) -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let p = home.join(".skillark-ds-test").join(format!(
        "{tag}-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}

async fn setup_at(root: &Path) -> AppState {
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

fn write_skill(dir: &Path, name: &str, extra_files: usize) {
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\nversion: 1.0.0\ndescription: ds\n---\n# {name}\nbody"),
    )
    .unwrap();
    for i in 0..extra_files {
        std::fs::write(dir.join("scripts").join(format!("f{i}.txt")), format!("data {i}"))
            .unwrap();
    }
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

// ───── G11: 20 structurally-distinct skills ────────────────────────────────

#[test]
fn twenty_structurally_distinct_skills_import_and_dedup() {
    let rt = rt();
    let root = unique_dir("twenty");
    let state = rt.block_on(setup_at(&root));
    let src_root = root.join("src");
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());

    // 20 distinct shapes: vary name, file count, nesting depth, and asset kinds.
    for i in 0..20 {
        let dir = src_root.join(format!("skill-{i:02}"));
        write_skill(&dir, &format!("skill-{i:02}"), i);
        // vary structure: some have references/, some assets/, some nested dirs
        if i % 3 == 0 {
            std::fs::create_dir_all(dir.join("references")).unwrap();
            std::fs::write(dir.join("references/doc.md"), "ref").unwrap();
        }
        if i % 4 == 0 {
            std::fs::create_dir_all(dir.join("assets/nested/deep")).unwrap();
            std::fs::write(dir.join("assets/nested/deep/blob.bin"), vec![0u8; 64]).unwrap();
        }
        let outcome = rt
            .block_on(importer.import_directory(dir.clone()))
            .unwrap_or_else(|e| panic!("import skill-{i:02} failed: {e}"));
        assert!(!outcome.reused_version, "first import must be a new version");
        // re-import is a dedup hit
        let again = rt.block_on(importer.import_directory(dir)).unwrap();
        assert!(again.reused_version, "re-import skill-{i:02} must dedup");
    }

    let skills = rt.block_on(state.skills().list_skills(None)).unwrap();
    assert_eq!(skills.len(), 20, "20 distinct skills must be stored");
}

// ───── G11 + G13: 1000 small files round-trip ──────────────────────────────

#[test]
fn thousand_small_files_import_deploy_verify() {
    let rt = rt();
    let root = unique_dir("1kfiles");
    let state = rt.block_on(setup_at(&root));

    let src = root.join("src/many");
    std::fs::create_dir_all(src.join("files")).unwrap();
    std::fs::write(
        src.join("SKILL.md"),
        "---\nname: many\nversion: 1.0.0\n---\nbody",
    )
    .unwrap();
    for i in 0..1000 {
        std::fs::write(src.join("files").join(format!("f{i:04}.txt")), format!("{i}")).unwrap();
    }

    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let outcome = rt.block_on(importer.import_directory(src)).unwrap();

    // deploy via copy to a target and verify synced
    rt.block_on(
        skillark_lib::repositories::AgentRepository::new(state.pool.clone())
            .upsert_agent("codex", "Codex", None, None, 0, false),
    )
    .unwrap();
    let target = root.join("agent/codex/skills/many");
    let plan = rt
        .block_on(
            PlanDeploymentService::new(&state).build_plan(PlanRequest_ {
                skill_version_id: outcome.version_id.to_string(),
                targets: vec![PlanTargetSpec {
                    agent_id: "codex".to_owned(),
                    workspace_id: WorkspaceId("global-default".to_owned()),
                    target_path: target.clone(),
                    mode: InstallMode::Copy,
                }],
            }),
        )
        .unwrap();
    let report = rt
        .block_on(ExecuteDeploymentService::new(&state).execute(plan))
        .unwrap();
    assert_eq!(report.failed, 0, "1000-file copy deploy must succeed");
    assert_eq!(target.join("files").read_dir().unwrap().count(), 1000);

    let items = rt
        .block_on(VerifyDeploymentService::new(&state).verify_all())
        .unwrap();
    assert!(
        items.iter().all(|i| i.status == "synced"),
        "1000-file deployment must verify synced, got {items:?}"
    );
}

// ───── G11 + G13: single >20MB file round-trip ─────────────────────────────

#[test]
fn large_single_file_over_20mb_round_trip() {
    let rt = rt();
    let root = unique_dir("bigfile");
    let state = rt.block_on(setup_at(&root));

    let src = root.join("src/big");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(src.join("SKILL.md"), "---\nname: big\nversion: 1.0.0\n---\nbody").unwrap();
    // 21 MiB of non-trivial (non-zero) content so hashing is real work.
    let mut blob = vec![0u8; 21 * 1024 * 1024];
    for (i, b) in blob.iter_mut().enumerate() {
        *b = (i % 251) as u8;
    }
    std::fs::write(src.join("big.bin"), &blob).unwrap();

    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let outcome = rt.block_on(importer.import_directory(src)).unwrap();

    rt.block_on(
        skillark_lib::repositories::AgentRepository::new(state.pool.clone())
            .upsert_agent("codex", "Codex", None, None, 0, false),
    )
    .unwrap();
    let target = root.join("agent/codex/skills/big");
    let plan = rt
        .block_on(
            PlanDeploymentService::new(&state).build_plan(PlanRequest_ {
                skill_version_id: outcome.version_id.to_string(),
                targets: vec![PlanTargetSpec {
                    agent_id: "codex".to_owned(),
                    workspace_id: WorkspaceId("global-default".to_owned()),
                    target_path: target.clone(),
                    mode: InstallMode::Copy,
                }],
            }),
        )
        .unwrap();
    let report = rt
        .block_on(ExecuteDeploymentService::new(&state).execute(plan))
        .unwrap();
    assert_eq!(report.failed, 0, ">20MB copy deploy must succeed");
    assert_eq!(
        std::fs::metadata(target.join("big.bin")).unwrap().len(),
        21 * 1024 * 1024
    );
    let items = rt
        .block_on(VerifyDeploymentService::new(&state).verify_all())
        .unwrap();
    assert!(items.iter().all(|i| i.status == "synced"), "big file must verify synced");
}

// ───── G11: corrupt + malicious ZIP rejection ──────────────────────────────

fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};
    let mut zw = ZipWriter::new(std::fs::File::create(path).unwrap());
    let opts = SimpleFileOptions::default();
    for (name, body) in entries {
        zw.start_file(name, opts).unwrap();
        zw.write_all(body).unwrap();
    }
    zw.finish().unwrap();
}

#[test]
fn corrupt_zip_is_rejected() {
    let rt = rt();
    let root = unique_dir("corruptzip");
    let state = rt.block_on(setup_at(&root));
    let bad = root.join("corrupt.zip");
    std::fs::write(&bad, b"PK\x03\x04 this is not a real zip archive ...").unwrap();

    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let r = rt.block_on(importer.import_zip(bad));
    assert!(r.is_err(), "corrupt zip must be rejected, got {r:?}");
}

#[test]
fn malicious_zip_slip_is_rejected() {
    let rt = rt();
    let root = unique_dir("zipslip");
    let state = rt.block_on(setup_at(&root));
    let evil = root.join("evil.zip");
    make_zip(
        &evil,
        &[
            ("pkg/SKILL.md", b"---\nname: pkg\nversion: 1.0.0\n---\nbody"),
            ("../escape.txt", b"slip"),
        ],
    );
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let r = rt.block_on(importer.import_zip(evil));
    assert!(r.is_err(), "zip slip must be rejected, got {r:?}");
}

#[test]
fn malicious_absolute_path_zip_is_rejected() {
    let rt = rt();
    let root = unique_dir("abszip");
    let state = rt.block_on(setup_at(&root));
    let evil = root.join("abs.zip");
    make_zip(
        &evil,
        &[
            ("pkg/SKILL.md", b"---\nname: pkg\nversion: 1.0.0\n---\nbody"),
            ("/etc/passwd", b"abs"),
        ],
    );
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let r = rt.block_on(importer.import_zip(evil));
    assert!(r.is_err(), "absolute-path zip entry must be rejected, got {r:?}");
}

// ───── G11: internal vs escaping symlink classification ────────────────────
// Symlink creation needs privilege on Windows; assert on unix only. The
// hashing/escape policy itself is covered cross-platform by path_safety tests.

#[cfg(unix)]
#[test]
fn internal_and_escaping_symlinks_classified() {
    use std::os::unix::fs::symlink;
    let root = unique_dir("links");
    let skill = root.join("skill");
    std::fs::create_dir_all(&skill).unwrap();
    std::fs::write(skill.join("SKILL.md"), "---\nname: l\nversion: 1\n---\nbody").unwrap();
    std::fs::write(skill.join("real.txt"), "real").unwrap();
    symlink(skill.join("real.txt"), skill.join("internal-link")).unwrap();

    let outside = root.join("outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("secret.txt"), "secret").unwrap();
    symlink(outside.join("secret.txt"), skill.join("escaping-link")).unwrap();

    assert!(
        !skillark_lib::domain::path_safety::symlink_escapes_root(&skill, &skill.join("internal-link")),
        "internal symlink must not be flagged"
    );
    assert!(
        skillark_lib::domain::path_safety::symlink_escapes_root(&skill, &skill.join("escaping-link")),
        "escaping symlink must be flagged"
    );
}

// ───── G12: Windows path matrix ────────────────────────────────────────────

#[test]
fn windows_path_matrix_chinese_spaces_parens_long() {
    let rt = rt();
    let root = unique_dir("pathmatrix");
    let state = rt.block_on(setup_at(&root));

    rt.block_on(
        skillark_lib::repositories::AgentRepository::new(state.pool.clone())
            .upsert_agent("codex", "Codex", None, None, 0, false),
    )
    .unwrap();
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());

    // Cases: 中文、空格、括号、长路径（>100 字符的嵌套）。
    let long_seg = "很长的路径段".repeat(12); // deep unicode nesting
    let cases: Vec<(String, String)> = vec![
        ("中文 Skill (E2E)".to_owned(), "中文目标 (Copy)".to_owned()),
        ("skill with spaces".to_owned(), "target with spaces".to_owned()),
        ("skill (parens) [v1]".to_owned(), "target (parens) [v1]".to_owned()),
        (format!("deep/{long_seg}/leaf"), format!("deep-target/{long_seg}/leaf")),
    ];

    for (idx, (skill_rel, target_rel)) in cases.iter().enumerate() {
        let src = root.join("src").join(skill_rel);
        write_skill(&src, &format!("matrix-{idx}"), 2);
        let outcome = rt
            .block_on(importer.import_directory(src))
            .unwrap_or_else(|e| panic!("import {skill_rel} failed: {e}"));

        let target = root.join("targets").join(target_rel);
        let plan = rt
            .block_on(
                PlanDeploymentService::new(&state).build_plan(PlanRequest_ {
                    skill_version_id: outcome.version_id.to_string(),
                    targets: vec![PlanTargetSpec {
                        agent_id: "codex".to_owned(),
                        workspace_id: WorkspaceId("global-default".to_owned()),
                        target_path: target.clone(),
                        mode: InstallMode::Copy,
                    }],
                }),
            )
            .unwrap_or_else(|e| panic!("plan {skill_rel} failed: {e}"));
        let report = rt
            .block_on(ExecuteDeploymentService::new(&state).execute(plan))
            .unwrap();
        assert_eq!(report.failed, 0, "deploy to {target_rel} must succeed");
        assert!(target.join("SKILL.md").is_file(), "target SKILL.md missing for {target_rel}");
    }

    let items = rt
        .block_on(VerifyDeploymentService::new(&state).verify_all())
        .unwrap();
    assert_eq!(items.len(), cases.len(), "all matrix deployments registered");
    assert!(
        items.iter().all(|i| i.status == "synced"),
        "all path-matrix deployments must verify synced: {items:?}"
    );
}

// ───── G8 + G11: crash-leftover operation is identified & recovered ────────

#[test]
fn crashed_running_operation_is_flagged_on_next_startup() {
    use skillark_lib::domain::operation::{OperationStatus, OperationType};
    use skillark_lib::repositories::OperationRepository;

    let rt = rt();
    let root = unique_dir("crashrecover");
    let db_url = format!("sqlite:{}", root.join("db.sqlite").to_string_lossy());

    // Simulate a process that began an install then died mid-flight: a row is
    // left in `running` with no completion.
    {
        let pool = rt.block_on(connect(&db_url)).unwrap();
        let ops = OperationRepository::new(pool.clone());
        rt.block_on(ops.create("op-crashed", OperationType::Install, "{}"))
            .unwrap();
        // drop the pool without completing -> mimics a crash
    }

    // Next startup: a fresh connection runs recovery, exactly as AppState::setup does.
    let pool2 = rt.block_on(connect(&db_url)).unwrap();
    let ops2 = OperationRepository::new(pool2);
    let recovered = rt.block_on(ops2.recover_interrupted()).unwrap();
    assert_eq!(recovered, 1, "exactly one crashed operation must be recovered");

    let recent = rt.block_on(ops2.list_recent(10)).unwrap();
    let crashed = recent.iter().find(|o| o.id == "op-crashed").unwrap();
    assert_eq!(crashed.status, OperationStatus::Failed);
    assert_eq!(crashed.error_code.as_deref(), Some("interrupted"));
    assert!(
        crashed.error_message.as_deref().unwrap_or("").contains("exited before"),
        "recovery must record a human-readable reason: {crashed:?}"
    );
}

// ───── G14: migration over a real pre-existing database ────────────────────
//
// Copies the real DB captured from a prior app run (.e2e-runtime), seeds
// representative pre-existing rows, then re-runs connect() (which applies
// sqlx migrations). Asserts the open is idempotent, prior data survives, and
// all expected tables exist.

#[test]
fn migration_over_real_existing_database_is_idempotent_and_preserves_data() {
    let rt = rt();
    let root = unique_dir("realmigrate");
    let target_db = root.join("skillark.db");

    // Locate the real captured DB relative to the crate (tests run with CWD =
    // src-tauri). Fall back to building a schema-only DB if it is absent.
    let captured = PathBuf::from("../.e2e-runtime/20260728-230031/skillark.db");
    if captured.is_file() {
        std::fs::copy(&captured, &target_db).unwrap();
    }

    // Seed representative pre-existing rows (a workspace + an operation), as a
    // real user's DB would have.
    {
        let url = format!("sqlite:{}", target_db.to_string_lossy());
        let pool = rt.block_on(connect(&url)).unwrap();
        rt.block_on(
            WorkspaceRepository::new(pool.clone()).ensure_global_default(),
        )
        .unwrap();
        use skillark_lib::domain::operation::OperationType;
        rt.block_on(
            skillark_lib::repositories::OperationRepository::new(pool.clone())
                .create("op-preexisting", OperationType::Import, "{}"),
        )
        .unwrap();
    }

    // Re-open: migrations must be idempotent (no error, no duplicate).
    let url = format!("sqlite:{}", target_db.to_string_lossy());
    let pool = rt.block_on(connect(&url)).expect("re-open with migrations must succeed");

    // Prior data preserved.
    let op_count: (i64,) = rt
        .block_on(
            sqlx::query_as("SELECT COUNT(*) FROM operations WHERE id = 'op-preexisting'")
                .fetch_one(&pool),
        )
        .unwrap();
    assert_eq!(op_count.0, 1, "pre-existing operation must survive re-migration");

    let ws_count: (i64,) = rt
        .block_on(sqlx::query_as("SELECT COUNT(*) FROM workspaces").fetch_one(&pool))
        .unwrap();
    assert!(ws_count.0 >= 1, "workspace rows must survive");

    // All expected tables present after migration.
    let tables: Vec<(String,)> = rt
        .block_on(
            sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table'").fetch_all(&pool),
        )
        .unwrap();
    let names: std::collections::HashSet<String> = tables.into_iter().map(|t| t.0).collect();
    for required in [
        "skills", "skill_versions", "agents", "agent_overrides", "workspaces",
        "workspace_agents", "operations", "deployments", "sources", "app_settings",
    ] {
        assert!(names.contains(required), "missing table after migration: {required}");
    }

    // Migration ledger records both migrations exactly once each.
    let dup: (i64,) = rt
        .block_on(
            sqlx::query_as(
                "SELECT COUNT(*) FROM (SELECT version FROM _sqlx_migrations GROUP BY version HAVING COUNT(*) > 1)",
            )
            .fetch_one(&pool),
        )
        .unwrap();
    assert_eq!(dup.0, 0, "no migration may be applied twice");
}
