use gitdb::types::*;
use std::str::FromStr;

#[test]
fn test_item_type_conversion() {
    // Test Display trait
    assert_eq!(ItemType::Issue.to_string(), "issue");
    assert_eq!(ItemType::PullRequest.to_string(), "pull_request");

    // Test FromStr trait
    assert_eq!(ItemType::from_str("issue").unwrap(), ItemType::Issue);
    assert_eq!(
        ItemType::from_str("pull_request").unwrap(),
        ItemType::PullRequest
    );

    // Test invalid conversion
    assert!(ItemType::from_str("invalid").is_err());
}

#[test]
fn test_issue_state_conversion() {
    // Test Display trait
    assert_eq!(IssueState::Open.to_string(), "open");
    assert_eq!(IssueState::Closed.to_string(), "closed");

    // Test FromStr trait
    assert_eq!(IssueState::from_str("open").unwrap(), IssueState::Open);
    assert_eq!(IssueState::from_str("closed").unwrap(), IssueState::Closed);
}

#[test]
fn test_pull_request_state_conversion() {
    // Test Display trait
    assert_eq!(PullRequestState::Open.to_string(), "open");
    assert_eq!(PullRequestState::Closed.to_string(), "closed");
    assert_eq!(PullRequestState::Merged.to_string(), "merged");

    // Test FromStr trait
    assert_eq!(
        PullRequestState::from_str("open").unwrap(),
        PullRequestState::Open
    );
    assert_eq!(
        PullRequestState::from_str("closed").unwrap(),
        PullRequestState::Closed
    );
    assert_eq!(
        PullRequestState::from_str("merged").unwrap(),
        PullRequestState::Merged
    );
}

#[test]
fn test_resource_type_conversion() {
    // Test Display trait
    assert_eq!(ResourceType::Issues.to_string(), "issues");
    assert_eq!(ResourceType::PullRequests.to_string(), "pull_requests");

    // Test FromStr trait
    assert_eq!(
        ResourceType::from_str("issues").unwrap(),
        ResourceType::Issues
    );
    assert_eq!(
        ResourceType::from_str("pull_requests").unwrap(),
        ResourceType::PullRequests
    );
}

#[test]
fn test_sync_status_type_conversion() {
    // Test Display trait
    assert_eq!(SyncStatusType::Success.to_string(), "success");
    assert_eq!(SyncStatusType::Failed.to_string(), "failed");

    // Test FromStr trait
    assert_eq!(
        SyncStatusType::from_str("success").unwrap(),
        SyncStatusType::Success
    );
    assert_eq!(
        SyncStatusType::from_str("failed").unwrap(),
        SyncStatusType::Failed
    );
}

#[test]
fn test_provider_conversion() {
    // Test Display trait
    assert_eq!(Provider::Github.to_string(), "github");

    // Test FromStr trait
    assert_eq!(Provider::from_str("github").unwrap(), Provider::Github);
}

#[test]
fn test_serialization() {
    // Test JSON serialization
    let item_type = ItemType::Issue;
    let json = serde_json::to_string(&item_type).unwrap();
    assert_eq!(json, "\"issue\"");

    let deserialized: ItemType = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, ItemType::Issue);

    // Test PullRequestState serialization
    let pr_state = PullRequestState::Merged;
    let json = serde_json::to_string(&pr_state).unwrap();
    assert_eq!(json, "\"merged\"");

    let deserialized: PullRequestState = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, PullRequestState::Merged);
}