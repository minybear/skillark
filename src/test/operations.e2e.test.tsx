import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import OperationsPage from "../pages/OperationsPage";
import type { Deployment, Operation } from "../shared/library";

afterEach(() => clearMocks());

const INSTALL_OP: Operation = {
  id: "op-1",
  operationType: "install",
  status: "succeeded",
  startedAt: "2026-07-28T10:00:00Z",
  completedAt: "2026-07-28T10:00:01Z",
  errorMessage: null,
  resultJson: null,
};

const UNINSTALL_OP: Operation = {
  id: "op-2",
  operationType: "uninstall",
  status: "succeeded",
  startedAt: "2026-07-28T11:00:00Z",
  completedAt: "2026-07-28T11:00:01Z",
  errorMessage: null,
  resultJson: null,
};

function deployment(over: Partial<Deployment> = {}): Deployment {
  return {
    id: "dep-1",
    skillVersionId: "ver-1",
    agentId: "codex",
    workspaceId: "global-default",
    targetPath: "C:/agents/codex/skills/my-skill",
    mode: "copy",
    status: "synced",
    deployedHash: "abc123",
    installedAt: "2026-07-28T10:00:00Z",
    lastVerifiedAt: null,
    errorMessage: null,
    ...over,
  };
}

describe("OperationsPage interaction E2E", () => {
  it("lists operation audit records (install + uninstall traceable)", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_operations") return [INSTALL_OP, UNINSTALL_OP];
      if (cmd === "list_deployments") return [];
      return undefined;
    });
    render(<OperationsPage lang="en" />);
    expect(await screen.findByText("install")).toBeInTheDocument();
    expect(screen.getByText("uninstall")).toBeInTheDocument();
    expect(screen.getAllByText("succeeded").length).toBeGreaterThanOrEqual(2);
  });

  it("verify all surfaces a modified (drifted) target", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_operations") return [INSTALL_OP];
      if (cmd === "list_deployments") return [deployment()];
      if (cmd === "verify_deployments")
        return [
          {
            deploymentId: "dep-1",
            agentId: "codex",
            targetPath: "C:/agents/codex/skills/my-skill",
            mode: "copy",
            status: "modified",
            reason: "hash_mismatch",
            observedHash: "zzz999",
          },
        ];
      return undefined;
    });

    render(<OperationsPage lang="en" />);
    await screen.findByText("C:/agents/codex/skills/my-skill");
    await userEvent.click(screen.getByRole("button", { name: /verify/i }));
    expect(await screen.findByText("modified")).toBeInTheDocument();
  });

  it("uninstall calls the backend and refreshes the deployment list", async () => {
    let deployments = [deployment()];
    const uninstalled: { id: string; force: boolean }[] = [];
    mockIPC((cmd, payload) => {
      if (cmd === "list_operations") return [INSTALL_OP];
      if (cmd === "list_deployments") return deployments;
      if (cmd === "uninstall_deployment") {
        const p = payload as { deploymentId: string; force: boolean };
        uninstalled.push({ id: p.deploymentId, force: p.force });
        deployments = [];
        return { removedTarget: true };
      }
      return undefined;
    });

    render(<OperationsPage lang="en" />);
    await screen.findByText("C:/agents/codex/skills/my-skill");
    await userEvent.click(screen.getByRole("button", { name: /delete/i }));

    await waitFor(() =>
      expect(uninstalled).toEqual([{ id: "dep-1", force: false }]),
    );
    // deployment removed from the list after refresh
    await waitFor(() =>
      expect(screen.queryByText("C:/agents/codex/skills/my-skill")).not.toBeInTheDocument(),
    );
  });
});
