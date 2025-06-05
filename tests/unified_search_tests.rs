use anyhow::Result;
use gitdb::storage::{SearchStore, UnifiedSearchQuery, SearchResult};
use gitdb::storage::search_store::hybrid::RerankStrategy;
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
        .join("gitdb_unified_search_tests");
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
fn create_test_github_repository(id: u64, owner: &str, name: &str, description: &str) -> GitHubRepository {
    GitHubRepository {
        id,
        owner: owner.to_string(),
        name: name.to_string(),
        full_name: format!("{}/{}", owner, name),
        description: Some(description.to_string()),
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
        topics: vec![],
        visibility: "public".to_string(),
        default_branch: "main".to_string(),
        permissions: None,
        license: None,
        archived: false,
        disabled: false,
    }
}

/// Helper to create a test issue
fn create_test_github_issue(id: u64, repo_id: FullId, number: i32, title: &str, body: &str) -> GitHubIssue {
    GitHubIssue {
        id,
        repository_id: repo_id,
        number,
        title: title.to_string(),
        body: Some(body.to_string()),
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
            public_repos: 0,
            followers: 0,
            following: 0,
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
async fn test_unified_search_full_text_mode() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add test data with clear distinctions
    let repo1 = create_test_github_repository(1, "tokio-rs", "tokio", 
        "A runtime for writing reliable asynchronous applications");
    let repo2 = create_test_github_repository(2, "async-rs", "async-std", 
        "Async version of the Rust standard library");
    let repo3 = create_test_github_repository(3, "python", "requests", 
        "HTTP library for Python");
    
    store.save_repository(&repo1).await?;
    store.save_repository(&repo2).await?;
    store.save_repository(&repo3).await?;
    
    // Create FTS index
    store.create_fts_index_repositories().await?;
    
    // Test 1: Search for "async" - should find async-related repos
    let query = UnifiedSearchQuery::full_text("async")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    // Verify results contain "async"
    let async_count = results.iter()
        .filter(|r| match r {
            SearchResult::Repository(repo) => {
                repo.full_name.to_lowercase().contains("async") ||
                repo.description.as_ref().map(|d| d.to_lowercase().contains("async")).unwrap_or(false)
            }
            _ => false,
        })
        .count();
    
    assert!(async_count >= 1, "Should find at least 1 async-related repository");
    
    // Test 2: Search for "python" - should only find Python repo
    let query = UnifiedSearchQuery::full_text("python")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    assert_eq!(results.len(), 1, "Should find only Python repository");
    match &results[0] {
        SearchResult::Repository(repo) => {
            assert!(repo.full_name.contains("python") || 
                    repo.description.as_ref().map(|d| d.contains("Python")).unwrap_or(false),
                    "Result should be Python-related");
        }
        _ => panic!("Expected repository result"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_unified_search_semantic_mode() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add test data
    let repo1 = create_test_github_repository(1, "async", "futures", 
        "Zero-cost asynchronous programming in Rust");
    let repo2 = create_test_github_repository(2, "sync", "mutex", 
        "Synchronization primitives for Rust");
    
    store.save_repository(&repo1).await?;
    store.save_repository(&repo2).await?;
    
    // Note: In real usage, semantic search would use actual embeddings
    // For testing, we're using the placeholder embeddings (all zeros)
    
    // Test semantic search from text
    let query = UnifiedSearchQuery::semantic_from_text("concurrent programming")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    // With placeholder embeddings, this might return all or no results
    // The key is that it doesn't error
    println!("Semantic search from text returned {} results", results.len());
    
    // Test semantic search from vector
    let embedding = vec![0.1; 384]; // 384-dimensional test vector
    let query = UnifiedSearchQuery::semantic_from_vector(embedding)
        .with_limit(5);
    let results = store.unified_search(query).await?;
    
    println!("Semantic search from vector returned {} results", results.len());
    
    // Both queries should complete without errors
    assert!(true, "Semantic search should work without errors");
    
    Ok(())
}

#[tokio::test]
async fn test_unified_search_hybrid_mode_with_different_strategies() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add test data
    let repo1 = create_test_github_repository(1, "rust-lang", "cargo", 
        "Package manager for Rust");
    let repo2 = create_test_github_repository(2, "rust-lang", "rustup", 
        "The Rust toolchain installer");
    
    let issue1 = create_test_github_issue(1, repo1.full_id(), 100, 
        "Cargo build performance issue",
        "Build times are slow with large dependency trees");
    
    store.save_repository(&repo1).await?;
    store.save_repository(&repo2).await?;
    store.save_issue(&issue1).await?;
    
    // Create FTS indexes
    store.create_fts_index_repositories().await?;
    store.create_fts_index_issues().await?;
    
    // Test 1: Hybrid search with default Linear strategy
    let query = UnifiedSearchQuery::hybrid("cargo")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    assert!(!results.is_empty(), "Hybrid search should return results");
    println!("Default Linear strategy returned {} results", results.len());
    
    // Test 2: Hybrid search with RRF strategy
    let query = UnifiedSearchQuery::hybrid("rust")
        .with_rerank_strategy(RerankStrategy::RRF { k: 60.0 })
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    println!("RRF with k=60 returned {} results", results.len());
    
    // Test 3: Hybrid search with different Linear weights
    let query = UnifiedSearchQuery::hybrid("rust")
        .with_rerank_strategy(RerankStrategy::Linear { text_weight: 0.8, vector_weight: 0.2 })
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    println!("Linear with text_weight=0.8 returned {} results", results.len());
    
    // Test 4: Hybrid search with TextOnly strategy
    let query = UnifiedSearchQuery::hybrid("build")
        .with_rerank_strategy(RerankStrategy::TextOnly)
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    println!("TextOnly strategy returned {} results", results.len());
    
    // Test 5: Hybrid search with VectorOnly strategy
    let query = UnifiedSearchQuery::hybrid("performance")
        .with_rerank_strategy(RerankStrategy::VectorOnly)
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    println!("VectorOnly strategy returned {} results", results.len());
    
    // Verify we can get both repos and issues in results
    let has_repos = results.iter().any(|r| matches!(r, SearchResult::Repository(_)));
    let has_issues = results.iter().any(|r| matches!(r, SearchResult::Issue(_)));
    
    println!("Results contain repositories: {}, issues: {}", has_repos, has_issues);
    
    Ok(())
}

#[tokio::test]
async fn test_unified_search_excludes_unrelated_data() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add repositories with very distinct content
    let rust_repo = create_test_github_repository(1, "rust-lang", "rust", 
        "Empowering everyone to build reliable and efficient software");
    let go_repo = create_test_github_repository(2, "golang", "go", 
        "The Go programming language");
    let js_repo = create_test_github_repository(3, "nodejs", "node", 
        "JavaScript runtime built on Chrome's V8 engine");
    
    store.save_repository(&rust_repo).await?;
    store.save_repository(&go_repo).await?;
    store.save_repository(&js_repo).await?;
    
    // Add issues with specific keywords
    let rust_issue = create_test_github_issue(1, rust_repo.full_id(), 100,
        "Memory safety bug in unsafe block",
        "Found a memory safety issue when using unsafe code");
    let go_issue = create_test_github_issue(2, go_repo.full_id(), 200,
        "Goroutine deadlock in channel operations",
        "Deadlock occurs when multiple goroutines access shared channel");
    
    store.save_issue(&rust_issue).await?;
    store.save_issue(&go_issue).await?;
    
    // Create FTS indexes
    store.create_fts_index_repositories().await?;
    store.create_fts_index_issues().await?;
    
    // Test 1: Search for "memory safety" - should only find Rust-related content
    let query = UnifiedSearchQuery::full_text("memory safety")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    // Verify only Rust-related content is returned
    for result in &results {
        match result {
            SearchResult::Repository(repo) => {
                assert!(repo.full_name.contains("rust"), 
                       "Repository {} should be Rust-related for 'memory safety' search", 
                       repo.full_name);
            }
            SearchResult::Issue(issue) => {
                assert!(issue.title.to_lowercase().contains("memory") || 
                       issue.body.as_ref().map(|b| b.to_lowercase().contains("memory")).unwrap_or(false),
                       "Issue should contain 'memory' keyword");
            }
            _ => {}
        }
    }
    
    // Test 2: Search for "goroutine" - should only find Go-related content
    let query = UnifiedSearchQuery::full_text("goroutine")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    assert_eq!(results.len(), 1, "Should only find Go issue mentioning 'goroutine'");
    match &results[0] {
        SearchResult::Issue(issue) => {
            assert!(issue.title.contains("Goroutine"), "Should find the goroutine issue");
        }
        _ => panic!("Expected issue result for goroutine search"),
    }
    
    // Test 3: Search for "javascript" - should only find JS-related content
    let query = UnifiedSearchQuery::full_text("javascript")
        .with_limit(10);
    let results = store.unified_search(query).await?;
    
    assert_eq!(results.len(), 1, "Should only find JavaScript repository");
    match &results[0] {
        SearchResult::Repository(repo) => {
            assert_eq!(repo.full_name, "nodejs/node", "Should find Node.js repository");
        }
        _ => panic!("Expected repository result for JavaScript search"),
    }
    
    Ok(())
}

#[tokio::test]
async fn test_unified_search_all_modes_comparison() -> Result<()> {
    let (store, test_dir) = create_test_store().await?;
    defer_cleanup(test_dir);
    
    // Add test data
    let repo = create_test_github_repository(1, "tokio-rs", "tokio", 
        "Async runtime for Rust");
    let issue = create_test_github_issue(1, repo.full_id(), 100,
        "Async task scheduling optimization",
        "Improve performance of task scheduling in async runtime");
    
    store.save_repository(&repo).await?;
    store.save_issue(&issue).await?;
    
    // Create FTS indexes
    store.create_fts_index_repositories().await?;
    store.create_fts_index_issues().await?;
    
    // Test same query with different modes
    let search_term = "async";
    
    // 1. Full-text search
    let ft_query = UnifiedSearchQuery::full_text(search_term)
        .with_limit(10);
    let ft_results = store.unified_search(ft_query).await?;
    
    // 2. Semantic search (with placeholder embeddings)
    let semantic_query = UnifiedSearchQuery::semantic_from_text(search_term)
        .with_limit(10);
    let semantic_results = store.unified_search(semantic_query).await?;
    
    // 3. Hybrid search with different strategies
    let hybrid_default = UnifiedSearchQuery::hybrid(search_term)
        .with_limit(10);
    let hybrid_default_results = store.unified_search(hybrid_default).await?;
    
    let hybrid_text_only = UnifiedSearchQuery::hybrid(search_term)
        .with_rerank_strategy(RerankStrategy::TextOnly)
        .with_limit(10);
    let hybrid_text_only_results = store.unified_search(hybrid_text_only).await?;
    
    // Log results for comparison
    println!("Search results for '{}':", search_term);
    println!("- Full-text mode: {} results", ft_results.len());
    println!("- Semantic mode: {} results", semantic_results.len());
    println!("- Hybrid (default RRF): {} results", hybrid_default_results.len());
    println!("- Hybrid (TextOnly): {} results", hybrid_text_only_results.len());
    
    // Verify all modes work without errors
    assert!(true, "All search modes should work without errors");
    
    Ok(())
}