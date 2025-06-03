use anyhow::Result;
use gitdb::ids::{IssueId, IssueNumber, PullRequestId, PullRequestNumber, RepositoryId};
use gitdb::storage::{CrossReference, GitDatabase, Repository, SyncStatus};
use gitdb::types::{IssueState, ItemType, PullRequestState, ResourceType, SyncStatusType};
use tempfile::TempDir;

async fn create_test_db() -> Result<(GitDatabase, TempDir)> {
    let temp_dir = TempDir::new()?;
    
    // Create unique directories for this test
    let test_id = uuid::Uuid::new_v4();
    let data_dir = temp_dir.path().join(format!("data_{}", test_id));
    let config_dir = temp_dir.path().join(format!("config_{}", test_id));
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&config_dir)?;
    
    // Use static mutex to ensure env vars are set/unset atomically
    use once_cell::sync::Lazy;
    use std::sync::Mutex;
    
    static ENV_MUTEX: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));
    
    let db = {
        let _guard = ENV_MUTEX.lock().unwrap();
        unsafe {
            std::env::set_var("GITDB_DATA_DIR", &data_dir);
            std::env::set_var("GITDB_CONFIG_DIR", &config_dir);
        }
        let db = GitDatabase::new().await?;
        unsafe {
            std::env::remove_var("GITDB_DATA_DIR");
            std::env::remove_var("GITDB_CONFIG_DIR");
        }
        db
    };
    
    Ok((db, temp_dir))
}

#[tokio::test]
async fn test_repository_operations() -> Result<()> {
    let (db, _temp_dir) = create_test_db().await?;

    let repo = Repository {
        id: RepositoryId::new(1),
        owner: "test".to_string(),
        name: "repo".to_string(),
        full_name: "test/repo".to_string(),
        description: Some("Test repository".to_string()),
        stars: 42,
        forks: 10,
        language: Some("Rust".to_string()),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        indexed_at: chrono::Utc::now(),
    };

    // Test save
    db.save_repository(&repo).await?;

    // Test get by full name
    let retrieved = db.get_repository_by_full_name("test/repo").await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.full_name, "test/repo");
    assert_eq!(retrieved.stars, 42);

    // Test list repositories
    let repos = db.list_repositories().await?;
    assert_eq!(repos.len(), 1);
    assert_eq!(repos[0].full_name, "test/repo");

    Ok(())
}

#[tokio::test]
async fn test_issue_operations() -> Result<()> {
    let (db, _temp_dir) = create_test_db().await?;

    let issue = gitdb::storage::Issue {
        id: IssueId::new(1),
        repository_id: RepositoryId::new(1),
        number: IssueNumber::new(123),
        title: "Test Issue".to_string(),
        body: Some("This is a test issue".to_string()),
        state: IssueState::Open,
        author: "testuser".to_string(),
        assignees: vec!["assignee1".to_string()],
        labels: vec!["bug".to_string(), "urgent".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        closed_at: None,
        comments_count: 0,
    };

    // Test save
    db.save_issue(&issue).await?;

    // Test get by repository
    let issues = db
        .list_issues_by_repository(&RepositoryId::new(1))
        .await?;
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].title, "Test Issue");
    assert_eq!(issues[0].state, IssueState::Open);

    Ok(())
}

#[tokio::test]
async fn test_pull_request_operations() -> Result<()> {
    let (db, _temp_dir) = create_test_db().await?;

    let pr = gitdb::storage::PullRequest {
        id: PullRequestId::new(1),
        repository_id: RepositoryId::new(1),
        number: PullRequestNumber::new(456),
        title: "Test PR".to_string(),
        body: Some("This is a test pull request".to_string()),
        state: PullRequestState::Open,
        author: "testuser".to_string(),
        assignees: vec!["reviewer1".to_string()],
        labels: vec!["feature".to_string()],
        head_ref: "feature-branch".to_string(),
        base_ref: "main".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        merged_at: None,
        closed_at: None,
        comments_count: 0,
        commits_count: 3,
        additions: 100,
        deletions: 50,
        changed_files: 5,
    };

    // Test save
    db.save_pull_request(&pr).await?;

    // Test get by repository
    let prs = db
        .list_pull_requests_by_repository(&RepositoryId::new(1))
        .await?;
    assert_eq!(prs.len(), 1);
    assert_eq!(prs[0].title, "Test PR");
    assert_eq!(prs[0].state, PullRequestState::Open);

    Ok(())
}

