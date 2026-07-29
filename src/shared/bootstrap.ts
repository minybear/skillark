export type BootstrapStatus = {
  project: string;
  version: string;
  phase: string;
  nextMilestone: string;
  foundations: string[];
};

export const browserFallbackStatus: BootstrapStatus = {
  project: "SkillArk",
  version: "0.1.0",
  phase: "M4 · Release verification",
  nextMilestone: "v0.1 · Windows release",
  foundations: [
    "Tauri 2 + React",
    "Rust domain core",
    "SQLite migrations",
    "JSON contract tests",
  ],
};
