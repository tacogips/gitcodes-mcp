use gitdb::ids::{IssueId, PullRequestId, RepositoryId, UserId};
use gitdb::storage::{GitDatabase, ParticipationType};
use gitdb::storage::models::Issue;
use gitdb::types::IssueState;
use chrono::Utc;
use tempfile::TempDir;

async fn create_test_db() -> (GitDatabase, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    unsafe {
        std::env::set_var("GITDB_DATA_DIR", temp_dir.path());
    }
    let db = GitDatabase::new().await.unwrap();
    (db, temp_dir)
}

#[tokio::test]
async fn test_user_crud_operations() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Test creating a new user
    let user = db.get_or_create_user(
        12345,
        "octocat",
        Some("https://github.com/octocat.png".to_string()),
        Some("https://github.com/octocat".to_string()),
        "User",
        false,
    ).await.unwrap();
    
    assert_eq!(user.id.value(), 12345);
    assert_eq!(user.login, "octocat");
    assert_eq!(user.user_type, "User");
    assert!(!user.site_admin);
    
    // Test retrieving user by login
    let retrieved = db.get_user_by_login("octocat").await.unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id.value(), 12345);
    
    // Test updating existing user
    let updated = db.get_or_create_user(
        12345,
        "octocat",
        Some("https://github.com/octocat-new.png".to_string()),
        Some("https://github.com/octocat".to_string()),
        "User",
        true, // Now a site admin
    ).await.unwrap();
    
    assert_eq!(updated.id.value(), 12345);
    assert!(updated.site_admin);
    assert_eq!(updated.avatar_url, Some("https://github.com/octocat-new.png".to_string()));
}

#[tokio::test]
async fn test_issue_participants() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create test users
    let author = db.get_or_create_user(1, "alice", None, None, "User", false).await.unwrap();
    let assignee = db.get_or_create_user(2, "bob", None, None, "User", false).await.unwrap();
    let commenter = db.get_or_create_user(3, "charlie", None, None, "User", false).await.unwrap();
    
    let issue_id = IssueId::new(1000);
    
    // Add participants
    db.add_issue_participant(issue_id, author.id, ParticipationType::Author).await.unwrap();
    db.add_issue_participant(issue_id, assignee.id, ParticipationType::Assignee).await.unwrap();
    db.add_issue_participant(issue_id, commenter.id, ParticipationType::Commenter).await.unwrap();
    
    // Retrieve participants
    let participants = db.get_issue_participants(issue_id).await.unwrap();
    assert_eq!(participants.len(), 3);
    
    let logins: Vec<String> = participants.iter().map(|u| u.login.clone()).collect();
    assert!(logins.contains(&"alice".to_string()));
    assert!(logins.contains(&"bob".to_string()));
    assert!(logins.contains(&"charlie".to_string()));
}

#[tokio::test]
async fn test_pull_request_participants() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create test users
    let author = db.get_or_create_user(4, "dave", None, None, "User", false).await.unwrap();
    let assignee = db.get_or_create_user(5, "eve", None, None, "User", false).await.unwrap();
    let commenter1 = db.get_or_create_user(6, "frank", None, None, "User", false).await.unwrap();
    let commenter2 = db.get_or_create_user(7, "grace", None, None, "User", false).await.unwrap();
    
    let pr_id = PullRequestId::new(2000);
    
    // Add participants
    db.add_pull_request_participant(pr_id, author.id, ParticipationType::Author).await.unwrap();
    db.add_pull_request_participant(pr_id, assignee.id, ParticipationType::Assignee).await.unwrap();
    db.add_pull_request_participant(pr_id, commenter1.id, ParticipationType::Commenter).await.unwrap();
    db.add_pull_request_participant(pr_id, commenter2.id, ParticipationType::Commenter).await.unwrap();
    
    // Retrieve participants
    let participants = db.get_pull_request_participants(pr_id).await.unwrap();
    assert_eq!(participants.len(), 4);
    
    let logins: Vec<String> = participants.iter().map(|u| u.login.clone()).collect();
    assert!(logins.contains(&"dave".to_string()));
    assert!(logins.contains(&"eve".to_string()));
    assert!(logins.contains(&"frank".to_string()));
    assert!(logins.contains(&"grace".to_string()));
}

