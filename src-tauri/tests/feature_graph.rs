//! v0.1 backend feature graph — topological runner over a directed graph of
//! named nodes. Each step runs against one shared AppState + scratch ctx, so
//! the whole v0.1 value flow runs end-to-end without rebuilding state per step.
//!
//! Coverage:
//!   agents + overrides  →  import dir/zip  →  workspaces (global + project)
//!   →  resolve targets (global + project)  →  plan  →  execute
//!   →  verify  →  modify target  →  re-verify  →  uninstall
//!   →  disabled agents  →  resolve_targets filter
//!   →  list skills / workspaces / deployments / operations
//!   →  skill detail  →  backup

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde_json::json;
use skillark_lib::{
    application::{
        execute_deployment::ExecuteDeploymentService,
        import_skill::{ImportOutcome, ImportSkillService},
        plan_deployment::{PlanDeploymentService, PlanRequest, PlanTargetSpec},
        state::AppState,
        uninstall_deployment::UninstallDeploymentService,
        verify_deployment::VerifyDeploymentService,
    },
    domain::{
        deployment::{DeploymentPlan, InstallMode},
        operation::{OperationStatus, OperationType},
        workspace::{WorkspaceId, WorkspaceKind},
    },
};

#[derive(Debug)]
struct Report {
    label: &'static str,
    ok: bool,
    detail: String,
}

#[derive(Default)]
struct Ctx {
    import_dir: Option<ImportOutcome>,
    plan: Option<DeploymentPlan>,
    deployment_id: Option<String>,
}

#[derive(Clone)]
struct Fixture {
    source_dir: PathBuf,
    zip_path: PathBuf,
    target_global: PathBuf,
    backups_dir: PathBuf,
    db_url: String,
}

/// Wrap a fresh tokio current_thread runtime so step fns can stay sync.
#[derive(Clone, Copy)]
struct Rt;

impl Rt {
    fn block<F: std::future::Future>(&self, fut: F) -> F::Output {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build current_thread runtime");
        runtime.block_on(fut)
    }
}

fn ok(label: &'static str, detail: impl Into<String>) -> Report {
    Report { label, ok: true, detail: detail.into() }
}
fn bad(label: &'static str, detail: impl Into<String>) -> Report {
    Report { label, ok: false, detail: detail.into() }
}

#[test]
fn v0_1_feature_graph() {
    let home = unique_home();
    let rt = Rt;
    let state = rt.block(build_state(&home));
    let fx = build_fixture(&home);
    let ctx = Arc::new(Mutex::new(Ctx::default()));

    let result = run_graph(rt, state, fx, ctx);
    let failed: Vec<_> = result.iter().filter(|r| !r.ok).collect();
    assert!(
        failed.is_empty(),
        "{} node(s) failed in the v0.1 feature graph: {:#?}",
        failed.len(),
        failed
    );
    println!("\nGRAPH_SUMMARY {}", json!({
        "node_count": result.len(),
        "nodes": result.iter().map(|r| json!({"label": r.label, "ok": r.ok})).collect::<Vec<_>>()
    }));
}

