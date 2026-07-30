pub mod contracts;

use std::{collections::HashMap, path::PathBuf};

use contracts::AgentCandidateDto;
use serde::{Deserialize, Serialize};

use crate::application::agent_overrides::{self, AgentOverride};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatusDto {
    project: &'static str,
    version: &'static str,
    phase: &'static str,
    next_milestone: &'static str,
    foundations: [&'static str; 4],
}

#[tauri::command]
pub fn get_bootstrap_status() -> BootstrapStatusDto {
    let status = crate::application::bootstrap::current_status();

    BootstrapStatusDto {
        project: status.project,
        version: status.version,
        phase: status.phase,
        next_milestone: status.next_milestone,
        foundations: status.foundations,
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverAgentsRequest {
    #[serde(default)]
    manual_skill_paths: HashMap<String, String>,
}

#[tauri::command]
pub async fn discover_agents(
    request: Option<DiscoverAgentsRequest>,
) -> Result<Vec<AgentCandidateDto>, String> {
    let manual_skill_paths = request
        .unwrap_or_default()
        .manual_skill_paths
        .into_iter()
        .map(|(agent_type, path)| (agent_type, PathBuf::from(path)))
        .collect();

    tauri::async_runtime::spawn_blocking(move || {
        crate::application::agent_discovery::discover_agents(manual_skill_paths)
            .map(|candidates| candidates.iter().map(AgentCandidateDto::from).collect())
    })
    .await
    .map_err(|error| format!("Agent discovery task failed: {error}"))?
}

#[tauri::command]
pub fn cancel_agent_discovery() {
    crate::application::agent_discovery::cancel_discovery();
}

// ===== Agent Overrides =====

#[tauri::command]
pub fn get_agent_overrides() -> Result<Vec<AgentOverride>, String> {
    Ok(agent_overrides::load_overrides())
}

#[tauri::command]
pub fn save_agent_override(request: AgentOverride) -> Result<(), String> {
    agent_overrides::save_override(request)
}

#[tauri::command]
pub fn delete_agent_override(agent_type: String) -> Result<(), String> {
    agent_overrides::delete_override(&agent_type)
}

// ===== v0.1 Library / Deploy =====

use crate::application::state::AppState;
use crate::domain::workspace::WorkspaceId;
use contracts::{
    DeploymentDto, OperationDto, PlanDto, PlanRequestDto, PlanTargetDto, SkillDto, SkillVersionDto,
    WorkspaceDto,
};

#[tauri::command]
pub async fn import_skill_from_directory(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<crate::application::import_skill::ImportOutcome, String> {
    let importer = crate::application::import_skill::ImportSkillService::new(
        state.vault_path.clone(),
        state.pool.clone(),
    );
    importer.import_directory(std::path::PathBuf::from(path)).await
}

#[tauri::command]
pub async fn import_skill_from_zip(
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<crate::application::import_skill::ImportOutcome, String> {
    let importer = crate::application::import_skill::ImportSkillService::new(
        state.vault_path.clone(),
        state.pool.clone(),
    );
    importer.import_zip(std::path::PathBuf::from(path)).await
}

#[tauri::command]
pub async fn list_skills(state: tauri::State<'_, AppState>) -> Result<Vec<SkillDto>, String> {
    let repo = state.skills();
    let skills = repo.list_skills(None).await?;
    let mut out = Vec::with_capacity(skills.len());
    for skill in skills {
        let (hash, label) = match repo.get_latest_version(skill.id).await? {
            Some(v) => (Some(v.content_hash), v.version_label),
            None => (None, None),
        };
        out.push(SkillDto::from_skill(&skill, hash, label));
    }
    Ok(out)
}

#[tauri::command]
pub async fn delete_skill(
    state: tauri::State<'_, AppState>,
    skill_id: String,
) -> Result<(), String> {
    let id = uuid::Uuid::parse_str(&skill_id).map_err(|e| e.to_string())?;
    state.skills().delete_skill(id).await
}

#[tauri::command]
pub async fn list_deployments(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<DeploymentDto>, String> {
    let deps = state.deployments().list_active().await?;
    Ok(deps.iter().map(DeploymentDto::from).collect())
}

#[tauri::command]
pub async fn plan_deployment(
    state: tauri::State<'_, AppState>,
    request: PlanRequestDto,
) -> Result<PlanDto, String> {
    let targets = request
        .targets
        .into_iter()
        .map(|t| crate::application::plan_deployment::PlanTargetSpec {
            agent_id: t.agent_id,
            workspace_id: WorkspaceId(t.workspace_id),
            target_path: std::path::PathBuf::from(t.target_path),
            mode: std::str::FromStr::from_str(&t.mode).unwrap_or(crate::domain::deployment::InstallMode::Copy),
        })
        .collect();

    let plan = crate::application::plan_deployment::PlanDeploymentService::new(&state)
        .build_plan(crate::application::plan_deployment::PlanRequest {
            skill_version_id: request.skill_version_id,
            targets,
        })
        .await?;

    Ok(PlanDto {
        operation_id: plan.operation_id.0.clone(),
        skill_version_id: plan.skill_version_id,
        requires_confirmation: plan.requires_confirmation,
        warnings: plan.warnings,
        targets: plan
            .targets
            .into_iter()
            .map(|t| PlanTargetDto {
                agent_id: t.agent_id,
                workspace_id: t.workspace_id.0,
                target_path: t.target_path.to_string_lossy().into_owned(),
                mode: t.mode.as_str().to_owned(),
                conflict: t.conflict.as_str().to_owned(),
                warnings: t.warnings,
            })
            .collect(),
    })
}

#[tauri::command]
pub async fn execute_deployment(
    state: tauri::State<'_, AppState>,
    plan: PlanDto,
) -> Result<crate::application::execute_deployment::ExecutionReport, String> {
    let report = crate::application::execute_deployment::ExecuteDeploymentService::new(&state)
        .execute(plan.to_plan())
        .await?;
    Ok(report)
}

#[tauri::command]
pub async fn verify_deployments(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<crate::application::verify_deployment::VerifyReportItem>, String> {
    crate::application::verify_deployment::VerifyDeploymentService::new(&state)
        .verify_all()
        .await
}

#[tauri::command]
pub async fn uninstall_deployment(
    state: tauri::State<'_, AppState>,
    deployment_id: String,
    force: bool,
) -> Result<crate::application::uninstall_deployment::UninstallOutcome, String> {
    crate::application::uninstall_deployment::UninstallDeploymentService::new(&state)
        .uninstall(&deployment_id, force)
        .await
}

#[tauri::command]
pub async fn list_operations(
    state: tauri::State<'_, AppState>,
    limit: Option<i64>,
) -> Result<Vec<OperationDto>, String> {
    let ops = state.operations().list_recent(limit.unwrap_or(50)).await?;
    Ok(ops.iter().map(OperationDto::from).collect())
}

#[tauri::command]
pub async fn backup_database() -> Result<String, String> {
    let dir = crate::application::state::skillark_dir()?;
    let src_url = format!("sqlite:{}", dir.join("skillark.db").display());
    let dest_dir = dir.join("backups");
    let dest = crate::application::backup::backup_database_file(&src_url, &dest_dir)
        .await
        .map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

// ===== Workspaces =====

#[tauri::command]
pub async fn list_workspaces(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<WorkspaceDto>, String> {
    let workspaces = state.workspaces().list().await?;
    Ok(workspaces
        .iter()
        .map(|w| {
            let kind = match w.kind {
                crate::domain::workspace::WorkspaceKind::Global => "global",
                crate::domain::workspace::WorkspaceKind::Project => "project",
            };
            // A project workspace is "missing" when its declared root no longer
            // exists on disk; global workspaces are always available.
            let status = match (&w.kind, &w.root_path) {
                (crate::domain::workspace::WorkspaceKind::Project, Some(root))
                    if !root.exists() =>
                {
                    "missing"
                }
                _ => "available",
            };
            WorkspaceDto {
                id: w.id.0.clone(),
                kind: kind.to_owned(),
                name: w.name.clone(),
                root_path: w.root_path.as_ref().map(|p| p.to_string_lossy().into_owned()),
                status: status.to_owned(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn create_project_workspace(
    state: tauri::State<'_, AppState>,
    name: String,
    root_path: String,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let root = if root_path.trim().is_empty() {
        None
    } else {
        Some(std::path::PathBuf::from(root_path))
    };
    state
        .workspaces()
        .create_project(&id, &name, root.as_deref())
        .await?;
    Ok(id)
}

#[tauri::command]
pub async fn delete_project_workspace(
    state: tauri::State<'_, AppState>,
    workspace_id: String,
) -> Result<(), String> {
    state.workspaces().delete(&workspace_id).await
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTargetDto {
    pub agent_type: String,
    pub target_path: String,
}

#[tauri::command]
pub async fn resolve_project_target_paths(
    state: tauri::State<'_, AppState>,
    canonical_name: String,
    project_root: String,
    agent_types: Vec<String>,
) -> Result<Vec<ResolvedTargetDto>, String> {
    let disabled = crate::application::disabled_agents::load_disabled(&state.pool).await?;
    let resolved = crate::application::resolve_targets::resolve_targets(
        &disabled,
        &canonical_name,
        Some(std::path::Path::new(&project_root)),
        &agent_types,
    )?;
    Ok(resolved
        .iter()
        .map(|r| ResolvedTargetDto {
            agent_type: r.agent_type.clone(),
            target_path: r.target_path.to_string_lossy().into_owned(),
        })
        .collect())
}

#[tauri::command]
pub async fn resolve_global_target_paths(
    state: tauri::State<'_, AppState>,
    canonical_name: String,
    agent_types: Vec<String>,
) -> Result<Vec<ResolvedTargetDto>, String> {
    let disabled = crate::application::disabled_agents::load_disabled(&state.pool).await?;
    let resolved = crate::application::resolve_targets::resolve_targets(
        &disabled,
        &canonical_name,
        None,
        &agent_types,
    )?;
    Ok(resolved
        .iter()
        .map(|r| ResolvedTargetDto {
            agent_type: r.agent_type.clone(),
            target_path: r.target_path.to_string_lossy().into_owned(),
        })
        .collect())
}

// ===== Skill detail (file tree + SKILL.md) =====

#[tauri::command]
pub async fn get_skill_detail(
    state: tauri::State<'_, AppState>,
    skill_id: String,
) -> Result<crate::application::skill_detail::SkillDetailDto, String> {
    use crate::application::skill_detail::{list_files, read_text};

    let id = uuid::Uuid::parse_str(&skill_id).map_err(|e| e.to_string())?;
    let repo = state.skills();
    let skill = repo
        .get_skill_by_id(id)
        .await?
        .ok_or_else(|| format!("skill {skill_id} not found"))?;
    let version = repo
        .get_latest_version(id)
        .await?
        .ok_or_else(|| "skill has no version".to_owned())?;

    let snapshot = std::path::PathBuf::from(&version.library_snapshot_path);
    let snap_for_task = snapshot.clone();
    let (files, skill_md) =
        tauri::async_runtime::spawn_blocking(move || -> (Vec<_>, Option<String>) {
            let files = list_files(&snap_for_task).unwrap_or_default();
            let skill_md = read_text(&snap_for_task.join("SKILL.md"));
            (files, skill_md)
        })
        .await
        .map_err(|e| format!("detail task: {e}"))?;

    Ok(crate::application::skill_detail::SkillDetailDto {
        id: skill.id.to_string(),
        canonical_name: skill.canonical_name,
        display_name: skill.display_name,
        description: skill.description,
        content_hash: Some(version.content_hash),
        version_label: version.version_label,
        snapshot_path: version.library_snapshot_path,
        skill_md,
        files,
    })
}

// ===== Agent enable/disable (PRD §5.2) =====

#[tauri::command]
pub async fn get_disabled_agents(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<String>, String> {
    crate::application::disabled_agents::load_disabled(&state.pool).await
}

#[tauri::command]
pub async fn set_agent_disabled(
    state: tauri::State<'_, AppState>,
    agent_type: String,
    disabled: bool,
) -> Result<Vec<String>, String> {
    crate::application::disabled_agents::set_disabled(&state.pool, &agent_type, disabled).await
}

#[tauri::command]
pub async fn list_skill_versions(
    state: tauri::State<'_, AppState>,
    skill_id: String,
) -> Result<Vec<SkillVersionDto>, String> {
    let id = uuid::Uuid::parse_str(&skill_id).map_err(|e| e.to_string())?;
    let versions = state.skills().list_versions(id).await?;
    Ok(versions
        .iter()
        .map(|v| SkillVersionDto {
            id: v.id.to_string(),
            content_hash: v.content_hash.clone(),
            version_label: v.version_label.clone(),
            created_at: v.created_at.clone(),
        })
        .collect())
}

// ===== v0.2 Link Bridge =====

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkCandidateDto {
    pub name: String,
    pub version: String,
    pub description: String,
    pub relative_path: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkPreviewDto {
    /// One-time token; the UI sends it back with the chosen candidate index.
    pub token: String,
    pub remote: String,
    pub resolved_revision: String,
    pub candidates: Vec<LinkCandidateDto>,
}

#[tauri::command]
pub async fn preview_link(
    state: tauri::State<'_, AppState>,
    link: String,
) -> Result<LinkPreviewDto, String> {
    let service = crate::application::link_bridge::LinkImportService::new(
        state.pool.clone(),
        state.vault_path.clone(),
        state.fetch_cache_root.clone(),
    );
    let preview = service.preview(&link).await?;
    let remote = preview.locator.display_label();
    let resolved_revision = preview.resolved_revision.clone();
    let candidates = preview
        .candidates
        .iter()
        .map(|c| LinkCandidateDto {
            name: c.manifest.name.clone(),
            version: c.manifest.version.clone(),
            description: c.manifest.description.clone(),
            relative_path: c.relative_path.clone(),
        })
        .collect();
    let token = state.store_link_preview(preview);
    Ok(LinkPreviewDto {
        token,
        remote,
        resolved_revision,
        candidates,
    })
}

#[tauri::command]
pub async fn import_link_candidate(
    state: tauri::State<'_, AppState>,
    token: String,
    candidate_index: usize,
) -> Result<crate::application::import_skill::ImportOutcome, String> {
    let preview = state
        .take_link_preview(&token)
        .ok_or_else(|| "preview expired or already imported".to_string())?;
    let service = crate::application::link_bridge::LinkImportService::new(
        state.pool.clone(),
        state.vault_path.clone(),
        state.fetch_cache_root.clone(),
    );
    service.import_candidate(&preview, candidate_index).await
}
