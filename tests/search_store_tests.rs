use anyhow::Result;
use gitdb::storage::{SearchStore, SearchResult, search_store::LanceDbQuery};
use gitdb::types::{GitHubRepository, GitHubIssue, GitHubUser, FullId};
use std::path::PathBuf;
use chrono::Utc;

// Helper to clean up test directory after test completion
fn defer_cleanup(path: PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        let _ = std::fs::remove_dir_all(&path);
    });
}

async fn create_test_store() -> Result<(SearchStore, PathBuf)> {
    // Use dirs crate to get temp dir path
    let temp_base = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("gitdb_search_tests");
    std::fs::create_dir_all(&temp_base)?;
    
    // Create unique directory for this test
    let test_id = uuid::Uuid::new_v4();
    let test_dir = temp_base.join(format!("test_{}", test_id));
    std::fs::create_dir_all(&test_dir)?;
    
    // Set environment variable to skip vector index creation
    unsafe {
        std::env::set_var("GITDB_SKIP_VECTOR_INDEX", "1");
    }
    
    let store = SearchStore::new(test_dir.clone()).await?;
    
    unsafe {
        std::env::remove_var("GITDB_SKIP_VECTOR_INDEX");
    }
    
    Ok((store, test_dir))
}

/// Helper to create a test repository
fn create_test_github_repository(id: u64, owner: &str, name: &str) -> GitHubRepository {
    GitHubRepository {
        id,
        owner: owner.to_string(),
        name: name.to_string(),
        full_name: format!("{}/{}", owner, name),
        description: Some(format!("Description for {}", name)),
        url: format!("https://github.com/{}/{}", owner, name),
        clone_url: format!("https://github.com/{}/{}.git", owner, name),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        language: Some("Rust".to_string()),
        fork: false,
        forks_count: 10,
        stargazers_count: 100,
        open_issues_count: 5,
        is_template: Some(false),
        topics: vec!["rust".to_string(), "async".to_string()],
        visibility: "public".to_string(),
        default_branch: "main".to_string(),
        permissions: None,
        license: None,
        archived: false,
        disabled: false,
    }
}

/// Helper to create a test issue
fn create_test_github_issue(id: u64, repo_id: FullId, number: i32) -> GitHubIssue {
    GitHubIssue {
        id,
        repository_id: repo_id,
        number,
        title: format!("Issue #{}", number),
        body: Some(format!("This is the body of issue #{}", number)),
        state: "open".to_string(),
        user: GitHubUser {
            id: 1,
            login: "testuser".to_string(),
            name: Some("Test User".to_string()),
            email: None,
            avatar_url: None,
            html_url: "https://github.com/testuser".to_string(),
            bio: None,
            company: None,
            location: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            public_repos: 10,
            followers: 100,
            following: 50,
        },
        assignees: vec![],
        labels: vec![],
        milestone: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
    }
}

#[tokio::test]
async fn test_search_store_creation() -> Result<()> {
    let (_store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir.clone());
    
    // Store should be created successfully
    assert!(test_dir.exists());
    
    Ok(())
}

#[tokio::test]
async fn test_save_and_retrieve_repository() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    let repo = create_test_github_repository(1, "test", "repo1");
    store.save_repository(&repo).await?;
    
    // Retrieve the repository
    let full_id = repo.full_id();
    let retrieved = store.get_repository(&full_id).await?;
    
    assert!(retrieved.is_some());
    let retrieved_repo = retrieved.unwrap();
    assert_eq!(retrieved_repo.full_name, "test/repo1");
    assert_eq!(retrieved_repo.id, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_debug_search() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add a simple repository
    let repo = create_test_github_repository(1, "test", "simple-repo");
    store.save_repository(&repo).await?;
    
    // Verify it was saved
    let retrieved = store.get_repository(&repo.full_id()).await?;
    println!("Repository saved: {}", retrieved.is_some());
    
    // Create FTS index after adding data
    match store.create_fts_index_repositories().await {
        Ok(_) => println!("FTS index created successfully"),
        Err(e) => println!("Failed to create FTS index: {}", e),
    }
    
    // Try immediate search (might fail)
    let query = LanceDbQuery::new("simple").with_limit(10);
    let results1 = store.search_repositories(&query).await?;
    println!("Immediate search results: {}", results1.len());
    
    // Wait and try again
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    let results2 = store.search_repositories(&query).await?;
    println!("After 1s wait results: {}", results2.len());
    
    // Try searching for full name
    let query = LanceDbQuery::new("simple-repo").with_limit(10);
    let results3 = store.search_repositories(&query).await?;
    println!("Full name search results: {}", results3.len());
    
    // Try exact match on full_name
    let query = LanceDbQuery::new("test/simple-repo").with_limit(10);
    let results4 = store.search_repositories(&query).await?;
    println!("Exact full name search results: {}", results4.len());
    
    Ok(())
}

#[tokio::test]
async fn test_search_repositories() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add multiple repositories
    let repo1 = create_test_github_repository(1, "rust-lang", "rust");
    let repo2 = create_test_github_repository(2, "tokio-rs", "tokio");
    let repo3 = create_test_github_repository(3, "hyperium", "hyper");
    
    store.save_repository(&repo1).await?;
    store.save_repository(&repo2).await?;
    store.save_repository(&repo3).await?;
    
    // Create FTS index after adding data
    store.create_fts_index_repositories().await?;
    
    
    // Search for "tokio"
    let query = LanceDbQuery::new("tokio").with_limit(10);
    let results = store.search_repositories(&query).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].full_name, "tokio-rs/tokio");
    
    // Search for "rust"
    let query = LanceDbQuery::new("rust").with_limit(10);
    let results = store.search_repositories(&query).await?;
    assert!(results.len() >= 1);
    assert!(results.iter().any(|r| r.full_name == "rust-lang/rust"));
    
    Ok(())
}

