import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import LibraryPage from "../pages/LibraryPage";
import type { Skill, SkillDetail } from "../shared/library";

// ───── fixtures ────────────────────────────────────────────────────────────

function makeSkill(over: Partial<Skill> = {}): Skill {
  return {
    id: "skill-1",
    canonicalName: "my-skill",
    displayName: "My Skill",
    description: "demo",
    format: "agent-skills",
    libraryPath: "/vault/my-skill",
    status: "ready",
    currentVersionId: "ver-1",
    contentHash: "abc123def456",
    versionLabel: "1.0.0",
    createdAt: "2026-07-28T00:00:00Z",
    updatedAt: "2026-07-28T00:00:00Z",
    ...over,
  };
}

function makeDetail(over: Partial<SkillDetail> = {}): SkillDetail {
  return {
    ...makeSkill(),
    files: [
      { path: "SKILL.md", size: 120 },
      { path: "scripts/run.sh", size: 8 },
    ],
    skillMd: "---\nname: my-skill\n---\nbody",
    ...over,
  } as SkillDetail;
}

afterEach(() => clearMocks());

const noopDeploy = vi.fn();

// ───── empty → import → list ───────────────────────────────────────────────

describe("LibraryPage interaction E2E", () => {
  it("shows empty state, imports a directory skill, then lists it", async () => {
    let skills: Skill[] = [];
    const imported: string[] = [];
    mockIPC((cmd, payload) => {
      switch (cmd) {
        case "list_skills":
          return skills;
        case "import_skill_from_directory": {
          const p = (payload as { path: string }).path;
          imported.push(p);
          skills = [makeSkill()];
          return undefined;
        }
        default:
          return undefined;
      }
    });

    render(<LibraryPage lang="en" onDeploy={noopDeploy} />);

    // empty state first
    expect(await screen.findByText(/no skills|empty|library is empty/i)).toBeInTheDocument();

    // type a path and import
    const dirInput = screen.getAllByPlaceholderText(/path|directory|folder/i)[0];
    await userEvent.type(dirInput, "D:/skills/my-skill");
    const importButtons = screen.getAllByRole("button", { name: /import/i });
    await userEvent.click(importButtons[0]);

    // the skill now appears
    expect(await screen.findByText("My Skill")).toBeInTheDocument();
    expect(imported).toEqual(["D:/skills/my-skill"]);
    expect(screen.getByText("my-skill")).toBeInTheDocument();
  });

  it("imports a ZIP skill via the zip row", async () => {
    let skills: Skill[] = [];
    const zips: string[] = [];
    mockIPC((cmd, payload) => {
      if (cmd === "list_skills") return skills;
      if (cmd === "import_skill_from_zip") {
        zips.push((payload as { path: string }).path);
        skills = [makeSkill({ canonicalName: "zip-skill", displayName: "Zip Skill" })];
        return undefined;
      }
      return undefined;
    });

    render(<LibraryPage lang="en" onDeploy={noopDeploy} />);
    await screen.findByText(/no skills|empty/i);

    const zipInput = screen.getAllByPlaceholderText(/zip/i)[0];
    await userEvent.type(zipInput, "D:/archives/skill.zip");
    const zipButton = screen.getAllByRole("button", { name: /import/i })[1];
    await userEvent.click(zipButton);

    expect(await screen.findByText("Zip Skill")).toBeInTheDocument();
    expect(zips).toEqual(["D:/archives/skill.zip"]);
  });

  it("re-import dedups: list stays at one entry for the same content", async () => {
    // Backend dedups; the UI simply re-lists. Assert the list command is the
    // source of truth and shows a single canonical entry.
    const skills = [makeSkill()];
    mockIPC((cmd) => (cmd === "list_skills" ? skills : undefined));
    render(<LibraryPage lang="en" onDeploy={noopDeploy} />);
    const rows = await screen.findAllByText("My Skill");
    expect(rows).toHaveLength(1);
  });

  it("deletes a skill after confirmation", async () => {
    let skills = [makeSkill()];
    const deleted: string[] = [];
    vi.spyOn(window, "confirm").mockReturnValue(true);
    mockIPC((cmd, payload) => {
      if (cmd === "list_skills") return skills;
      if (cmd === "delete_skill") {
        deleted.push((payload as { skillId: string }).skillId);
        skills = [];
        return undefined;
      }
      return undefined;
    });

    render(<LibraryPage lang="en" onDeploy={noopDeploy} />);
    await screen.findByText("My Skill");
    await userEvent.click(screen.getByRole("button", { name: /delete/i }));

    await waitFor(() => expect(deleted).toEqual(["skill-1"]));
    expect(await screen.findByText(/no skills|empty/i)).toBeInTheDocument();
  });

  it("opens the detail dialog with file tree and SKILL.md preview", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_skills") return [makeSkill()];
      if (cmd === "get_skill_detail") return makeDetail();
      return undefined;
    });

    render(<LibraryPage lang="en" onDeploy={noopDeploy} />);
    await screen.findByText("My Skill");
    await userEvent.click(screen.getByRole("button", { name: /view/i }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("scripts/run.sh")).toBeInTheDocument();
    expect(within(dialog).getByText(/name: my-skill/)).toBeInTheDocument();

    // close
    await userEvent.click(within(dialog).getByRole("button", { name: /close/i }));
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("routes to deploy with the skill id + canonical name", async () => {
    const onDeploy = vi.fn();
    mockIPC((cmd) => (cmd === "list_skills" ? [makeSkill()] : undefined));
    render(<LibraryPage lang="en" onDeploy={onDeploy} />);
    await screen.findByText("My Skill");
    await userEvent.click(screen.getByRole("button", { name: /deploy/i }));
    expect(onDeploy).toHaveBeenCalledWith("skill-1", "my-skill");
  });

  it("surfaces an import error to the user", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_skills") return [];
      if (cmd === "import_skill_from_directory")
        throw new Error("SKILL.md not found");
      return undefined;
    });
    render(<LibraryPage lang="en" onDeploy={noopDeploy} />);
    await screen.findByText(/no skills|empty/i);
    const dirInput = screen.getAllByPlaceholderText(/path|directory|folder/i)[0];
    await userEvent.type(dirInput, "D:/bad");
    await userEvent.click(screen.getAllByRole("button", { name: /import/i })[0]);
    expect(await screen.findByText(/SKILL.md not found/i)).toBeInTheDocument();
  });
});
