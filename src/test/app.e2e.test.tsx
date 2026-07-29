import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import App from "../app/App";

afterEach(() => clearMocks());

// A minimal IPC surface the whole app can boot against.
function installAppIPC(opts: { skills?: unknown[]; operations?: unknown[] } = {}) {
  const skills = opts.skills ?? [];
  const operations = opts.operations ?? [];
  mockIPC((cmd) => {
    switch (cmd) {
      case "list_skills":
        return skills;
      case "list_operations":
        return operations;
      case "list_deployments":
        return [];
      case "list_workspaces":
        return [];
      case "get_disabled_agents":
        return [];
      case "get_agent_overrides":
        return [];
      case "discover_agents":
        return [];
      case "get_bootstrap_status":
      case "bootstrap_status":
        return { agents: [], vaultPath: "C:/vault", dbPath: "C:/db" };
      default:
        return undefined;
    }
  });
}

describe("App first-run + navigation E2E", () => {
  it("empty library auto-opens the first-run wizard; dismissing remembers it", async () => {
    installAppIPC({ skills: [], operations: [] });
    render(<App />);

    // wizard appears for a genuinely empty library
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toBeInTheDocument();
    expect(window.localStorage.getItem("skillark.firstRunDone")).toBeNull();

    // close it via the ✕ (skip)
    await userEvent.click(screen.getByRole("button", { name: /close/i }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    expect(window.localStorage.getItem("skillark.firstRunDone")).toBe("1");
  });

  it("non-empty library does not open the wizard", async () => {
    installAppIPC({
      skills: [
        {
          id: "s1",
          canonicalName: "x",
          displayName: "X",
          description: "",
          format: "agent-skills",
          libraryPath: "/v/x",
          status: "ready",
          currentVersionId: "v1",
          contentHash: "h",
          versionLabel: "1.0.0",
          createdAt: "t",
          updatedAt: "t",
        },
      ],
      operations: [{ id: "o1" }],
    });
    render(<App />);
    // give the effect a tick to (not) fire
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("navigates across all five pages", async () => {
    installAppIPC({ skills: [{}], operations: [{}] }); // non-empty → no wizard
    render(<App />);

    // default lang is zh: 概览/技能库/代理/工作区/操作记录
    await userEvent.click(screen.getByRole("button", { name: "技能库" }));
    // library page container renders
    await waitFor(() =>
      expect(document.querySelector(".page-library")).not.toBeNull(),
    );

    await userEvent.click(screen.getByRole("button", { name: "代理" }));
    await userEvent.click(screen.getByRole("button", { name: "工作区" }));
    await userEvent.click(screen.getByRole("button", { name: "操作记录" }));
    // operations page renders its heading
    expect(
      await screen.findAllByText("操作记录", { selector: "h2, h3" }),
    ).not.toHaveLength(0);
  });
});
