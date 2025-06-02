use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::types::{IssueState, ItemType, PullRequestState, ResourceType, SyncStatusType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    pub id: i64,
    pub repository_id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: IssueState,
    pub author: String,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub comments_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    pub id: i64,
    pub repository_id: i64,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: PullRequestState,
    pub author: String,
    pub assignees: Vec<String>,
    pub labels: Vec<String>,
    pub head_ref: String,
    pub base_ref: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
    pub comments_count: i64,
    pub commits_count: i64,
    pub additions: i64,
    pub deletions: i64,
    pub changed_files: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueComment {
    pub id: i64,
    pub issue_id: i64,
    pub comment_id: i64,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestComment {
    pub id: i64,
    pub pull_request_id: i64,
    pub comment_id: i64,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub id: i64,
    pub repository_id: i64,
    pub resource_type: ResourceType,
    pub last_synced_at: DateTime<Utc>,
    pub status: SyncStatusType,
    pub error_message: Option<String>,
    pub items_synced: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossReference {
    pub source_type: ItemType,
    pub source_id: i64,
    pub source_repository_id: i64,
    pub target_type: ItemType,
    pub target_repository_id: i64,
    pub target_number: i64,
    pub link_text: String,
    pub created_at: DateTime<Utc>,
}