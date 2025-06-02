use anyhow::Result;
use gitdb::services::SyncService;
use gitdb::storage::GitDatabase;
use std::sync::Arc;
use tempfile::TempDir;

async fn create_test_service() -> Result<(SyncService, Arc<GitDatabase>, TempDir)> {
    let temp_dir = TempDir::new()?;
    unsafe {
        std::env::set_var("GITDB_DATA_DIR", temp_dir.path());
    }
    let db = Arc::new(GitDatabase::new().await?);
    
    // Use a test token if available, otherwise None
    let github_token = std::env::var("GITDB_TEST_GITHUB_TOKEN").ok();
    let service = SyncService::new(db.clone(), github_token)?;
    
    Ok((service, db, temp_dir))
}

#[tokio::test]
#[ignore] // This test requires network access
async fn test_sync_public_repository() -> Result<()> {
    let (service, db, _temp_dir) = create_test_service().await?;

    // Use the test repository
    let result = service.sync_repository("tacogips/gitdb-test-1", false).await?;
    
    assert!(result.issues_synced > 0 || result.pull_requests_synced > 0);
    assert!(result.errors.is_empty());

    // Verify repository was saved
    let repo = db.get_repository_by_full_name("tacogips/gitdb-test-1").await?;
    assert!(repo.is_some());

    Ok(())
}

#[tokio::test]
async fn test_parse_repo_url() -> Result<()> {
    use gitdb::services::parse_repo_url;

    // Test various URL formats
    let test_cases = vec![
        ("https://github.com/owner/repo", ("owner", "repo")),
        ("https://github.com/owner/repo.git", ("owner", "repo")),
        ("git@github.com:owner/repo.git", ("owner", "repo")),
        ("owner/repo", ("owner", "repo")),
    ];

    for (url, expected) in test_cases {
        let (owner, name) = parse_repo_url(url)?;
        assert_eq!(owner, expected.0);
        assert_eq!(name, expected.1);
    }

    // Test invalid format
    assert!(parse_repo_url("invalid-url").is_err());

    Ok(())
}