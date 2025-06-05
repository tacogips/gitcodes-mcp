use gitdb::ids::{IssueId, PullRequestId, RepositoryId, UserId};
use gitdb::storage::{GitDatabase, ParticipationType};
use gitdb::storage::models::{Issue, PullRequest, Repository};
use gitdb::types::{IssueState, PullRequestState};
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
async fn test_participant_workflow() {
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
    
    // Create users
    let alice = db.get_or_create_user(1001, "alice", 
        Some("https://github.com/alice.png".to_string()),
        Some("https://github.com/alice".to_string()),
        "User", false).await.unwrap();
    
    let bob = db.get_or_create_user(1002, "bob", 
        Some("https://github.com/bob.png".to_string()),
        Some("https://github.com/bob".to_string()),
        "User", false).await.unwrap();
    
    let charlie = db.get_or_create_user(1003, "charlie", 
        Some("https://github.com/charlie.png".to_string()),
        Some("https://github.com/charlie".to_string()),
        "User", false).await.unwrap();
    
    // Create issue
    let issue = Issue {
        id: IssueId::new(2001),
        repository_id: repo.id,
        number: gitdb::ids::IssueNumber::new(1),
        title: "Test issue".to_string(),
        body: Some("This is a test issue body".to_string()),
        state: IssueState::Open,
        author: alice.login.clone(),
        assignees: vec![bob.login.clone()],
        labels: vec!["bug".to_string(), "help wanted".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        closed_at: None,
        comments_count: 1,
        project_ids: vec![],
    };
    
    // Save issue
    db.save_issue(&issue).await.unwrap();
    
    // Add participants
    db.add_issue_participant(issue.id, alice.id, ParticipationType::Author).await.unwrap();
    db.add_issue_participant(issue.id, bob.id, ParticipationType::Assignee).await.unwrap();
    db.add_issue_participant(issue.id, charlie.id, ParticipationType::Commenter).await.unwrap();
    
    // Verify participants were added
    let participants = db.get_issue_participants(issue.id).await.unwrap();
    assert_eq!(participants.len(), 3);
    
    let participant_logins: Vec<String> = participants.iter()
        .map(|u| u.login.clone())
        .collect();
    assert!(participant_logins.contains(&"alice".to_string()));
    assert!(participant_logins.contains(&"bob".to_string()));
    assert!(participant_logins.contains(&"charlie".to_string()));
    
    // Create PR with multiple assignees and commenters
    let pr = PullRequest {
        id: PullRequestId::new(3001),
        repository_id: repo.id,
        number: gitdb::ids::PullRequestNumber::new(1),
        title: "Test PR".to_string(),
        body: Some("This is a test PR body".to_string()),
        state: PullRequestState::Open,
        author: alice.login.clone(),
        assignees: vec![bob.login.clone(), charlie.login.clone()],
        labels: vec!["enhancement".to_string()],
        head_ref: "feature-branch".to_string(),
        base_ref: "main".to_string(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
        merged_at: None,
        closed_at: None,
        comments_count: 2,
        commits_count: 3,
        additions: 100,
        deletions: 50,
        changed_files: 5,
        project_ids: vec![],
    };
    
    // Save PR
    db.save_pull_request(&pr).await.unwrap();
    
    // Add PR participants
    db.add_pull_request_participant(pr.id, alice.id, ParticipationType::Author).await.unwrap();
    db.add_pull_request_participant(pr.id, bob.id, ParticipationType::Assignee).await.unwrap();
    db.add_pull_request_participant(pr.id, charlie.id, ParticipationType::Assignee).await.unwrap();
    
    // Add another user as commenter
    let dave = db.get_or_create_user(1004, "dave", None, None, "User", false).await.unwrap();
    db.add_pull_request_participant(pr.id, dave.id, ParticipationType::Commenter).await.unwrap();
    
    // Verify PR participants
    let pr_participants = db.get_pull_request_participants(pr.id).await.unwrap();
    assert_eq!(pr_participants.len(), 4);
    
    let pr_participant_logins: Vec<String> = pr_participants.iter()
        .map(|u| u.login.clone())
        .collect();
    assert!(pr_participant_logins.contains(&"alice".to_string()));
    assert!(pr_participant_logins.contains(&"bob".to_string()));
    assert!(pr_participant_logins.contains(&"charlie".to_string()));
    assert!(pr_participant_logins.contains(&"dave".to_string()));
}

#[tokio::test]
async fn test_user_updates() {
    let (db, _temp_dir) = create_test_db().await;
    
    // Create user
    let user = db.get_or_create_user(5001, "testuser", 
        Some("https://old-avatar.png".to_string()),
        None,
        "User", false).await.unwrap();
    
    assert_eq!(user.avatar_url, Some("https://old-avatar.png".to_string()));
    assert!(!user.site_admin);
    
    // Update user with new data
    let updated = db.get_or_create_user(5001, "testuser",
        Some("https://new-avatar.png".to_string()),
        Some("https://github.com/testuser".to_string()),
        "User", true).await.unwrap();
    
    assert_eq!(updated.id, user.id);
    assert_eq!(updated.avatar_url, Some("https://new-avatar.png".to_string()));
    assert_eq!(updated.html_url, Some("https://github.com/testuser".to_string()));
    assert!(updated.site_admin);
    assert!(updated.last_updated_at > user.last_updated_at);
}

#[tokio::test]
async fn test_duplicate_participants() {
    let (db, _temp_dir) = create_test_db().await;
    
    let user = db.get_or_create_user(6001, "dupuser", None, None, "User", false).await.unwrap();
    let issue_id = IssueId::new(6002);
    
    // Add same user as both author and assignee
    db.add_issue_participant(issue_id, user.id, ParticipationType::Author).await.unwrap();
    
    // This should update the existing participant record (composite key prevents duplicates)
    let result = db.add_issue_participant(issue_id, user.id, ParticipationType::Assignee).await;
    
    // The exact behavior depends on native_db's handling of composite keys
    // but we should still only get one participant entry for this user
    let participants = db.get_issue_participants(issue_id).await.unwrap();
    
    // Should have exactly one participant (no duplicates)
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0].id, user.id);
}