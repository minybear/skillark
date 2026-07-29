#[derive(Clone, Debug)]
pub struct BootstrapStatus {
    pub project: &'static str,
    pub version: &'static str,
    pub phase: &'static str,
    pub next_milestone: &'static str,
    pub foundations: [&'static str; 4],
}

pub fn current_status() -> BootstrapStatus {
    BootstrapStatus {
        project: "SkillArk",
        version: "0.1.0",
        phase: "M4 · Release verification",
        next_milestone: "v0.1 · Windows release",
        foundations: [
            "Tauri 2 + React",
            "Rust domain core",
            "SQLite migrations",
            "JSON contract tests",
        ],
    }
}