#[tokio::test]
async fn test_sync_status_operations() -> Result<()> {
    let (db, _temp_dir) = create_test_db().await?;

    let status = SyncStatus {
        id: gitdb::ids::SyncStatusId::new(0),
        repository_id: RepositoryId::new(1),
        resource_type: ResourceType::Issues,
        last_synced_at: chrono::Utc::now(),
        status: SyncStatusType::Success,
        error_message: None,
        items_synced: 10,
    };

    // Test save
    db.save_sync_status(&status).await?;

    // Test get sync status
    let last_status = db
        .get_sync_status(&RepositoryId::new(1), ResourceType::Issues)
        .await?;
    assert!(last_status.is_some());
    let last_status = last_status.unwrap();
    assert_eq!(last_status.status, SyncStatusType::Success);
    assert_eq!(last_status.items_synced, 10);

    Ok(())
}

#[tokio::test]
async fn test_cross_reference_operations() -> Result<()> {
    let (db, _temp_dir) = create_test_db().await?;

    let cross_ref = CrossReference {
        source_type: ItemType::Issue,
        source_id: 1,
        source_repository_id: RepositoryId::new(1),
        target_type: ItemType::PullRequest,
        target_repository_id: RepositoryId::new(1),
        target_number: 456,
        link_text: "test/repo#456".to_string(),
        created_at: chrono::Utc::now(),
    };

    // Test save
    db.save_cross_reference(&cross_ref).await?;

    // Test list by source repository
    let refs_by_source = db.list_cross_references_from(
        &RepositoryId::new(1),
    ).await?;
    let refs_by_source: Vec<_> = refs_by_source.into_iter()
        .filter(|xref| xref.source_type == ItemType::Issue && xref.source_id == 1)
        .collect();
    assert_eq!(refs_by_source.len(), 1);
    assert_eq!(refs_by_source[0].target_type, ItemType::PullRequest);
    assert_eq!(refs_by_source[0].target_number, 456);

    // Test list by target repository
    let refs_by_target = db.list_cross_references_to(
        &RepositoryId::new(1),
    ).await?;
    let refs_by_target: Vec<_> = refs_by_target.into_iter()
        .filter(|xref| xref.target_type == ItemType::PullRequest && xref.target_number == 456)
        .collect();
    assert_eq!(refs_by_target.len(), 1);
    assert_eq!(refs_by_target[0].source_type, ItemType::Issue);
    assert_eq!(refs_by_target[0].source_id, 1);

    Ok(())
}

#[tokio::test]
async fn test_search_functionality() -> Result<()> {
    let (db, _temp_dir) = create_test_db().await?;

    // Add test data
    let issue1 = gitdb::storage::Issue {
        id: IssueId::new(1),
        repository_id: RepositoryId::new(1),
        number: IssueNumber::new(100),
        title: "Bug in authentication module".to_string(),
        body: Some("The login function fails when password contains special characters".to_string()),
        state: IssueState::Open,
        author: "alice".to_string(),
        assignees: vec![],
        labels: vec!["bug".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        closed_at: None,
        comments_count: 0,
    };

    let issue2 = gitdb::storage::Issue {
        id: IssueId::new(2),
        repository_id: RepositoryId::new(1),
        number: IssueNumber::new(101),
        title: "Feature request: Add password reset".to_string(),
        body: Some("Users should be able to reset their passwords via email".to_string()),
        state: IssueState::Open,
        author: "bob".to_string(),
        assignees: vec![],
        labels: vec!["enhancement".to_string()],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        closed_at: None,
        comments_count: 0,
    };

    db.save_issue(&issue1).await?;
    db.save_issue(&issue2).await?;

    // Wait for search index to update (OnCommitWithDelay requires some time)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Test search
    let results = db.search("password", 10).await?;
    assert_eq!(results.len(), 2);

    // Test search with specific terms
    let results = db.search("authentication", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Bug in authentication module");

    Ok(())
}