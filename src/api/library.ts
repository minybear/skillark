import { invoke } from "@tauri-apps/api/core";
import type {
  Deployment,
  ExecutionReport,
  LinkPreview,
  Operation,
  Plan,
  PlanRequest,
  Skill,
  SkillDetail,
  SkillVersion,
  VerifyReportItem,
  Workspace,
} from "../shared/library";

export async function listSkills(): Promise<Skill[]> {
  return invoke<Skill[]>("list_skills");
}

export async function listSkillVersions(skillId: string): Promise<SkillVersion[]> {
  return invoke<SkillVersion[]>("list_skill_versions", { skillId });
}

export async function deleteSkill(skillId: string): Promise<void> {
  await invoke("delete_skill", { skillId });
}

export async function importSkillFromDirectory(path: string) {
  return invoke("import_skill_from_directory", { path });
}

export async function importSkillFromZip(path: string) {
  return invoke("import_skill_from_zip", { path });
}

// v0.2 Link Bridge
export async function previewLink(link: string): Promise<LinkPreview> {
  return invoke<LinkPreview>("preview_link", { link });
}

export async function importLinkCandidate(token: string, candidateIndex: number) {
  return invoke("import_link_candidate", { token, candidateIndex });
}

export async function listDeployments(): Promise<Deployment[]> {
  return invoke<Deployment[]>("list_deployments");
}

export async function planDeployment(request: PlanRequest): Promise<Plan> {
  return invoke<Plan>("plan_deployment", { request });
}

export async function executeDeployment(plan: Plan): Promise<ExecutionReport> {
  return invoke<ExecutionReport>("execute_deployment", { plan });
}

export async function verifyDeployments(): Promise<VerifyReportItem[]> {
  return invoke<VerifyReportItem[]>("verify_deployments");
}

export async function uninstallDeployment(
  deploymentId: string,
  force: boolean,
) {
  return invoke("uninstall_deployment", { deploymentId, force });
}

export async function listOperations(limit = 50): Promise<Operation[]> {
  return invoke<Operation[]>("list_operations", { limit });
}

export async function listWorkspaces(): Promise<Workspace[]> {
  return invoke<Workspace[]>("list_workspaces");
}

export async function createProjectWorkspace(name: string, rootPath: string): Promise<string> {
  return invoke<string>("create_project_workspace", { name, rootPath });
}

export async function deleteProjectWorkspace(workspaceId: string): Promise<void> {
  await invoke("delete_project_workspace", { workspaceId });
}

export type ResolvedTarget = { agentType: string; targetPath: string };

export async function resolveGlobalTargetPaths(
  canonicalName: string,
  agentTypes: string[],
): Promise<ResolvedTarget[]> {
  return invoke<ResolvedTarget[]>("resolve_global_target_paths", { canonicalName, agentTypes });
}

export async function resolveProjectTargetPaths(
  canonicalName: string,
  projectRoot: string,
  agentTypes: string[],
): Promise<ResolvedTarget[]> {
  return invoke<ResolvedTarget[]>("resolve_project_target_paths", {
    canonicalName,
    projectRoot,
    agentTypes,
  });
}

export async function getSkillDetail(skillId: string): Promise<SkillDetail> {
  return invoke<SkillDetail>("get_skill_detail", { skillId });
}

export async function getDisabledAgents(): Promise<string[]> {
  return invoke<string[]>("get_disabled_agents");
}

export async function setAgentDisabled(
  agentType: string,
  disabled: boolean,
): Promise<string[]> {
  return invoke<string[]>("set_agent_disabled", { agentType, disabled });
}