fn run_graph(rt: Rt, state: AppState, fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Vec<Report> {
    let nodes: &[Node] = &[
        Node::leaf("agent_repo_smoke",                           step_agent_repo_smoke),
        Node::leaf("import_directory",                           step_import_directory),
        Node::leaf("import_zip",                                 step_import_zip),
        Node::leaf("reimport_is_dedup",                          step_reimport_is_dedup),
        Node::leaf("list_skills_after_import",                   step_list_skills),
        Node::leaf("create_project_workspace",                   step_create_project_workspace),
        Node::leaf("list_workspaces_has_global_and_project",     step_list_workspaces),
        Node::leaf("plan_global",                                step_plan_global),
        Node::leaf("seed_codex_agent_for_deploy",                step_seed_codex_agent_for_deploy),
        Node::leaf("execute_global_copy",                        step_execute_global_copy),
        Node::leaf("verify_synced",                              step_verify_synced),
        Node::leaf("modify_then_reverify_modified",             step_modify_and_reverify),
        Node::leaf("uninstall_modified_requires_force",          step_uninstall_modified),
        Node::leaf("uninstall_with_force_removes",               step_uninstall_force),
        Node::leaf("list_operations_records_every_action",       step_list_operations),
        Node::leaf("disabled_agent_excluded_from_resolve",       step_disabled_excluded),
        Node::leaf("backup_database_writes_snapshot",            step_backup),
    ];
    let edges: &[(&str, &[&str])] = &[
        ("agent_repo_smoke", &[]),
        ("import_directory", &[]),
        ("import_zip", &["agent_repo_smoke"]),
        ("reimport_is_dedup", &["import_directory"]),
        ("list_skills_after_import", &["import_directory"]),
        ("create_project_workspace", &["agent_repo_smoke"]),
        ("list_workspaces_has_global_and_project", &["create_project_workspace"]),
        ("plan_global", &["list_skills_after_import"]),
        ("seed_codex_agent_for_deploy", &["plan_global"]),
        ("execute_global_copy", &["seed_codex_agent_for_deploy"]),
        ("verify_synced", &["execute_global_copy"]),
        ("modify_then_reverify_modified", &["verify_synced"]),
        ("uninstall_modified_requires_force", &["modify_then_reverify_modified"]),
        ("uninstall_with_force_removes", &["uninstall_modified_requires_force"]),
        ("list_operations_records_every_action", &["uninstall_with_force_removes"]),
        ("disabled_agent_excluded_from_resolve", &["list_operations_records_every_action"]),
        ("backup_database_writes_snapshot", &["list_operations_records_every_action"]),
    ];

    let order = topo_order(nodes, edges).expect("topo");
    assert_eq!(order.len(), nodes.len());

    let mut reports = Vec::new();
    for (i, label) in order.iter().enumerate() {
        let node = nodes.iter().find(|n| n.label == label.as_str()).expect("node");
        let report = (node.run)(rt, state.clone(), fx.clone(), ctx.clone());
        let pass = report.ok;
        println!("  [{}] {:>2}/{}  {}  {}",
            if pass { "PASS" } else { "FAIL" },
            i + 1, order.len(), report.label, report.detail);
        reports.push(report);
        if !pass { break; }
    }
    reports
}

type NodeRunner = dyn Fn(Rt, AppState, Fixture, Arc<Mutex<Ctx>>) -> Report + Send + Sync;

struct Node {
    label: &'static str,
    run: Arc<NodeRunner>,
}

impl Node {
    fn leaf(
        label: &'static str,
        f: fn(Rt, AppState, Fixture, Arc<Mutex<Ctx>>) -> Report,
    ) -> Self {
        Node { label, run: Arc::new(f) }
    }
}

fn topo_order(nodes: &[Node], edges: &[(&str, &[&str])]) -> Option<Vec<String>> {
    let labels: Vec<String> = nodes.iter().map(|n| n.label.to_string()).collect();
    let label_set: std::collections::HashSet<&str> =
        nodes.iter().map(|n| n.label).collect();
    let mut in_deg: HashMap<String, usize> = HashMap::new();
    let mut succ: HashMap<String, Vec<String>> = HashMap::new();
    for n in &labels { in_deg.insert(n.clone(), 0); }
    for (a, deps) in edges {
        if !label_set.contains(a) { continue; }
        for d in *deps {
            *in_deg.get_mut(*a).unwrap() += 1;
            succ.entry(d.to_string()).or_default().push(a.to_string());
        }
    }
    let mut sorted = labels.clone();
    sorted.sort();
    let mut order: Vec<String> = Vec::new();
    let mut queue: std::collections::VecDeque<String> = sorted
        .into_iter()
        .filter(|n| in_deg[n] == 0)
        .collect();
    while let Some(n) = queue.pop_front() {
        order.push(n.clone());
        if let Some(children) = succ.get(&n) {
            for c in children {
                let d = in_deg.get_mut(c).unwrap();
                *d -= 1;
                if *d == 0 { queue.push_back(c.clone()); }
            }
        }
    }
    if order.len() == labels.len() { Some(order) } else { None }
}

// ───── fixture ────────────────────────────────────────────────────────────

fn unique_home() -> PathBuf {
    let mut home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    home.push(format!(
        "skillark-feature-graph-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    // Only set USERPROFILE/HOME if it isn't already pointing at a SkillArk
    // sandbox (so the ensure_global_default() call across runtimes reaches
    // the same DB). Set unconditionally for the first time only.
    unsafe { std::env::set_var("USERPROFILE", &home); }
    unsafe { std::env::set_var("HOME", &home); }
    home
}

async fn build_state(home: &Path) -> AppState {
    std::fs::create_dir_all(home.join(".skillark")).unwrap();
    std::fs::create_dir_all(home.join(".skillark/vault")).unwrap();
    let pool = skillark_lib::adapters::sqlite::connect(&format!(
        "sqlite:{}",
        home.join(".skillark/skillark.db").to_string_lossy()
    ))
    .await
    .unwrap();
    skillark_lib::repositories::WorkspaceRepository::new(pool.clone())
        .ensure_global_default()
        .await
        .unwrap();
    AppState::new(pool, home.join(".skillark/vault"))
}

fn build_fixture(home: &Path) -> Fixture {
    let source_dir = home.join("source/my-skill");
    std::fs::create_dir_all(source_dir.join("scripts")).unwrap();
    std::fs::write(
        source_dir.join("SKILL.md"),
        "---\nname: my-skill\nversion: 1.0.0\ndescription: feature graph\n---\nbody",
    )
    .unwrap();
    std::fs::write(source_dir.join("scripts/run.sh"), "echo").unwrap();

    let zip_path = home.join("skill.zip");
    write_test_zip(&zip_path);

    let target_global = home.join("agent/codex/skills/my-skill");
    let backups_dir = home.join(".skillark/backups");
    std::fs::create_dir_all(&backups_dir).unwrap();

    Fixture {
        source_dir,
        zip_path,
        target_global,
        backups_dir,
        db_url: format!("sqlite:{}", home.join(".skillark/skillark.db").to_string_lossy()),
    }
}

fn write_test_zip(path: &std::path::Path) {
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};
    let f = std::fs::File::create(path).unwrap();
    let mut zw = ZipWriter::new(f);
    let opts = SimpleFileOptions::default();
    zw.start_file("pkg/SKILL.md", opts).unwrap();
    zw.write_all(b"---\nname: pkg\nversion: 1.0.0\n---\nbody").unwrap();
    zw.finish().unwrap();
}

// ───── node steps ─────────────────────────────────────────────────────────

fn step_agent_repo_smoke(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(
        skillark_lib::repositories::AgentRepository::new(state.pool.clone())
            .upsert_agent("codex", "Codex", None, None, 0, false),
    );
    match r {
        Ok(id) => ok("agent_repo_smoke", format!("agent id={id}")),
        Err(e) => bad("agent_repo_smoke", e),
    }
}

fn step_import_directory(rt: Rt, state: AppState, fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Report {
    let imp = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let r = rt.block(imp.import_directory(fx.source_dir.clone()));
    match r {
        Ok(o) => {
            ctx.lock().unwrap().import_dir = Some(o.clone());
            ok("import_directory", format!("skill={} version={}", o.skill_id, o.version_id))
        }
        Err(e) => bad("import_directory", e),
    }
}

fn step_import_zip(rt: Rt, state: AppState, fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let imp = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let r = rt.block(imp.import_zip(fx.zip_path.clone()));
    match r {
        Ok(o) => ok("import_zip", format!("skill={} version={}", o.skill_id, o.version_id)),
        Err(e) => bad("import_zip", e),
    }
}

fn step_reimport_is_dedup(rt: Rt, state: AppState, fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let imp = ImportSkillService::new(state.vault_path.clone(), state.pool.clone());
    let r = rt.block(imp.import_directory(fx.source_dir.clone()));
    match r {
        Ok(o) if o.reused_version => ok("reimport_is_dedup", "dedup ok"),
        Ok(_) => bad("reimport_is_dedup", "expected reused_version=true"),
        Err(e) => bad("reimport_is_dedup", e),
    }
}

fn step_list_skills(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(state.skills().list_skills(None));
    match r {
        Ok(skills) if skills.is_empty() => bad("list_skills_after_import", "no skills after import"),
        Ok(skills) => ok("list_skills_after_import", format!("skills={}", skills.len())),
        Err(e) => bad("list_skills_after_import", e),
    }
}

fn step_create_project_workspace(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let id = uuid::Uuid::new_v4().to_string();
    let r = rt.block(
        skillark_lib::repositories::WorkspaceRepository::new(state.pool.clone())
            .create_project(&id, "demo", Some(std::path::Path::new("D:/code/demo"))),
    );
    match r {
        Ok(()) => ok("create_project_workspace", id),
        Err(e) => bad("create_project_workspace", e),
    }
}

fn step_list_workspaces(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(state.workspaces().list());
    match r {
        Ok(wss) => {
            let has_global = wss.iter().any(|w| w.id.0 == "global-default");
            let has_project = wss.iter().any(|w| matches!(w.kind, WorkspaceKind::Project));
            if has_global && has_project {
                ok("list_workspaces_has_global_and_project", format!("workspaces={}", wss.len()))
            } else {
                bad("list_workspaces_has_global_and_project",
                    format!("global={has_global} project={has_project}"))
            }
        }
        Err(e) => bad("list_workspaces_has_global_and_project", e),
    }
}

fn step_plan_global(rt: Rt, state: AppState, fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Report {
    let import_outcome = match ctx.lock().unwrap().import_dir.clone() {
        Some(o) => o,
        None => return bad("plan_global", "ctx missing import_dir outcome"),
    };
    let plan = rt.block(
        PlanDeploymentService::new(&state).build_plan(PlanRequest {
            skill_version_id: import_outcome.version_id.to_string(),
            targets: vec![PlanTargetSpec {
                agent_id: "codex".to_owned(),
                workspace_id: WorkspaceId("global-default".to_owned()),
                target_path: fx.target_global.clone(),
                mode: InstallMode::Copy,
            }],
        }),
    );
    match plan {
        Ok(p) => {
            ctx.lock().unwrap().plan = Some(p.clone());
            ok("plan_global", format!("targets={} requires_confirmation={}", p.targets.len(), p.requires_confirmation))
        }
        Err(e) => bad("plan_global", e),
    }
}

fn step_seed_codex_agent_for_deploy(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(
        skillark_lib::repositories::AgentRepository::new(state.pool.clone())
            .upsert_agent("codex", "Codex", None, None, 0, false),
    );
    match r {
        Ok(id) => ok("seed_codex_agent_for_deploy", format!("codex id={id}")),
        Err(e) => bad("seed_codex_agent_for_deploy", e),
    }
}

fn step_execute_global_copy(rt: Rt, state: AppState, fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Report {
    let plan = match ctx.lock().unwrap().plan.clone() {
        Some(p) => p,
        None => return bad("execute_global_copy", "ctx missing plan"),
    };
    let r = rt.block(ExecuteDeploymentService::new(&state).execute(plan));
    let report = match r {
        Ok(r) => r,
        Err(e) => return bad("execute_global_copy", e),
    };
    let target = fx.target_global.clone();
    // Debug: dump agents table + active count.
    let active = rt.block(state.deployments().list_active());
    let all = active.as_ref().map(|v| v.len()).unwrap_or(0);
    let agents = rt.block(
        sqlx::query_as::<_, (String,)>("SELECT id FROM agents WHERE agent_type = ?")
            .bind("codex")
            .fetch_all(&state.pool),
    );
    let agent_id_dbg = match agents {
        Ok(rows) => rows.iter().map(|(id,)| id.clone()).collect::<Vec<_>>().join(","),
        Err(e) => format!("err={e}"),
    };
    let rec = match rt.block(state.deployments().find_active_by_target(&target)) {
        Ok(Some(r)) => r,
        Ok(None) => {
            let detail = format!(
                "deployment not persisted; target={} active_count={} agents_in_table=[{}] report={:?}",
                target.display(), all, agent_id_dbg, report
            );
            return bad("execute_global_copy", detail);
        }
        Err(e) => return bad("execute_global_copy", e),
    };
    ctx.lock().unwrap().deployment_id = Some(rec.id);
    ok("execute_global_copy", format!("succeeded={} failed={}", report.succeeded, report.failed))
}

fn step_verify_synced(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(VerifyDeploymentService::new(&state).verify_all());
    match r {
        Ok(items) if items.is_empty() => bad("verify_synced", "no deployments verified"),
        Ok(items) if items.iter().all(|i| i.status == "synced") =>
            ok("verify_synced", format!("items={}", items.len())),
        Ok(items) => bad("verify_synced", format!("not synced: {:?}", items)),
        Err(e) => bad("verify_synced", e),
    }
}

fn step_modify_and_reverify(rt: Rt, state: AppState, fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Report {
    let _ = ctx;
    let target = fx.target_global.clone();
    std::fs::write(
        target.join("SKILL.md"),
        "---\nname: my-skill\nversion: 1.0.0\n---\nEDITED",
    )
    .unwrap();
    let r = rt.block(VerifyDeploymentService::new(&state).verify_all());
    match r {
        Ok(items) if items.iter().any(|i| i.status == "modified") =>
            ok("modify_then_reverify_modified", format!("items={}", items.len())),
        Ok(items) => bad("modify_then_reverify_modified", format!("no modified: {:?}", items)),
        Err(e) => bad("modify_then_reverify_modified", e),
    }
}

fn step_uninstall_modified(rt: Rt, state: AppState, _fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Report {
    let id = match ctx.lock().unwrap().deployment_id.clone() {
        Some(s) => s,
        None => return bad("uninstall_modified_requires_force", "ctx missing deployment_id"),
    };
    let r = rt.block(
        UninstallDeploymentService::new(&state).uninstall(&id, false),
    );
    match r {
        Ok(o) if !o.removed_target =>
            ok("uninstall_modified_requires_force", "refused to delete modified target"),
        Ok(_) => bad("uninstall_modified_requires_force", "removed modified without force"),
        Err(e) => bad("uninstall_modified_requires_force", e),
    }
}

fn step_uninstall_force(rt: Rt, state: AppState, fx: Fixture, ctx: Arc<Mutex<Ctx>>) -> Report {
    let id = match ctx.lock().unwrap().deployment_id.clone() {
        Some(s) => s,
        None => return bad("uninstall_with_force_removes", "ctx missing deployment_id"),
    };
    let r = rt.block(
        UninstallDeploymentService::new(&state).uninstall(&id, true),
    );
    match r {
        Ok(o) if o.removed_target && !fx.target_global.exists() =>
            ok("uninstall_with_force_removes", "force-removed; target gone"),
        Ok(_) => bad("uninstall_with_force_removes", "force-remove did not remove target"),
        Err(e) => bad("uninstall_with_force_removes", e),
    }
}

fn step_list_operations(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(state.operations().list_recent(50));
    match r {
        Ok(ops) => {
            let install = ops
                .iter()
                .any(|o| o.operation_type == OperationType::Install && o.status == OperationStatus::Succeeded);
            let uninstall = ops
                .iter()
                .any(|o| o.operation_type == OperationType::Uninstall && o.status == OperationStatus::Succeeded);
            if install && uninstall {
                ok("list_operations_records_every_action", format!("ops={}", ops.len()))
            } else {
                bad("list_operations_records_every_action",
                    format!("install={install} uninstall={uninstall}"))
            }
        }
        Err(e) => bad("list_operations_records_every_action", e),
    }
}

fn step_disabled_excluded(rt: Rt, state: AppState, _fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let r = rt.block(
        skillark_lib::application::disabled_agents::set_disabled(&state.pool, "codex", true),
    );
    if let Err(e) = r { return bad("disabled_agent_excluded_from_resolve", e); }
    let disabled = rt.block(
        skillark_lib::application::disabled_agents::load_disabled(&state.pool),
    ).unwrap_or_default();
    let resolved = skillark_lib::application::resolve_targets::resolve_targets(
        &disabled, "my-skill", None, &["codex".to_owned()],
    )
    .unwrap_or_default();
    if resolved.iter().all(|x| x.agent_type != "codex") {
        ok("disabled_agent_excluded_from_resolve", "codex filtered out")
    } else {
        bad("disabled_agent_excluded_from_resolve", format!("still resolved: {:?}", resolved))
    }
}

fn step_backup(_rt: Rt, _state: AppState, fx: Fixture, _ctx: Arc<Mutex<Ctx>>) -> Report {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
    let r = runtime.block_on(
        skillark_lib::application::backup::backup_database_file(&fx.db_url, &fx.backups_dir),
    );
    match r {
        Ok(p) if p.is_file() => ok("backup_database_writes_snapshot", p.to_string_lossy().into_owned()),
        Ok(p) => bad("backup_database_writes_snapshot", format!("missing file: {}", p.display())),
        Err(e) => bad("backup_database_writes_snapshot", e.to_string()),
    }
}