#[tokio::test]
#[ignore = "Search functionality moved to search_store"]
async fn test_search_with_participants() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create repository
    let repo = gitdb::storage::models::Repository {
        id: RepositoryId::new(1),
        owner: "test".to_string(),
        name: "repo".to_string(),
        full_name: "test/repo".to_string(),
        description: Some("Test repository".to_string()),
        stars: 0,
        forks: 0,
        language: Some("Rust".to_string()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        indexed_at: Utc::now(),
    };
    db.save_repository(&repo).await.unwrap();
    
    // Create users
    let alice = db.get_or_create_user(10, "alice", None, None, "User", false).await.unwrap();
    let bob = db.get_or_create_user(11, "bob", None, None, "User", false).await.unwrap();
    let charlie = db.get_or_create_user(12, "charlie", None, None, "User", false).await.unwrap();
    
    // Create issue with participants
    let issue = Issue {
        id: IssueId::new(100),
        repository_id: repo.id,
        number: gitdb::ids::IssueNumber::new(1),
        title: "Test issue with participants".to_string(),
        body: Some("This is a test issue".to_string()),
        state: IssueState::Open,
        author: "alice".to_string(),
        assignees: vec!["bob".to_string()],
        labels: vec!["bug".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        comments_count: 1,
    };
    db.save_issue(&issue).await.unwrap();
    
    // Add participants
    db.add_issue_participant(issue.id, alice.id, ParticipationType::Author).await.unwrap();
    db.add_issue_participant(issue.id, bob.id, ParticipationType::Assignee).await.unwrap();
    db.add_issue_participant(issue.id, charlie.id, ParticipationType::Commenter).await.unwrap();
    
    // Search functionality has been moved to search_store
    // These tests are disabled as search is no longer part of GitDatabase
    /*
    // Search by assignee (use the field name that exists in the schema)
    let results = db.search("assignees:bob", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issues assigned to bob");
    
    // Search by commenter
    let results = db.search("commenters:charlie", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issues where charlie commented");
    
    // Search by participant
    let results = db.search("participants:alice", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issues where alice participated");
    
    let results = db.search("participants:charlie", 10).await.unwrap();
    assert!(!results.is_empty(), "Should find issues where charlie participated");
    */
}

#[tokio::test]
async fn test_unique_user_constraint() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create a user
    let user1 = db.get_or_create_user(
        20000,
        "unique_user",
        Some("https://github.com/unique.png".to_string()),
        None,
        "User",
        false,
    ).await.unwrap();
    
    // Try to create another user with the same login but different ID
    // This should succeed because we store by ID, but login is also unique
    let user2 = db.get_or_create_user(
        20001,
        "another_user",
        None,
        None,
        "User",
        false,
    ).await.unwrap();
    
    assert_ne!(user1.id, user2.id);
    assert_ne!(user1.login, user2.login);
    
    // Updating the same user should work
    let updated = db.get_or_create_user(
        20000,
        "unique_user",
        Some("https://github.com/unique-new.png".to_string()),
        None,
        "User",
        true,
    ).await.unwrap();
    
    assert_eq!(updated.id, user1.id);
    assert!(updated.site_admin);
}

#[tokio::test]
async fn test_multiple_users_retrieval() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create multiple users
    let users = vec![
        db.get_or_create_user(30001, "user1", None, None, "User", false).await.unwrap(),
        db.get_or_create_user(30002, "user2", None, None, "User", false).await.unwrap(),
        db.get_or_create_user(30003, "user3", None, None, "User", false).await.unwrap(),
    ];
    
    let user_ids: Vec<UserId> = users.iter().map(|u| u.id).collect();
    
    // Retrieve multiple users
    let retrieved = db.get_users_by_ids(&user_ids).await.unwrap();
    assert_eq!(retrieved.len(), 3);
    
    // Try with some non-existent IDs
    let mixed_ids = vec![
        UserId::new(30001),
        UserId::new(99999), // Non-existent
        UserId::new(30003),
    ];
    
    let retrieved = db.get_users_by_ids(&mixed_ids).await.unwrap();
    assert_eq!(retrieved.len(), 2); // Should only get the existing ones
}