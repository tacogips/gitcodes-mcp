use chrono::{DateTime, Utc};
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

use crate::ids::{
    CommentId, IssueId, IssueNumber, ProjectId, ProjectNumber, PullRequestId, PullRequestNumber,
    RepositoryId, SyncStatusId, UserId,
};
use crate::types::{IssueState, ItemType, PullRequestState, ResourceType, SyncStatusType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 1, version = 1)]
#[native_db]
pub struct Repository {
    #[primary_key]
    pub id: RepositoryId,
    pub owner: String,
    pub name: String,
    #[secondary_key(unique)]
    pub full_name: String,
    pub description: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub indexed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 2, version = 1)]
#[native_db]
pub struct Issue {
    #[primary_key]
    pub id: IssueId,
    #[secondary_key]
    pub repository_id: RepositoryId,
    pub number: IssueNumber,
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
    // Projects this issue belongs to (populated by project sync)
    pub project_ids: Vec<ProjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 3, version = 1)]
#[native_db]
pub struct PullRequest {
    #[primary_key]
    pub id: PullRequestId,
    #[secondary_key]
    pub repository_id: RepositoryId,
    pub number: PullRequestNumber,
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
    // Projects this PR belongs to (populated by project sync)
    pub project_ids: Vec<ProjectId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 4, version = 1)]
#[native_db]
pub struct IssueComment {
    #[primary_key]
    pub id: CommentId,
    #[secondary_key]
    pub issue_id: IssueId,
    pub comment_id: CommentId,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 5, version = 1)]
#[native_db]
pub struct PullRequestComment {
    #[primary_key]
    pub id: CommentId,
    #[secondary_key]
    pub pull_request_id: PullRequestId,
    pub comment_id: CommentId,
    pub author: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 6, version = 1)]
#[native_db]
pub struct SyncStatus {
    #[primary_key]
    pub id: SyncStatusId,
    #[secondary_key]
    pub repository_id: RepositoryId,
    pub resource_type: ResourceType,
    pub last_synced_at: DateTime<Utc>,
    pub status: SyncStatusType,
    pub error_message: Option<String>,
    pub items_synced: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 7, version = 1)]
#[native_db]
pub struct CrossReference {
    pub source_type: ItemType,
    #[primary_key]
    pub source_id: i64,
    #[secondary_key]
    pub source_repository_id: RepositoryId,
    pub target_type: ItemType,
    #[secondary_key]
    pub target_repository_id: RepositoryId,
    pub target_number: i64,
    pub link_text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 8, version = 1)]
#[native_db]
pub struct User {
    #[primary_key]
    pub id: UserId,
    #[secondary_key(unique)]
    pub login: String,
    pub avatar_url: Option<String>,
    pub html_url: Option<String>,
    pub user_type: String,
    pub site_admin: bool,
    pub first_seen_at: DateTime<Utc>,
    pub last_updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 9, version = 1)]
#[native_db]
pub struct IssueParticipant {
    #[primary_key]
    pub id: String, // Composite key: "{issue_id}:{user_id}"
    #[secondary_key]
    pub issue_id: IssueId,
    #[secondary_key]
    pub user_id: UserId,
    pub participation_type: ParticipationType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 10, version = 1)]
#[native_db]
pub struct PullRequestParticipant {
    #[primary_key]
    pub id: String, // Composite key: "{pr_id}:{user_id}"
    #[secondary_key]
    pub pull_request_id: PullRequestId,
    #[secondary_key]
    pub user_id: UserId,
    pub participation_type: ParticipationType,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipationType {
    Author,
    Assignee,
    Commenter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 11, version = 1)]
#[native_db]
pub struct Project {
    #[primary_key]
    pub id: ProjectId,
    pub owner: String, // Organization or user login
    pub number: ProjectNumber,
    pub title: String,
    pub description: Option<String>,
    pub state: String, // OPEN, CLOSED
    pub visibility: String, // PUBLIC, PRIVATE
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub indexed_at: DateTime<Utc>,
    // Track which repositories this project can include items from
    pub linked_repositories: Vec<RepositoryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 12, version = 1)]
#[native_db]
pub struct ProjectItem {
    #[primary_key]
    pub id: String, // Composite: "{project_id}:{item_type}:{item_id}"
    #[secondary_key]
    pub project_id: ProjectId,
    pub item_type: ItemType, // Issue or PullRequest
    pub item_id: i64, // IssueId or PullRequestId value
    #[secondary_key]
    pub repository_id: RepositoryId, // The repository this item belongs to
    pub position: Option<f64>, // Position in the project board
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}