#[tokio::test]
async fn test_save_and_search_issues() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Create a repository first
    let repo = create_test_github_repository(1, "test", "repo");
    store.save_repository(&repo).await?;
    
    // Create issues
    let mut issue1 = create_test_github_issue(1, repo.full_id(), 1);
    issue1.title = "Async runtime panic".to_string();
    issue1.body = Some("The async runtime panics on shutdown".to_string());
    issue1.labels = vec![
        gitdb::types::GitHubLabel {
            id: 1,
            name: "bug".to_string(),
            color: "red".to_string(),
            description: Some("Bug report".to_string()),
        }
    ];
    
    let mut issue2 = create_test_github_issue(2, repo.full_id(), 2);
    issue2.title = "Add HTTP/2 support".to_string();
    issue2.body = Some("We should add support for HTTP/2 protocol".to_string());
    issue2.labels = vec![
        gitdb::types::GitHubLabel {
            id: 2,
            name: "enhancement".to_string(),
            color: "blue".to_string(),
            description: Some("Feature request".to_string()),
        }
    ];
    
    store.save_issue(&issue1).await?;
    store.save_issue(&issue2).await?;
    
    // Create FTS index for issues
    store.create_fts_index_issues().await?;
    
    
    // Search for "async"
    let query = LanceDbQuery::new("async").with_limit(10);
    let results = store.search_issues(&query).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("Async"));
    
    // Search for "HTTP"
    let query = LanceDbQuery::new("HTTP").with_limit(10);
    let results = store.search_issues(&query).await?;
    assert_eq!(results.len(), 1);
    assert!(results[0].title.contains("HTTP/2"));
    
    // Search in labels
    let query = LanceDbQuery::new("bug").with_limit(10);
    let results = store.search_issues(&query).await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].number, 1);
    
    Ok(())
}

#[tokio::test]
async fn test_search_all() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add repository
    let repo = create_test_github_repository(1, "async-rs", "async-std");
    store.save_repository(&repo).await?;
    
    // Add issue
    let mut issue = create_test_github_issue(1, repo.full_id(), 1);
    issue.title = "Async executor improvements".to_string();
    store.save_issue(&issue).await?;
    
    // Create FTS indexes
    store.create_fts_index_repositories().await?;
    store.create_fts_index_issues().await?;
    
    
    // Search across all types
    let query = LanceDbQuery::new("async").with_limit(10);
    let results = store.search_all(&query).await?;
    
    // Should find both repository and issue
    assert!(results.len() >= 2);
    
    // Check result types
    let has_repo = results.iter().any(|r| matches!(r, SearchResult::Repository(_)));
    let has_issue = results.iter().any(|r| matches!(r, SearchResult::Issue(_)));
    
    assert!(has_repo);
    assert!(has_issue);
    
    Ok(())
}

#[tokio::test]
async fn test_full_text_search_features() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Create a simple repository first
    let mut repo1 = create_test_github_repository(1, "test", "simple");
    repo1.description = Some("Simple test repository".to_string());
    
    store.save_repository(&repo1).await?;
    
    // Create FTS index after adding data
    store.create_fts_index_repositories().await?;
    
    
    // Verify the repository was saved
    let saved_repo = store.get_repository(&repo1.full_id()).await?;
    println!("Repository saved: {}", saved_repo.is_some());
    
    // Try to search for the exact word
    let query = LanceDbQuery::new("simple").with_limit(10);
    let results = store.search_repositories(&query).await?;
    println!("Search for 'simple' returned {} results", results.len());
    
    // Try to search for partial content
    let query2 = LanceDbQuery::new("test").with_limit(10);
    let results2 = store.search_repositories(&query2).await?;
    println!("Search for 'test' returned {} results", results2.len());
    
    // Try to search for description content
    let query3 = LanceDbQuery::new("repository").with_limit(10);
    let results3 = store.search_repositories(&query3).await?;
    println!("Search for 'repository' returned {} results", results3.len());
    
    // Assert at least one search works
    assert!(results.len() > 0 || results2.len() > 0 || results3.len() > 0, 
            "At least one search should return results");
    
    Ok(())
}

#[tokio::test]
async fn test_search_with_special_characters() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Create repository with special characters
    let mut repo = create_test_github_repository(1, "test-org", "test-repo");
    repo.description = Some("Test repo with /api/v1 endpoints".to_string());
    store.save_repository(&repo).await?;
    
    // Create FTS index
    store.create_fts_index_repositories().await?;
    
    
    // Search with special characters
    let query = LanceDbQuery::new("test-org").with_limit(10);
    let results = store.search_repositories(&query).await?;
    assert!(!results.is_empty());
    
    let query = LanceDbQuery::new("/api/v1").with_limit(10);
    let results = store.search_repositories(&query).await?;
    assert!(!results.is_empty());
    
    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    use std::sync::Arc;
    
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    let store = Arc::new(store);
    
    // Add some data
    for i in 1..=5 {
        let repo = create_test_github_repository(i, "test", &format!("repo{}", i));
        store.save_repository(&repo).await?;
    }
    
    // Create FTS index after adding data
    store.create_fts_index_repositories().await?;
    
    
    // Perform concurrent searches
    let mut handles = vec![];
    
    for i in 0..3 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let query_str = format!("repo{}", i + 1);
            let query = LanceDbQuery::new(query_str).with_limit(10);
            store_clone.search_repositories(&query).await
        });
        handles.push(handle);
    }
    
    // Wait for all searches to complete
    for handle in handles {
        let results = handle.await??;
        assert!(!results.is_empty());
    }
    
    Ok(())
}