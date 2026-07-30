//! v0.2 Link Bridge integration: preview → import_candidate records full Git
//! provenance against a real database. The git fetch is unit-tested in
//! `adapters::sources::git`; here we feed a materialized checkout (simulating a
//! fetched repo) so the provenance + import wiring is exercised end-to-end.

use std::path::PathBuf;

use skillark_lib::{
    adapters::sqlite::connect,
    application::link_bridge::{scan_repository, LinkImportService, LinkPreview},
    domain::link_bridge::RepositoryLocator,
};
use sqlx::Row;

async fn setup_db() -> (sqlx::SqlitePool, PathBuf) {
    let root = unique_dir();
    let db_url = format!("sqlite:{}", root.join("db.sqlite").to_string_lossy());
    let pool = connect(&db_url).await.unwrap();
    let vault = root.join("vault");
    std::fs::create_dir_all(&vault).unwrap();
    (pool, vault)
}

fn unique_dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let p = home
        .join(".skillark-linkbridge-test")
        .join(format!("{}-{}", std::process::id(), uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn materialize_checkout(dir: &std::path::Path, name: &str, version: &str) {
    std::fs::create_dir_all(dir.join("scripts")).unwrap();
    std::fs::write(
        dir.join("SKILL.md"),
        format!(
            "---\nname: {name}\nversion: {version}\ndescription: link-bridge e2e\n---\n# {name}\n"
        ),
    )
    .unwrap();
    std::fs::write(dir.join("scripts/run.sh"), "echo hi").unwrap();
}

#[tokio::test]
async fn import_candidate_records_git_provenance() {
    let (pool, vault) = setup_db().await;
    let root = unique_dir();
    let fetch_cache = root.join("fetch-cache");
    std::fs::create_dir_all(&fetch_cache).unwrap();

    // Simulate a fetched working tree for https://github.com/acme/cool-skill.
    let checkout = root.join("checkout");
    materialize_checkout(&checkout, "cool-skill", "1.2.0");

    let candidates = scan_repository(&checkout, None).expect("scan");
    assert_eq!(candidates.len(), 1);

    let resolved_revision = "abc123def456789012345678901234567890abcd"; // 40-char SHA
    let preview = LinkPreview {
        locator: RepositoryLocator {
            remote: "https://github.com/acme/cool-skill.git".to_owned(),
            host: "github.com".to_owned(),
            owner_repo: Some("acme/cool-skill".to_owned()),
            requested_ref: Some("main".to_owned()),
            subpath: None,
        },
        resolved_revision: resolved_revision.to_owned(),
        candidates,
        checkout_dir: checkout.clone(),
    };

    let service = LinkImportService::new(pool.clone(), vault, fetch_cache);
    let outcome = service
        .import_candidate(&preview, 0)
        .await
        .expect("import candidate");

    // The imported version carries the resolved commit.
    let version_row = sqlx::query("SELECT source_revision, content_hash FROM skill_versions WHERE id = ?")
        .bind(outcome.version_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    let stamped_revision: String = version_row.get("source_revision");
    assert_eq!(stamped_revision, resolved_revision);
    assert_eq!(version_row.get::<String, _>("content_hash"), outcome.content_hash);

    // A git source row exists with the canonical remote.
    let source_row = sqlx::query("SELECT source_type, base_url FROM sources WHERE source_type = 'git'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(source_row.get::<String, _>("base_url"), "https://github.com/acme/cool-skill.git");

    // The skill is linked to that source.
    let skill_row = sqlx::query("SELECT source_id FROM skills WHERE id = ?")
        .bind(outcome.skill_id.to_string())
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!skill_row.get::<Option<String>, _>("source_id").is_null_or_empty());

    // source_revisions records the immutable fetch identity.
    let rev_row = sqlx::query(
        "SELECT content_hash FROM source_revisions WHERE resolved_revision = ?",
    )
    .bind(resolved_revision)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rev_row.get::<String, _>("content_hash"), outcome.content_hash);
}

#[tokio::test]
async fn reimport_same_link_is_idempotent_and_dedups_source() {
    let (pool, vault) = setup_db().await;
    let root = unique_dir();
    let fetch_cache = root.join("fetch-cache");
    std::fs::create_dir_all(&fetch_cache).unwrap();

    let checkout = root.join("checkout");
    materialize_checkout(&checkout, "dup-skill", "1.0.0");
    let candidates = scan_repository(&checkout, None).unwrap();
    let resolved = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
    let preview = LinkPreview {
        locator: RepositoryLocator {
            remote: "https://github.com/o/dup.git".to_owned(),
            host: "github.com".to_owned(),
            owner_repo: Some("o/dup".to_owned()),
            requested_ref: None,
            subpath: None,
        },
        resolved_revision: resolved.to_owned(),
        candidates,
        checkout_dir: checkout.clone(),
    };
    let service = LinkImportService::new(pool.clone(), vault, fetch_cache);

    service.import_candidate(&preview, 0).await.unwrap();
    service.import_candidate(&preview, 0).await.unwrap();

    // Exactly one git source row (dedup by base_url).
    let n_sources: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sources WHERE source_type = 'git'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n_sources, 1);

    // Exactly one source_revisions row (dedup by unique index).
    let n_revs: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM source_revisions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n_revs, 1);
}

// Extension trait so the assertion reads cleanly; Option<String> has no
// is_null_or_empty in stable Rust.
trait OptStrExt {
    fn is_null_or_empty(&self) -> bool;
}
impl OptStrExt for Option<String> {
    fn is_null_or_empty(&self) -> bool {
        self.as_ref().map(|s| s.is_empty()).unwrap_or(true)
    }
}
