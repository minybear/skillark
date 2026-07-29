//! G13: performance & scale benchmarks, asserted against the v0.1 gates.
//!
//!   - 500 skills: first list load < 2s
//!   - 100 deployment targets: verify_all < 5s
//!   - hash of a large directory is interruptible / reports progress (cancel)
//!
//! These are real end-to-end timings against a real vault + DB, based under the
//! user home (not Temp) to avoid host AV/EDR skew. They run single-threaded so
//! the numbers reflect the code, not scheduler contention.

use std::path::{Path, PathBuf};
use std::time::Instant;

use skillark_lib::{
    adapters::sqlite::connect,
    application::{
        execute_deployment::ExecuteDeploymentService,
        import_skill::ImportSkillService,
        plan_deployment::{PlanDeploymentService, PlanRequest, PlanTargetSpec},
        state::AppState,
        verify_deployment::VerifyDeploymentService,
    },
    domain::{deployment::InstallMode, workspace::WorkspaceId},
    repositories::WorkspaceRepository,
};

fn unique_dir(tag: &str) -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let p = home.join(".skillark-perf-test").join(format!(
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

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn write_skill(dir: &Path, name: &str) {
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\nversion: 1.0.0\ndescription: perf\n---\n# {name}\nbody"),
    )
    .unwrap();
    std::fs::write(dir.join("scripts/run.sh"), "echo hi").unwrap();
}

/// 500 skills listed in under 2 seconds (first load = cold query, no cache).
#[test]
fn list_500_skills_under_two_seconds() {
    let rt = rt();
    let root = unique_dir("list500");
    let state = rt.block_on(setup_at(&root));
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());

    // Seed 500 distinct skills.
    let seed_start = Instant::now();
    for i in 0..500 {
        let dir = root.join("src").join(format!("skill-{i:03}"));
        write_skill(&dir, &format!("skill-{i:03}"));
        rt.block_on(importer.import_directory(dir)).unwrap();
    }
    let seed_ms = seed_start.elapsed().as_millis();

    // Cold first list (drop any repo instance, fresh query).
    let list_start = Instant::now();
    let skills = rt.block_on(state.skills().list_skills(None)).unwrap();
    let list_ms = list_start.elapsed().as_millis();

    assert_eq!(skills.len(), 500);
    println!(
        "PERF list_500: seed={}ms first_list={}ms (gate <2000ms)",
        seed_ms, list_ms
    );
    assert!(
        list_ms < 2000,
        "listing 500 skills took {list_ms}ms, gate is <2000ms"
    );
}

/// 100 deployment targets verified in under 5 seconds.
#[test]
fn verify_100_targets_under_five_seconds() {
    let rt = rt();
    let root = unique_dir("verify100");
    let state = rt.block_on(setup_at(&root));
    let importer = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());

    rt.block_on(
        skillark_lib::repositories::AgentRepository::new(state.pool.clone())
            .upsert_agent("codex", "Codex", None, None, 0, false),
    )
    .unwrap();

    // One shared skill version deployed to 100 distinct targets.
    let src = root.join("src/shared");
    write_skill(&src, "shared");
    let outcome = rt.block_on(importer.import_directory(src)).unwrap();

    let targets: Vec<PlanTargetSpec> = (0..100)
        .map(|i| PlanTargetSpec {
            agent_id: "codex".to_owned(),
            workspace_id: WorkspaceId("global-default".to_owned()),
            target_path: root.join("targets").join(format!("t-{i:03}")),
            mode: InstallMode::Copy,
        })
        .collect();

    let plan = rt
        .block_on(
            PlanDeploymentService::new(&state).build_plan(PlanRequest {
                skill_version_id: outcome.version_id.to_string(),
                targets,
            }),
        )
        .unwrap();
    let report = rt
        .block_on(ExecuteDeploymentService::new(&state).execute(plan))
        .unwrap();
    assert_eq!(report.failed, 0, "all 100 deploys must succeed before verify");

    let verify_start = Instant::now();
    let items = rt
        .block_on(VerifyDeploymentService::new(&state).verify_all())
        .unwrap();
    let verify_ms = verify_start.elapsed().as_millis();

    assert_eq!(items.len(), 100);
    assert!(items.iter().all(|i| i.status == "synced"));
    println!(
        "PERF verify_100: verify={}ms (gate <5000ms)",
        verify_ms
    );
    assert!(
        verify_ms < 5000,
        "verifying 100 targets took {verify_ms}ms, gate is <5000ms"
    );
}
