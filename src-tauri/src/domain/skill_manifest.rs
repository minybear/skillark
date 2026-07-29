//! SKILL.md front-matter parser.
//!
//! SkillArk stores skill metadata as a simple YAML-like front-matter block at
//! the top of a `SKILL.md` file, delimited by `---`. This module parses that
//! block into a [`SkillManifest`] domain model.
//!
//! Design note: the manifest format is intentionally simple (`key: value`
//! pairs), so we hand-roll the parser instead of pulling in a YAML crate. This
//! keeps the domain layer dependency-free and the parsing behaviour explicit.

use serde::{Deserialize, Serialize};

/// Metadata parsed from the front-matter of a `SKILL.md` file.
///
/// This is distinct from the deployment-time [`SkillManifestDto`](crate::commands::contracts::SkillManifestDto),
/// which aggregates file hashes and conflict warnings after a skill is scanned
/// on disk. This struct captures only what the author declared up front.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Why parsing failed. Callers can branch on this to give actionable feedback
/// (e.g. "missing required field `name` in SKILL.md") instead of a generic
/// error string.
#[derive(Clone, Debug, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParseErrorKind {
    /// The file has no `---`-delimited front-matter block at all.
    MissingFrontMatter,
    /// A line could not be interpreted as a `key: value` pair.
    InvalidYaml,
    /// The required `name` field is absent.
    MissingName,
    /// The required `version` field is absent.
    MissingVersion,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind_label(), self.message)
    }
}

impl std::error::Error for ParseError {}

impl ParseError {
    fn kind_label(&self) -> &'static str {
        match self.kind {
            ParseErrorKind::MissingFrontMatter => "missing front-matter",
            ParseErrorKind::InvalidYaml => "invalid yaml",
            ParseErrorKind::MissingName => "missing name",
            ParseErrorKind::MissingVersion => "missing version",
        }
    }
}

/// Parse a `SKILL.md` document into a [`SkillManifest`].
///
/// Accepted shape:
///
/// ```text
/// ---
/// name: my-skill
/// version: 1.0.0
/// description: A short blurb.
/// entry: ./main.md
/// tags: rust, cli, tools
/// ---
/// # Body markdown…
/// ```
///
/// - Only the first `---`-delimited block is treated as front-matter.
/// - `name` and `version` are required; every other field is optional.
/// - `tags` may be a comma-separated scalar. Surrounding whitespace per tag is
///   trimmed, empty tags are dropped.
/// - Unknown keys are ignored (forward compatibility).
/// - Leading/trailing whitespace on every value is trimmed.
pub fn parse_skill_md(content: &str) -> Result<SkillManifest, ParseError> {
    let body = extract_front_matter(content)?;

    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut description = String::new();
    let mut entry: Option<String> = None;
    let mut tags: Vec<String> = Vec::new();

    for (line_no, raw_line) in body.lines().enumerate() {
        let line = raw_line.trim();
        // Skip blank lines and comments inside front-matter.
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = split_kv(line).ok_or_else(|| ParseError {
            kind: ParseErrorKind::InvalidYaml,
            message: format!("line {}: expected `key: value`, got `{line}`", line_no + 1),
        })?;

        let key = key.trim();
        let value = value.trim();

        match key {
            "name" => name = Some(value.to_owned()),
            "version" => version = Some(value.to_owned()),
            "description" => description = value.to_owned(),
            "entry" => entry = if value.is_empty() { None } else { Some(value.to_owned()) },
            "tags" => {
                tags = value
                    .split(',')
                    .map(|t| t.trim().to_owned())
                    .filter(|t| !t.is_empty())
                    .collect();
            }
            // Forward-compat: ignore unknown keys silently.
            _ => {}
        }
    }

    let name = name.ok_or_else(|| ParseError {
        kind: ParseErrorKind::MissingName,
        message: "SKILL.md front-matter is missing the required `name` field".to_owned(),
    })?;
    let version = version.ok_or_else(|| ParseError {
        kind: ParseErrorKind::MissingVersion,
        message: "SKILL.md front-matter is missing the required `version` field".to_owned(),
    })?;

    Ok(SkillManifest {
        name,
        version,
        description,
        entry,
        tags,
    })
}

