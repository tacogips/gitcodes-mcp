use anyhow::Result;
use gitdb::storage::{SearchStore, SearchResult, search_store::LanceDbQuery};
use gitdb::types::{GitHubRepository, GitHubIssue, GitHubPullRequest, GitHubUser};
use gitdb::ids::FullId;
use tempfile::TempDir;
use chrono::Utc;

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
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
    // Store should be created successfully
    assert!(temp_dir.path().exists());
    
    Ok(())
}

#[tokio::test]
async fn test_save_and_retrieve_repository() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
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
async fn test_search_repositories() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
    // Add multiple repositories
    let repo1 = create_test_github_repository(1, "rust-lang", "rust");
    let repo2 = create_test_github_repository(2, "tokio-rs", "tokio");
    let repo3 = create_test_github_repository(3, "hyperium", "hyper");
    
    store.save_repository(&repo1).await?;
    store.save_repository(&repo2).await?;
    store.save_repository(&repo3).await?;
    
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
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
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
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
    // Add repository
    let repo = create_test_github_repository(1, "async-rs", "async-std");
    store.save_repository(&repo).await?;
    
    // Add issue
    let mut issue = create_test_github_issue(1, repo.full_id(), 1);
    issue.title = "Async executor improvements".to_string();
    store.save_issue(&issue).await?;
    
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
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
    // Create repositories with various content
    let mut repo1 = create_test_github_repository(1, "test", "optimization-tools");
    repo1.description = Some("Tools for optimizing Rust code performance".to_string());
    
    let mut repo2 = create_test_github_repository(2, "test", "optimize-rs");
    repo2.description = Some("A library to optimize various algorithms".to_string());
    
    let mut repo3 = create_test_github_repository(3, "test", "perf-monitor");
    repo3.description = Some("Performance monitoring for applications".to_string());
    
    store.save_repository(&repo1).await?;
    store.save_repository(&repo2).await?;
    store.save_repository(&repo3).await?;
    
    // Test partial word matching
    let query = LanceDbQuery::new("optim").with_limit(10);
    let results = store.search_repositories(&query).await?;
    assert!(results.len() >= 2); // Should match "optimization" and "optimize"
    
    // Test case insensitive search
    let query_lower = LanceDbQuery::new("rust").with_limit(10);
    let results_lower = store.search_repositories(&query_lower).await?;
    let query_upper = LanceDbQuery::new("RUST").with_limit(10);
    let results_upper = store.search_repositories(&query_upper).await?;
    assert_eq!(results_lower.len(), results_upper.len());
    
    Ok(())
}

#[tokio::test]
async fn test_search_with_special_characters() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let store = SearchStore::new(temp_dir.path().to_path_buf()).await?;
    
    // Create repository with special characters
    let mut repo = create_test_github_repository(1, "test-org", "test-repo");
    repo.description = Some("Test repo with /api/v1 endpoints".to_string());
    store.save_repository(&repo).await?;
    
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
    
    let temp_dir = TempDir::new()?;
    let store = Arc::new(SearchStore::new(temp_dir.path().to_path_buf()).await?);
    
    // Add some data
    for i in 1..=5 {
        let repo = create_test_github_repository(i, "test", &format!("repo{}", i));
        store.save_repository(&repo).await?;
    }
    
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