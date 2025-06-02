use gitdb::ids::*;

#[test]
fn test_repository_id() {
    let id = RepositoryId::new(123);
    assert_eq!(id.value(), 123);
    assert_eq!(id.to_string(), "123");

    // Test From trait
    let id2: RepositoryId = 456.into();
    assert_eq!(id2.value(), 456);

    // Test equality
    assert_eq!(RepositoryId::new(789), RepositoryId::new(789));
    assert_ne!(RepositoryId::new(789), RepositoryId::new(790));
}

#[test]
fn test_issue_id() {
    let id = IssueId::new(1000);
    assert_eq!(id.value(), 1000);
    assert_eq!(id.to_string(), "1000");

    // Test serialization
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, "1000");

    let deserialized: IssueId = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, id);
}

#[test]
fn test_pull_request_id() {
    let id = PullRequestId::new(2000);
    assert_eq!(id.value(), 2000);

    // Test that different ID types are not interchangeable
    let _issue_id = IssueId::new(2000);
    let _pr_id = PullRequestId::new(2000);

    // This would not compile:
    // let _: IssueId = pr_id; // Error: mismatched types
}

#[test]
fn test_comment_id() {
    let id = CommentId::new(3000);
    assert_eq!(id.value(), 3000);
    assert_eq!(id.to_string(), "3000");
}

#[test]
fn test_issue_number() {
    let num = IssueNumber::new(123);
    assert_eq!(num.value(), 123);
    assert_eq!(num.to_string(), "123");
}

#[test]
fn test_pull_request_number() {
    let num = PullRequestNumber::new(456);
    assert_eq!(num.value(), 456);
    assert_eq!(num.to_string(), "456");
}

#[test]
fn test_sync_status_id() {
    let id = SyncStatusId::new(999);
    assert_eq!(id.value(), 999);
    assert_eq!(id.to_string(), "999");
}

#[test]
fn test_id_hashing() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(RepositoryId::new(1));
    set.insert(RepositoryId::new(2));
    set.insert(RepositoryId::new(1)); // Duplicate

    assert_eq!(set.len(), 2); // Should only have 2 unique IDs
    assert!(set.contains(&RepositoryId::new(1)));
    assert!(set.contains(&RepositoryId::new(2)));
}

#[test]
fn test_id_cannot_be_manipulated() {
    let id1 = RepositoryId::new(10);
    let id2 = RepositoryId::new(20);

    // These would not compile - IDs cannot be added, subtracted, etc.
    // let sum = id1 + id2; // Error: cannot add
    // let diff = id2 - id1; // Error: cannot subtract
    // let product = id1 * 2; // Error: cannot multiply

    // The only way to work with the values is to extract them explicitly
    let sum = id1.value() + id2.value();
    assert_eq!(sum, 30);
}