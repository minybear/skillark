import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import DeployPage from "../pages/DeployPage";
import type { Skill, Workspace } from "../shared/library";
import type { AgentCandidate } from "../shared/agents";

afterEach(() => clearMocks());

const SKILL: Skill = {
  id: "skill-1",
  canonicalName: "my-skill",
  displayName: "My Skill",
  description: "demo",
  format: "agent-skills",
  libraryPath: "/vault/my-skill",
  status: "ready",
  currentVersionId: "ver-1",
  contentHash: "abc123",
  versionLabel: "1.0.0",
  createdAt: "2026-07-28T00:00:00Z",
  updatedAt: "2026-07-28T00:00:00Z",
};

const GLOBAL_WS: Workspace = {
  id: "global-default",
  name: "Global",
  kind: "global",
  rootPath: null,
  status: "available",
} as Workspace;

function agent(agentType: string, displayName: string): AgentCandidate {
  return {
    agentType,
    displayName,
    confidence: 1,
    executablePath: `C:/agents/${agentType}/bin`,
    globalSkillPath: `C:/agents/${agentType}/skills`,
    writable: true,
    signals: [],
  };
}

describe("DeployPage interaction E2E", () => {
  it("skill → scan → pick 2 agents → plan → execute shows per-target results", async () => {
    const planCalls: unknown[] = [];
    mockIPC((cmd, payload) => {
      switch (cmd) {
        case "list_skills":
          return [SKILL];
        case "list_workspaces":
          return [GLOBAL_WS];
        case "get_disabled_agents":
          return [];
        case "discover_agents":
          return [agent("codex", "Codex"), agent("cursor", "Cursor")];
        case "resolve_global_target_paths":
          return (payload as { agentTypes: string[] }).agentTypes.map((t) => ({
            agentType: t,
            targetPath: `C:/agents/${t}/skills/my-skill`,
          }));
        case "plan_deployment": {
          planCalls.push(payload);
          const req = payload as { request: { targets: { agentId: string; targetPath: string }[] } };
          return {
            operationId: "op-1",
            skillVersionId: "skill-1",
            requiresConfirmation: false,
            warnings: [],
            targets: req.request.targets.map((t) => ({
              agentId: t.agentId,
              workspaceId: "global-default",
              targetPath: t.targetPath,
              mode: "copy",
              conflict: "none",
              warnings: [],
            })),
          };
        }
        case "execute_deployment":
          return {
            succeeded: 1,
            failed: 1,
            outcomes: [
              { agentId: "codex", targetPath: "C:/agents/codex/skills/my-skill", ok: true, error: null },
              { agentId: "cursor", targetPath: "C:/agents/cursor/skills/my-skill", ok: false, error: "permission denied" },
            ],
          };
        default:
          return undefined;
      }
    });

    render(<DeployPage lang="en" />);

    // pick the skill
    const skillSelect = (await screen.findAllByRole("combobox"))[0];
    await userEvent.selectOptions(skillSelect, "skill-1");

    // scan agents
    await userEvent.click(screen.getByRole("button", { name: /scan agents/i }));
    const codex = await screen.findByText("Codex");
    expect(codex).toBeInTheDocument();

    // pick both agents
    const boxes = screen.getAllByRole("checkbox");
    await userEvent.click(boxes[0]);
    await userEvent.click(boxes[1]);

    // build the plan
    await userEvent.click(screen.getByRole("button", { name: /build plan/i }));
    expect(await screen.findByText("C:/agents/codex/skills/my-skill")).toBeInTheDocument();
    expect(screen.getByText("C:/agents/cursor/skills/my-skill")).toBeInTheDocument();

    // execute → per-target results: one ok, one failed (partial success visible)
    await userEvent.click(screen.getByRole("button", { name: /execute plan/i }));
    expect(await screen.findByText(/permission denied/i)).toBeInTheDocument();
    expect(screen.getByText(/1 succeeded/i)).toBeInTheDocument();
    expect(planCalls).toHaveLength(1);
  });

  it("disabled agent is excluded from the pick list", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_skills":
          return [SKILL];
        case "list_workspaces":
          return [GLOBAL_WS];
        case "get_disabled_agents":
          return ["cursor"]; // cursor disabled
        case "discover_agents":
          return [agent("codex", "Codex"), agent("cursor", "Cursor")];
        default:
          return undefined;
      }
    });

    render(<DeployPage lang="en" />);
    const skillSelect = (await screen.findAllByRole("combobox"))[0];
    await userEvent.selectOptions(skillSelect, "skill-1");
    await userEvent.click(screen.getByRole("button", { name: /scan agents/i }));

    await screen.findByText("Codex");
    // Cursor is disabled → not offered as a deploy target
    expect(screen.queryByText("Cursor")).not.toBeInTheDocument();
  });

  it("requires-confirmation warning is shown for unsafe plans", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "list_skills":
          return [SKILL];
        case "list_workspaces":
          return [GLOBAL_WS];
        case "get_disabled_agents":
          return [];
        case "discover_agents":
          return [agent("codex", "Codex")];
        case "resolve_global_target_paths":
          return [{ agentType: "codex", targetPath: "C:/agents/codex/skills/my-skill" }];
        case "plan_deployment":
          return {
            operationId: "op-1",
            skillVersionId: "skill-1",
            requiresConfirmation: true,
            warnings: ["unmanaged directory"],
            targets: [
              {
                agentId: "codex",
                workspaceId: "global-default",
                targetPath: "C:/agents/codex/skills/my-skill",
                mode: "copy",
                conflict: "unmanaged_directory",
                warnings: ["unmanaged directory"],
              },
            ],
          };
        default:
          return undefined;
      }
    });

    render(<DeployPage lang="en" />);
    const skillSelect = (await screen.findAllByRole("combobox"))[0];
    await userEvent.selectOptions(skillSelect, "skill-1");
    await userEvent.click(screen.getByRole("button", { name: /scan agents/i }));
    await screen.findByText("Codex");
    await userEvent.click(screen.getAllByRole("checkbox")[0]);
    await userEvent.click(screen.getByRole("button", { name: /build plan/i }));

    // the conflict tag + confirmation warning must both surface
    expect(await screen.findByText("unmanaged_directory")).toBeInTheDocument();
    expect(screen.getByText(/overwrites existing targets/i)).toBeInTheDocument();
  });

  it("junction failure can build a Copy retry plan for failed targets only", async () => {
    const planCalls: Array<{
      request: {
        skillVersionId: string;
        targets: Array<{
          agentId: string;
          workspaceId: string;
          targetPath: string;
          mode: string;
        }>;
      };
    }> = [];
    mockIPC((cmd, payload) => {
      switch (cmd) {
        case "list_skills":
          return [SKILL];
        case "list_workspaces":
          return [GLOBAL_WS];
        case "get_disabled_agents":
          return [];
        case "discover_agents":
          return [agent("codex", "Codex"), agent("cursor", "Cursor")];
        case "resolve_global_target_paths":
          return (payload as { agentTypes: string[] }).agentTypes.map((agentType) => ({
            agentType,
            targetPath: `C:/agents/${agentType}/skills/my-skill`,
          }));
        case "plan_deployment": {
          const request = (payload as { request: (typeof planCalls)[number]["request"] }).request;
          planCalls.push({ request });
          return {
            operationId: `op-${planCalls.length}`,
            skillVersionId: request.skillVersionId,
            requiresConfirmation: false,
            warnings: [],
            targets: request.targets.map((target) => ({
              ...target,
              conflict: "none",
              warnings: [],
            })),
          };
        }
        case "execute_deployment":
          return {
            operationId: "op-1",
            skillVersionId: "skill-1",
            succeeded: 1,
            failed: 1,
            outcomes: [
              {
                agentId: "codex",
                workspaceId: "global-default",
                targetPath: "C:/agents/codex/skills/my-skill",
                mode: "junction",
                conflict: "none",
                ok: true,
                deployedHash: null,
                error: null,
              },
              {
                agentId: "cursor",
                workspaceId: "global-default",
                targetPath: "C:/agents/cursor/skills/my-skill",
                mode: "junction",
                conflict: "none",
                ok: false,
                deployedHash: null,
                error: "mklink /J failed: Access is denied.",
              },
            ],
          };
        default:
          return undefined;
      }
    });

    render(<DeployPage lang="en" />);
    await userEvent.selectOptions((await screen.findAllByRole("combobox"))[0], "skill-1");
    await userEvent.click(screen.getByRole("button", { name: /scan agents/i }));
    await screen.findByText("Codex");
    for (const checkbox of screen.getAllByRole("checkbox")) {
      await userEvent.click(checkbox);
    }
    await userEvent.click(screen.getByRole("radio", { name: /junction/i }));
    await userEvent.click(screen.getByRole("button", { name: /build plan/i }));
    await userEvent.click(await screen.findByRole("button", { name: /execute plan/i }));

    expect(await screen.findByText(/security software or local policy/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /review copy retry plan/i }));

    expect(planCalls).toHaveLength(2);
    expect(planCalls[1].request.targets).toEqual([
      {
        agentId: "cursor",
        workspaceId: "global-default",
        targetPath: "C:/agents/cursor/skills/my-skill",
        mode: "copy",
      },
    ]);
    expect(
      await screen.findAllByText("C:/agents/cursor/skills/my-skill"),
    ).toHaveLength(2);
    expect(screen.getByRole("button", { name: /execute plan/i })).toBeInTheDocument();
  });
});
