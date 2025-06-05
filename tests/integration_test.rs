use anyhow::Result;
use gitdb::storage::{GitDatabase, SearchQuery};
use tempfile::TempDir;

#[tokio::test]
async fn test_gitdatabase_has_search_store() -> Result<()> {
    // Set up temporary directory for test
    let temp_dir = TempDir::new()?;
    unsafe {
        std::env::set_var("GITDB_DATA_DIR", temp_dir.path());
    }
    
    // Create GitDatabase which should initialize SearchStore
    let db = GitDatabase::new().await?;
    
    // Verify we can access the search store
    let _search_store = db.search_store();
    // SearchStore exists if we get here without panic
    
    // Test the search functionality (even if it returns empty results)
    let query = SearchQuery {
        text: "test".to_string(),
        repository: None,
        state: None,
        label: None,
        limit: Some(10),
        offset: None,
        filter: None,
    };
    
    // Search may fail if no FTS index exists, which is expected for an empty database
    match db.search(query).await {
        Ok(results) => assert_eq!(results.len(), 0), // Expecting empty results
        Err(e) if e.to_string().contains("no inverted index") => {
            // This is expected - no FTS index exists yet
        }
        Err(e) => return Err(e), // Other errors should propagate
    }
    
    Ok(())
}

#[tokio::test]
async fn test_gitdatabase_search_methods() -> Result<()> {
    let temp_dir = TempDir::new()?;
    unsafe {
        std::env::set_var("GITDB_DATA_DIR", temp_dir.path());
    }
    
    let db = GitDatabase::new().await?;
    
    // Test search_repositories
    let query = gitdb::storage::search_store::LanceDbQuery::new("rust");
    match db.search_repositories(&query).await {
        Ok(repos) => assert_eq!(repos.len(), 0),
        Err(e) if e.to_string().contains("no inverted index") => {
            // Expected - no FTS index exists yet
        }
        Err(e) => return Err(e),
    }
    
    // Test search_issues
    match db.search_issues(&query).await {
        Ok(issues) => assert_eq!(issues.len(), 0),
        Err(e) if e.to_string().contains("no inverted index") => {
            // Expected - no FTS index exists yet
        }
        Err(e) => return Err(e),
    }
    
    Ok(())
}