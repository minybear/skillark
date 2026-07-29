//! Skill and SkillVersion domain models for the persistence layer.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub id: Uuid,
    pub canonical_name: String,
    pub display_name: String,
    pub description: String,
    pub format: String,
    pub library_path: String,
    pub status: SkillStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SkillStatus {
    Ready,
    Corrupted,
    Missing,
}

impl std::fmt::Display for SkillStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ready => write!(f, "ready"),
            Self::Corrupted => write!(f, "corrupted"),
            Self::Missing => write!(f, "missing"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SkillVersion {
    pub id: Uuid,
    pub skill_id: Uuid,
    pub version_label: Option<String>,
    pub source_revision: Option<String>,
    pub content_hash: String,
    pub manifest_json: String,
    pub library_snapshot_path: String,
    pub created_at: String,
}
