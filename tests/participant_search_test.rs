use gitdb::ids::{IssueId, RepositoryId};
use gitdb::storage::{GitDatabase, ParticipationType};
use gitdb::storage::models::{Issue, Repository};
use gitdb::types::IssueState;
use chrono::Utc;
use tempfile::TempDir;
use std::time::Duration;
use tokio::time::sleep;

async fn create_test_db() -> (GitDatabase, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("GITDB_DATA_DIR", temp_dir.path());
    }
    let db = GitDatabase::new().await.unwrap();
    (db, temp_dir)
}

#[tokio::test]
async fn test_search_basics() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create repository
    let repo = Repository {
        id: RepositoryId::new(1),
        owner: "test".to_string(),
        name: "repo".to_string(),
        full_name: "test/repo".to_string(),
        description: Some("Test repository".to_string()),
        stars: 100,
        forks: 20,
        language: Some("Rust".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.save_repository(&repo).await.unwrap();
    
    // Create a simple issue
    let issue = Issue {
        id: IssueId::new(1),
        repository_id: repo.id,
        number: gitdb::ids::IssueNumber::new(1),
        title: "Test search functionality".to_string(),
        body: Some("This issue tests the search feature".to_string()),
        state: IssueState::Open,
        author: "testauthor".to_string(),
        assignees: vec!["testassignee".to_string()],
        labels: vec!["bug".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        comments_count: 0,
    };
    
    db.save_issue(&issue).await.unwrap();
    
    // Give the index time to update (Tantivy has async indexing)
    sleep(Duration::from_millis(100)).await;
    
    // Search by title keyword
    let results = db.search("search", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issue by title keyword");
    
    // Search by body content
    let results = db.search("feature", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issue by body content");
    
    // Search by author
    let results = db.search("testauthor", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issue by author");
}

#[tokio::test]
async fn test_participant_search_after_indexing() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create repository
    let repo = Repository {
        id: RepositoryId::new(10),
        owner: "test".to_string(),
        name: "participant-repo".to_string(),
        full_name: "test/participant-repo".to_string(),
        description: Some("Repository with participants".to_string()),
        stars: 50,
        forks: 10,
        language: Some("Rust".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.save_repository(&repo).await.unwrap();
    
    // Create users first
    let alice = db.get_or_create_user(101, "alice", None, None, "User", false).await.unwrap();
    let bob = db.get_or_create_user(102, "bob", None, None, "User", false).await.unwrap();
    let charlie = db.get_or_create_user(103, "charlie", None, None, "User", false).await.unwrap();
    
    // Create issue with participants already set
    let issue_id = IssueId::new(100);
    
    // Add participants BEFORE creating the issue
    db.add_issue_participant(issue_id, alice.id, ParticipationType::Author).await.unwrap();
    db.add_issue_participant(issue_id, bob.id, ParticipationType::Assignee).await.unwrap();
    db.add_issue_participant(issue_id, charlie.id, ParticipationType::Commenter).await.unwrap();
    
    // Now create and save the issue
    let issue = Issue {
        id: issue_id,
        repository_id: repo.id,
        number: gitdb::ids::IssueNumber::new(10),
        title: "Issue with multiple participants".to_string(),
        body: Some("This issue has alice as author, bob as assignee, and charlie as commenter".to_string()),
        state: IssueState::Open,
        author: alice.login.clone(),
        assignees: vec![bob.login.clone()],
        labels: vec!["discussion".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        comments_count: 1,
    };
    
    // Save issue - this will trigger indexing which will fetch participants
    db.save_issue(&issue).await.unwrap();
    
    // Give the index time to update
    sleep(Duration::from_millis(200)).await;
    
    // Now search should work
    
    // Search by title to verify basic search works
    let results = db.search("multiple participants", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issue by title");
    
    // Search by participant field
    let results = db.search("participants:alice", 10).await.unwrap();
    println!("Search for participants:alice returned {} results", results.len());
    for result in &results {
        println!("  Found: {} - {}", result.id, result.title);
    }
    assert!(!results.is_empty(), "Should find issues where alice is a participant");
    
    let results = db.search("participants:charlie", 10).await.unwrap();
    println!("Search for participants:charlie returned {} results", results.len());
    assert!(!results.is_empty(), "Should find issues where charlie is a participant");
}

#[tokio::test]
async fn test_field_specific_searches() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create repository
    let repo = Repository {
        id: RepositoryId::new(20),
        owner: "fieldtest".to_string(),
        name: "repo".to_string(),
        full_name: "fieldtest/repo".to_string(),
        description: Some("Test field searches".to_string()),
        stars: 10,
        forks: 5,
        language: Some("Rust".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.save_repository(&repo).await.unwrap();
    
    // Create issue with specific fields to search
    let issue = Issue {
        id: IssueId::new(200),
        repository_id: repo.id,
        number: gitdb::ids::IssueNumber::new(20),
        title: "Specific field test".to_string(),
        body: Some("Testing field-specific searches".to_string()),
        state: IssueState::Open,
        author: "fieldauthor".to_string(),
        assignees: vec!["fieldassignee1".to_string(), "fieldassignee2".to_string()],
        labels: vec!["fieldlabel1".to_string(), "fieldlabel2".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        comments_count: 0,
    };
    
    db.save_issue(&issue).await.unwrap();
    
    // Give the index time to update
    sleep(Duration::from_millis(100)).await;
    
    // Test various field searches
    let results = db.search("title:specific", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find by title field");
    
    let results = db.search("author:fieldauthor", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find by author field");
    
    let results = db.search("assignees:fieldassignee1", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find by assignees field");
    
    let results = db.search("labels:fieldlabel2", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find by labels field");
}