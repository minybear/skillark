//! Resolve concrete deployment target paths for a scope (global or project).
//!
//! For global scope the target is `<agent global skill dir>/<canonical>`. For a
//! project workspace the target is `<project root>/<agent skill dir>/<canonical>`
//! — computed by the real adapter's `project_skill_path`, so the per-agent
//! `.claude/skills`, `.codex/skills`, … rules live in exactly one place.

use std::path::PathBuf;

use crate::{
    adapters::{agents::built_in_adapters, custom_agent::custom_adapters},
    application::{agent_discovery::collect_context, agent_overrides},
    ports::AgentAdapter,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub agent_type: String,
    pub target_path: PathBuf,
}

/// Compute each requested agent's deployment target for a skill of
/// `canonical_name`. `project_root`, when `Some`, switches the scope from the
/// agent's user-level skill directory to a project-local one. The caller
/// supplies the disabled-agent set so this stays sync.
pub fn resolve_targets(
    disabled: &[String],
    canonical_name: &str,
    project_root: Option<&std::path::Path>,
    agent_types: &[String],
) -> Result<Vec<ResolvedTarget>, String> {
    let context = collect_context(std::collections::HashMap::new())?;
    let overrides = agent_overrides::load_overrides();

    // Build (adapter, detected candidate) pairs from both registries.
    let mut adapters: Vec<Box<dyn AgentAdapter>> = built_in_adapters();
    adapters.extend(custom_adapters(&overrides));

    let mut out = Vec::with_capacity(agent_types.len());
    for agent_type in agent_types {
        if disabled.iter().any(|a| a == agent_type) {
            continue;
        }
        let adapter = adapters
            .iter()
            .find(|a| a.kind().as_contract_value() == agent_type)
            .ok_or_else(|| format!("unknown agent type: {agent_type}"))?;

        let candidate = adapter
            .detect(&context)
            .into_iter()
            .next()
            .ok_or_else(|| format!("agent {agent_type} produced no candidate"))?;

        let base = match project_root {
            Some(root) => adapter.project_skill_path(&candidate, root),
            None => adapter.global_skill_path(&candidate),
        };

        let Some(base) = base else {
            // Skip agents with no resolvable skill path for this scope.
            continue;
        };

        out.push(ResolvedTarget {
            agent_type: agent_type.clone(),
            target_path: base.join(canonical_name),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn global_targets_use_agent_skill_dir() {
        // claude-code's adapter resolves a global skill dir under the home; the
        // resolved target must be that dir joined with the canonical name.
        let resolved =
            resolve_targets(&[], "my-skill", None, &["claude-code".to_owned()])
                .expect("resolve");
        assert_eq!(resolved.len(), 1);
        assert!(
            resolved[0]
                .target_path
                .to_string_lossy()
                .ends_with("my-skill"),
            "got {}",
            resolved[0].target_path.display()
        );
        assert!(
            resolved[0].target_path.to_string_lossy().contains("skills"),
            "global target should land under the agent skills dir: {}",
            resolved[0].target_path.display()
        );
    }

    #[test]
    fn project_targets_are_under_project_root() {
        let project = std::path::Path::new("D:/code/my-project");
        let resolved = resolve_targets(
            &[],
            "demo",
            Some(project),
            &["claude-code".to_owned(), "codex".to_owned()],
        )
        .expect("resolve");

        // Each resolved path starts with the project root and ends with the
        // canonical name, regardless of agent.
        for r in &resolved {
            let s = r.target_path.to_string_lossy().replace('\\', "/");
            assert!(
                s.starts_with("D:/code/my-project"),
                "project target must be under project root: {s}"
            );
            assert!(s.ends_with("demo"), "project target should end with canonical: {s}");
        }
        // Different agents get different skill subdirs.
        let kinds: Vec<_> = resolved
            .iter()
            .map(|r| r.agent_type.as_str())
            .collect();
        assert!(kinds.contains(&"claude-code"));
        assert!(kinds.contains(&"codex"));
    }

    #[test]
    fn unknown_agent_type_errors() {
        assert!(resolve_targets(&[], "x", None, &["nope".to_owned()]).is_err());
    }
}
