use anyhow::Result;
use gitdb::storage::{GitDatabase, Repository, Issue, PullRequest};
use gitdb::ids::{RepositoryId, IssueId, PullRequestId};
use gitdb::types::ResourceType;
use std::sync::Arc;
use tempfile::TempDir;

/// Helper to create test repository
fn create_test_repository(id: u64, name: &str, description: &str) -> Repository {
    Repository {
        id: RepositoryId::new(id),
        full_name: format!("test-org/{}", name),
        description: Some(description.to_string()),
        language: Some("Rust".to_string()),
        topics: vec!["rust".to_string(), "testing".to_string()],
        updated_at: chrono::Utc::now(),
        stargazers_count: 100,
        fork: false,
    }
}

/// Helper to create test issue
fn create_test_issue(id: u64, repo_id: RepositoryId, title: &str, body: &str, labels: Vec<String>) -> Issue {
    Issue {
        id: IssueId::new(id),
        repository_id: repo_id,
        number: id as i32,
        title: title.to_string(),
        body: Some(body.to_string()),
        state: "open".to_string(),
        author: "test-user".to_string(),
        assignees: vec![],
        labels,
        comments_count: 0,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        closed_at: None,
    }
}

/// Helper to create test pull request
fn create_test_pr(id: u64, repo_id: RepositoryId, title: &str, body: &str, labels: Vec<String>) -> PullRequest {
    PullRequest {
        id: PullRequestId::new(id),
        repository_id: repo_id,
        number: id as i32,
        title: title.to_string(),
        body: Some(body.to_string()),
        state: "open".to_string(),
        author: "test-user".to_string(),
        assignees: vec![],
        labels,
        base_ref: "main".to_string(),
        head_ref: "feature-branch".to_string(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        closed_at: None,
        merged_at: None,
        draft: false,
    }
}

#[tokio::test]
async fn test_search_repositories() -> Result<()> {
    // Create a temporary database
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    // Add test repositories
    let repo1 = create_test_repository(1, "tokio-async", "Async runtime for Rust");
    let repo2 = create_test_repository(2, "reqwest-http", "HTTP client library");
    let repo3 = create_test_repository(3, "serde-json", "JSON serialization framework");
    
    db.save_repository(&repo1).await?;
    db.save_repository(&repo2).await?;
    db.save_repository(&repo3).await?;
    
    // Search for "async"
    let results = db.search("async", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_type, "repository");
    assert!(results[0].title.contains("tokio-async"));
    
    // Search for "http"
    let results = db.search("http", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].item_type, "repository");
    assert!(results[0].title.contains("reqwest-http"));
    
    Ok(())
}

#[tokio::test]
async fn test_search_issues() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    // Add a repository first
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    // Add test issues
    let issue1 = create_test_issue(
        1, 
        repo.id, 
        "Async runtime panic", 
        "The tokio runtime panics when shutting down",
        vec!["bug".to_string(), "runtime".to_string()]
    );
    let issue2 = create_test_issue(
        2,
        repo.id,
        "Add retry logic",
        "Implement exponential backoff for HTTP requests",
        vec!["enhancement".to_string(), "http".to_string()]
    );
    let issue3 = create_test_issue(
        3,
        repo.id,
        "Documentation improvements",
        "Add more examples for async patterns",
        vec!["documentation".to_string()]
    );
    
    db.save_issue(&issue1).await?;
    db.save_issue(&issue2).await?;
    db.save_issue(&issue3).await?;
    
    // Search for "async"
    let results = db.search("async", 10).await?;
    assert!(results.len() >= 2); // Should find issue1 and issue3
    
    // Search for "http"
    let results = db.search("http", 10).await?;
    assert!(results.iter().any(|r| r.title.contains("retry logic")));
    
    // Search in labels
    let results = db.search("bug", 10).await?;
    assert!(results.iter().any(|r| r.title.contains("runtime panic")));
    
    Ok(())
}

#[tokio::test]
async fn test_search_pull_requests() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    // Add a repository
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    // Add test PRs
    let pr1 = create_test_pr(
        1,
        repo.id,
        "Fix memory leak in async executor",
        "This PR fixes a memory leak that occurs when tasks are dropped",
        vec!["bug".to_string(), "memory".to_string()]
    );
    let pr2 = create_test_pr(
        2,
        repo.id,
        "Optimize vector allocations",
        "Pre-allocate vectors to reduce memory allocations in hot paths",
        vec!["performance".to_string(), "optimization".to_string()]
    );
    
    db.save_pull_request(&pr1).await?;
    db.save_pull_request(&pr2).await?;
    
    // Search for "memory"
    let results = db.search("memory", 10).await?;
    assert!(results.len() >= 2); // Both PRs mention memory
    
    // Search for "optimization"
    let results = db.search("optimization", 10).await?;
    assert!(results.iter().any(|r| r.title.contains("vector allocations")));
    
    Ok(())
}

