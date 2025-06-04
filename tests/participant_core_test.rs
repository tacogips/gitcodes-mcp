use gitdb::ids::{IssueId, PullRequestId, RepositoryId};
use gitdb::storage::{GitDatabase, ParticipationType};
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
async fn test_core_participant_functionality() {
    let (db, _temp_dir) = create_test_db().await;
    
    println!("Creating users...");
    
    // Create users
    let alice = db.get_or_create_user(1, "alice", None, None, "User", false).await.unwrap();
    let bob = db.get_or_create_user(2, "bob", None, None, "User", false).await.unwrap();
    let charlie = db.get_or_create_user(3, "charlie", None, None, "User", false).await.unwrap();
    
    println!("Created users: alice({}), bob({}), charlie({})", alice.id, bob.id, charlie.id);
    
    // Test issue participants
    let issue_id = IssueId::new(100);
    
    println!("Adding issue participants...");
    
    db.add_issue_participant(issue_id, alice.id, ParticipationType::Author).await.unwrap();
    db.add_issue_participant(issue_id, bob.id, ParticipationType::Assignee).await.unwrap();
    db.add_issue_participant(issue_id, charlie.id, ParticipationType::Commenter).await.unwrap();
    
    println!("Retrieving issue participants...");
    
    let participants = db.get_issue_participants(issue_id).await.unwrap();
    
    println!("Found {} issue participants:", participants.len());
    for p in &participants {
        println!("  - {} (ID: {})", p.login, p.id);
    }
    
    assert_eq!(participants.len(), 3);
    
    // Test PR participants
    let pr_id = PullRequestId::new(200);
    
    println!("\nAdding PR participants...");
    
    db.add_pull_request_participant(pr_id, alice.id, ParticipationType::Author).await.unwrap();
    db.add_pull_request_participant(pr_id, bob.id, ParticipationType::Assignee).await.unwrap();
    db.add_pull_request_participant(pr_id, charlie.id, ParticipationType::Commenter).await.unwrap();
    
    println!("Retrieving PR participants...");
    
    let pr_participants = db.get_pull_request_participants(pr_id).await.unwrap();
    
    println!("Found {} PR participants:", pr_participants.len());
    for p in &pr_participants {
        println!("  - {} (ID: {})", p.login, p.id);
    }
    
    assert_eq!(pr_participants.len(), 3);
    
    // Test user retrieval
    println!("\nTesting user retrieval by login...");
    
    let found_alice = db.get_user_by_login("alice").await.unwrap();
    assert!(found_alice.is_some());
    assert_eq!(found_alice.unwrap().id, alice.id);
    
    let found_none = db.get_user_by_login("nonexistent").await.unwrap();
    assert!(found_none.is_none());
    
    println!("All core participant functionality tests passed!");
}