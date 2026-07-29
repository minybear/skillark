//! Import a skill source into the central vault and persist it.
//!
//! Flow (PRD §5.1, ARCHITECTURE §3.2 `ImportSkillService`):
//! 1. scan the source → manifest + source root
//! 2. hash the source root (content-addressed)
//! 3. copy into `<vault>/<canonical>/<hash12>` unless that snapshot already exists
//! 4. verify the copy hashes identically
//! 5. upsert the Skill row (by canonical name) and a SkillVersion (dedup by hash)
//! 6. point the skill's `current_version_id` at the new version

use std::path::PathBuf;

use sqlx::SqlitePool;
use uuid::Uuid;

use crate::{
    adapters::{filesystem::copy_tree, sources::ZipSource},
    application::state::{hash_prefix, sanitize_canonical},
    domain::{
        content_hash::hash_directory,
        operation::{OperationStatus, OperationType},
        skill::SkillStatus,
    },
    ports::{ScannedSource, SkillSourceAdapter},
    repositories::{OperationRepository, SkillRepository},
};

use crate::adapters::sources::LocalDirectorySource;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub skill_id: Uuid,
    pub version_id: Uuid,
    pub canonical_name: String,
    pub display_name: String,
    pub content_hash: String,
    /// True when an identical version already existed and was reused.
    pub reused_version: bool,
}

#[derive(Clone)]
pub struct ImportSkillService {
    vault_path: PathBuf,
    pool: SqlitePool,
}

impl ImportSkillService {
    pub fn new(vault_path: PathBuf, pool: SqlitePool) -> Self {
        Self { vault_path, pool }
    }

    pub async fn import_directory(&self, source: PathBuf) -> Result<ImportOutcome, String> {
        let operation_id = Uuid::new_v4().to_string();
        let plan_json = import_plan_json("directory", &source)?;
        let operations = OperationRepository::new(self.pool.clone());
        operations
            .create(&operation_id, OperationType::Import, &plan_json)
            .await?;

        let result = match LocalDirectorySource::new(source)
            .scan()
            .map_err(|e| e.to_string())
        {
            Ok(scanned) => self.import_scanned(scanned).await,
            Err(error) => Err(error),
        };
        complete_import(&operations, &operation_id, result).await
    }

    pub async fn import_zip(&self, zip_path: PathBuf) -> Result<ImportOutcome, String> {
        let operation_id = Uuid::new_v4().to_string();
        let plan_json = import_plan_json("zip", &zip_path)?;
        let operations = OperationRepository::new(self.pool.clone());
        operations
            .create(&operation_id, OperationType::Import, &plan_json)
            .await?;

        // Extract into a scratch sibling, scan it, then clean up.
        let scratch = unique_scratch();
        let zip = ZipSource::new(zip_path);
        let result = match zip.extract_to(&scratch).map_err(|e| e.to_string()) {
            Ok(()) => match LocalDirectorySource::new(scratch.clone())
                .scan()
                .map_err(|e| e.to_string())
            {
                Ok(scanned) => self.import_scanned(scanned).await,
                Err(error) => Err(error),
            },
            Err(error) => Err(error),
        };
        let _ = crate::adapters::filesystem::force_remove_dir_all(&scratch);
        complete_import(&operations, &operation_id, result).await
    }

