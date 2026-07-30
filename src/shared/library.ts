// v0.1 Library / Deploy / Operations wire types. Mirror the camelCase DTOs in
// src-tauri/src/commands/contracts.rs and the Serialize structs returned by the
// execute / verify commands.

export type SkillStatus = "ready" | "corrupted" | "missing";

export type Skill = {
  id: string;
  canonicalName: string;
  displayName: string;
  description: string;
  format: string;
  libraryPath: string;
  status: string;
  currentVersionId: string | null;
  contentHash: string | null;
  versionLabel: string | null;
  createdAt: string;
  updatedAt: string;
};

export type SkillVersion = {
  id: string;
  contentHash: string;
  versionLabel: string | null;
  createdAt: string;
};

export type InstallMode = "copy" | "junction";

export type ConflictKind =
  | "none"
  | "managed_same"
  | "managed_outdated"
  | "managed_modified"
  | "unmanaged_skill"
  | "unmanaged_directory"
  | "file_conflict"
  | "permission_denied";

export type DeploymentStatus =
  | "planned"
  | "installing"
  | "synced"
  | "outdated"
  | "modified"
  | "missing"
  | "failed"
  | "uninstalled";

export type PlanTargetSpec = {
  agentId: string;
  workspaceId: string;
  targetPath: string;
  mode: InstallMode;
};

export type PlanRequest = {
  skillVersionId: string;
  targets: PlanTargetSpec[];
};

export type PlanTarget = {
  agentId: string;
  workspaceId: string;
  targetPath: string;
  mode: InstallMode;
  conflict: ConflictKind;
  warnings: string[];
};

export type Plan = {
  operationId: string;
  skillVersionId: string;
  requiresConfirmation: boolean;
  warnings: string[];
  targets: PlanTarget[];
};

export type TargetOutcome = {
  agentId: string;
  workspaceId: string;
  targetPath: string;
  mode: string;
  conflict: string;
  ok: boolean;
  deployedHash: string | null;
  error: string | null;
};

export type ExecutionReport = {
  operationId: string;
  skillVersionId: string;
  succeeded: number;
  failed: number;
  outcomes: TargetOutcome[];
};

export type Deployment = {
  id: string;
  skillVersionId: string;
  agentId: string;
  workspaceId: string;
  targetPath: string;
  mode: InstallMode;
  status: DeploymentStatus;
  deployedHash: string | null;
  installedAt: string | null;
  lastVerifiedAt: string | null;
  errorMessage: string | null;
};

export type VerifyReportItem = {
  deploymentId: string;
  agentId: string;
  targetPath: string;
  mode: string;
  status: string;
  reason: string;
  observedHash: string | null;
};

export type Operation = {
  id: string;
  operationType: string;
  status: string;
  startedAt: string;
  completedAt: string | null;
  errorMessage: string | null;
  resultJson: string | null;
};

export type Workspace = {
  id: string;
  kind: "global" | "project";
  name: string;
  rootPath: string | null;
  status: "available" | "missing";
};

export type FileEntry = { path: string; size: number };

export type SkillDetail = {
  id: string;
  canonicalName: string;
  displayName: string;
  description: string;
  contentHash: string | null;
  versionLabel: string | null;
  snapshotPath: string;
  skillMd: string | null;
  files: FileEntry[];
};

// v0.2 Link Bridge — paste a Git link → preview candidates → import.
export type LinkCandidate = {
  name: string;
  version: string;
  description: string;
  relativePath: string;
};

export type LinkPreview = {
  token: string;
  remote: string;
  resolvedRevision: string;
  candidates: LinkCandidate[];
};