/// Extract the text between the first pair of `---` delimiters.
///
/// Returns the raw (un-split) front-matter body, or an error when no opening
/// delimiter is found at the top of the document.
fn extract_front_matter(content: &str) -> Result<String, ParseError> {
    // The opening `---` must be the first non-blank line.
    let mut lines = content.lines();
    let first = loop {
        match lines.next() {
            Some(l) if l.trim().is_empty() => continue,
            Some(l) => break l,
            None => {
                return Err(ParseError {
                    kind: ParseErrorKind::MissingFrontMatter,
                    message: "SKILL.md is empty".to_owned(),
                })
            }
        }
    };

    if first.trim() != "---" {
        return Err(ParseError {
            kind: ParseErrorKind::MissingFrontMatter,
            message: "SKILL.md must start with a `---` front-matter delimiter".to_owned(),
        });
    }

    // Collect lines until the closing `---`.
    let mut body = Vec::new();
    let mut closed = false;
    for line in lines {
        if line.trim() == "---" {
            closed = true;
            break;
        }
        body.push(line);
    }

    if !closed {
        return Err(ParseError {
            kind: ParseErrorKind::MissingFrontMatter,
            message: "SKILL.md front-matter is never closed with a trailing `---`".to_owned(),
        });
    }

    Ok(body.join("\n"))
}

/// Split a single `key: value` line into its parts.
///
/// Returns `None` when there is no `:` at all. The key/value halves are
/// returned untrimmed so the caller can report the original line on error.
fn split_kv(line: &str) -> Option<(&str, &str)> {
    let idx = line.find(':')?;
    Some((&line[..idx], &line[idx + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_skill_md() {
        let content = "---\n\
name: my-skill\n\
version: 1.2.0\n\
description: A handy skill.\n\
entry: ./main.md\n\
tags: rust, cli, automation\n\
---\n\
# Body\n\
Some markdown here.\n";
        let manifest = parse_skill_md(content).expect("valid manifest parses");

        assert_eq!(manifest.name, "my-skill");
        assert_eq!(manifest.version, "1.2.0");
        assert_eq!(manifest.description, "A handy skill.");
        assert_eq!(manifest.entry.as_deref(), Some("./main.md"));
        assert_eq!(manifest.tags, vec!["rust", "cli", "automation"]);
    }

    #[test]
    fn rejects_missing_front_matter() {
        let content = "# No front matter here\n\nJust a plain markdown body.";
        let err = parse_skill_md(content).expect_err("should reject missing front-matter");
        assert_eq!(err.kind, ParseErrorKind::MissingFrontMatter);
    }

    #[test]
    fn rejects_missing_name() {
        let content = "---\nversion: 1.0.0\n---\nbody";
        let err = parse_skill_md(content).expect_err("should reject missing name");
        assert_eq!(err.kind, ParseErrorKind::MissingName);
    }

    #[test]
    fn rejects_missing_version() {
        let content = "---\nname: my-skill\n---\nbody";
        let err = parse_skill_md(content).expect_err("should reject missing version");
        assert_eq!(err.kind, ParseErrorKind::MissingVersion);
    }

    #[test]
    fn parses_minimal_with_only_name_and_version() {
        let content = "---\nname: minimal\nversion: 0.1.0\n---\n";
        let manifest = parse_skill_md(content).expect("minimal manifest parses");

        assert_eq!(manifest.name, "minimal");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.description, "");
        assert_eq!(manifest.entry, None);
        assert!(manifest.tags.is_empty());
    }

    #[test]
    fn parses_with_optional_entry_field() {
        let content = "---\nname: with-entry\nversion: 2.0.0\nentry: src/run.sh\n---\n";
        let manifest = parse_skill_md(content).expect("entry manifest parses");

        assert_eq!(manifest.entry.as_deref(), Some("src/run.sh"));
    }

    #[test]
    fn rejects_unclosed_front_matter() {
        let content = "---\nname: dangling\nversion: 1.0.0\nbody without closing delimiter";
        let err = parse_skill_md(content).expect_err("should reject unclosed front-matter");
        assert_eq!(err.kind, ParseErrorKind::MissingFrontMatter);
    }

    #[test]
    fn ignores_unknown_keys() {
        let content = "---\nname: forward-compat\nversion: 1.0.0\nauthor: someone\nfuture: yes\n---\n";
        let manifest = parse_skill_md(content).expect("unknown keys are ignored");
        assert_eq!(manifest.name, "forward-compat");
    }

    #[test]
    fn empty_entry_becomes_none() {
        let content = "---\nname: e\nversion: 1.0.0\nentry:\n---\n";
        let manifest = parse_skill_md(content).expect("empty entry parses");
        assert_eq!(manifest.entry, None);
    }
}