    /// Core import path shared by directory and zip sources.
    pub async fn import_scanned(&self, scanned: ScannedSource) -> Result<ImportOutcome, String> {
        let canonical = sanitize_canonical(&scanned.manifest.name);
        let display_name = scanned.manifest.name.clone();
        let manifest_json = serde_json::to_string(&scanned.manifest)
            .map_err(|e| format!("serialize manifest: {e}"))?;

        let source_root = scanned.source_root.clone();
        let hash = tauri::async_runtime::spawn_blocking(move || hash_directory(&source_root))
            .await
            .map_err(|e| format!("hash task: {e}"))??;

        let snapshot_dir = self.vault_path.join(&canonical).join(hash_prefix(&hash));

        // Materialize the immutable snapshot if missing.
        let vault_root = self.vault_path.clone();
        let snap = snapshot_dir.clone();
        let src = scanned.source_root.clone();
        let expected_hash = hash.clone();
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            if !snap.exists() {
                copy_tree(&src, &snap).map_err(|e| e.to_string())?;
            }
            // Verify the snapshot hashes as expected.
            let observed = hash_directory(&snap).map_err(|e| e.to_string())?;
            if observed != expected_hash {
                return Err(format!(
                    "snapshot hash mismatch after copy: expected {expected_hash}, got {observed}"
                ));
            }
            let _ = &vault_root; // vault anchor (unused but documents intent)
            Ok(())
        })
        .await
        .map_err(|e| format!("copy task: {e}"))??;

        // Persist. Dedup the skill (by canonical name) and the version (by hash).
        let repo = SkillRepository::new(self.pool.clone());
        let skill = match repo.find_by_canonical_name(&canonical).await? {
            Some(existing) => existing,
            None => {
                let now = chrono::Utc::now().to_rfc3339();
                let new_skill = crate::domain::skill::Skill {
                    id: Uuid::new_v4(),
                    canonical_name: canonical.clone(),
                    display_name: display_name.clone(),
                    description: scanned.manifest.description.clone(),
                    format: "agent-skills".to_owned(),
                    library_path: self.vault_path.join(&canonical).to_string_lossy().into_owned(),
                    status: SkillStatus::Ready,
                    created_at: now.clone(),
                    updated_at: now,
                };
                repo.create_skill(&new_skill).await?;
                new_skill
            }
        };

        // Reuse an identical version when the content hash already exists.
        if let Some(version) = repo.find_version_by_hash(skill.id, &hash).await? {
            repo.set_current_version(skill.id, version.id).await?;
            return Ok(ImportOutcome {
                skill_id: skill.id,
                version_id: version.id,
                canonical_name: canonical,
                display_name: skill.display_name,
                content_hash: hash,
                reused_version: true,
            });
        }

        let version = crate::domain::skill::SkillVersion {
            id: Uuid::new_v4(),
            skill_id: skill.id,
            version_label: Some(scanned.manifest.version.clone()),
            source_revision: None,
            content_hash: hash.clone(),
            manifest_json,
            library_snapshot_path: snapshot_dir.to_string_lossy().into_owned(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        repo.create_skill_version(&version).await?;
        repo.set_current_version(skill.id, version.id).await?;

        Ok(ImportOutcome {
            skill_id: skill.id,
            version_id: version.id,
            canonical_name: canonical,
            display_name: skill.display_name,
            content_hash: hash,
            reused_version: false,
        })
    }
}

fn import_plan_json(source_type: &str, source: &std::path::Path) -> Result<String, String> {
    serde_json::to_string(&serde_json::json!({
        "sourceType": source_type,
        "sourcePath": source.to_string_lossy(),
    }))
    .map_err(|error| format!("serialize import plan: {error}"))
}

async fn complete_import(
    operations: &OperationRepository,
    operation_id: &str,
    result: Result<ImportOutcome, String>,
) -> Result<ImportOutcome, String> {
    match result {
        Ok(outcome) => {
            let result_json = serde_json::to_string(&outcome)
                .map_err(|error| format!("serialize import result: {error}"))?;
            operations
                .complete(
                    operation_id,
                    OperationStatus::Succeeded,
                    Some(&result_json),
                    None,
                )
                .await?;
            Ok(outcome)
        }
        Err(error) => {
            operations
                .complete(
                    operation_id,
                    OperationStatus::Failed,
                    None,
                    Some(&error),
                )
                .await?;
            Err(error)
        }
    }
}

fn unique_scratch() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!(
        "skillark-zip-extract-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).ok();
    p
}