#[tokio::test]
async fn test_search_across_types() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    // Add mixed content
    let repo = create_test_repository(1, "async-toolkit", "Comprehensive async toolkit");
    db.save_repository(&repo).await?;
    
    let issue = create_test_issue(
        1,
        repo.id,
        "Async task scheduling",
        "Improve async task scheduling algorithm",
        vec!["enhancement".to_string()]
    );
    db.save_issue(&issue).await?;
    
    let pr = create_test_pr(
        1,
        repo.id,
        "Implement async stream processing",
        "Add support for async stream processing with backpressure",
        vec!["feature".to_string()]
    );
    db.save_pull_request(&pr).await?;
    
    // Search for "async" should find all three
    let results = db.search("async", 10).await?;
    assert!(results.len() >= 3);
    
    // Verify we have different types
    let types: Vec<&str> = results.iter().map(|r| r.item_type.as_str()).collect();
    assert!(types.contains(&"repository"));
    assert!(types.contains(&"issue"));
    assert!(types.contains(&"pull_request"));
    
    Ok(())
}

#[tokio::test]
async fn test_search_case_insensitive() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    let issue = create_test_issue(
        1,
        repo.id,
        "CRITICAL BUG in HTTP handler",
        "The HTTP handler crashes on malformed input",
        vec!["bug".to_string()]
    );
    db.save_issue(&issue).await?;
    
    // Test case variations
    let searches = vec!["CRITICAL", "critical", "Critical", "cRiTiCaL"];
    
    for query in searches {
        let results = db.search(query, 10).await?;
        assert_eq!(results.len(), 1, "Failed for query: {}", query);
        assert!(results[0].title.contains("CRITICAL"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_search_special_characters() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    let repo = create_test_repository(1, "rust-http/server", "HTTP server implementation");
    db.save_repository(&repo).await?;
    
    let issue = create_test_issue(
        1,
        repo.id,
        "Fix path /api/v1/users",
        "The endpoint /api/v1/users returns 404",
        vec!["bug".to_string()]
    );
    db.save_issue(&issue).await?;
    
    // Search with special characters
    let results = db.search("rust-http", 10).await?;
    assert!(!results.is_empty());
    
    let results = db.search("/api/v1", 10).await?;
    assert!(!results.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_search_empty_results() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    // Add some content
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    // Search for non-existent term
    let results = db.search("nonexistentterm12345", 10).await?;
    assert_eq!(results.len(), 0);
    
    Ok(())
}

#[tokio::test]
async fn test_search_limit() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    // Add many issues with common term
    for i in 1..=10 {
        let issue = create_test_issue(
            i,
            repo.id,
            &format!("Issue {} with async code", i),
            "This issue involves async programming",
            vec!["async".to_string()]
        );
        db.save_issue(&issue).await?;
    }
    
    // Search with limit
    let results = db.search("async", 5).await?;
    assert_eq!(results.len(), 5);
    
    Ok(())
}

#[tokio::test]
async fn test_search_by_author() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    // Create issues with different authors
    let mut issue1 = create_test_issue(1, repo.id, "First issue", "Description", vec![]);
    issue1.author = "alice".to_string();
    
    let mut issue2 = create_test_issue(2, repo.id, "Second issue", "Description", vec![]);
    issue2.author = "bob".to_string();
    
    db.save_issue(&issue1).await?;
    db.save_issue(&issue2).await?;
    
    // Search by author
    let results = db.search("alice", 10).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "First issue");
    
    Ok(())
}

#[tokio::test]
async fn test_search_by_labels() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    let issue1 = create_test_issue(
        1,
        repo.id,
        "Security issue",
        "Security vulnerability found",
        vec!["security".to_string(), "high-priority".to_string()]
    );
    let issue2 = create_test_issue(
        2,
        repo.id,
        "Performance issue",
        "Slow response times",
        vec!["performance".to_string(), "low-priority".to_string()]
    );
    
    db.save_issue(&issue1).await?;
    db.save_issue(&issue2).await?;
    
    // Search by label
    let results = db.search("security", 10).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("Security"));
    
    let results = db.search("high-priority", 10).await?;
    assert_eq!(results.len(), 1);
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_search_operations() -> Result<()> {
    std::env::set_var("GITDB_TEST_MODE", "true");
    let db = Arc::new(GitDatabase::new().await?);
    
    // Add test data
    let repo = create_test_repository(1, "test-repo", "Test repository");
    db.save_repository(&repo).await?;
    
    for i in 1..=5 {
        let issue = create_test_issue(
            i,
            repo.id,
            &format!("Issue {}", i),
            "Test issue content",
            vec!["test".to_string()]
        );
        db.save_issue(&issue).await?;
    }
    
    // Perform concurrent searches
    let db_clone1 = Arc::clone(&db);
    let db_clone2 = Arc::clone(&db);
    let db_clone3 = Arc::clone(&db);
    
    let handle1 = tokio::spawn(async move {
        db_clone1.search("test", 10).await
    });
    let handle2 = tokio::spawn(async move {
        db_clone2.search("issue", 10).await
    });
    let handle3 = tokio::spawn(async move {
        db_clone3.search("content", 10).await
    });
    
    let results1 = handle1.await??;
    let results2 = handle2.await??;
    let results3 = handle3.await??;
    
    assert!(!results1.is_empty());
    assert!(!results2.is_empty());
    assert!(!results3.is_empty());
    
    Ok(())
}