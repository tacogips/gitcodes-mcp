use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    Issue,
    PullRequest,
}

impl ItemType {
    /// Returns the string representation for unknown item types.
    ///
    /// # Returns
    ///
    /// A static string slice containing "unknown"
    pub fn unknown_str() -> &'static str {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum IssueState {
    Open,
    Closed,
}

impl IssueState {
    /// Returns the string representation for unknown issue states.
    ///
    /// # Returns
    ///
    /// A static string slice containing "unknown"
    pub fn unknown_str() -> &'static str {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
}

impl PullRequestState {
    /// Returns the string representation for unknown pull request states.
    ///
    /// # Returns
    ///
    /// A static string slice containing "unknown"
    pub fn unknown_str() -> &'static str {
        "unknown"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    Issues,
    PullRequests,
    Projects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum SyncStatusType {
    Success,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Github,
}

/// A strongly-typed repository name (format: owner/repo)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RepositoryName(String);

impl RepositoryName {
    /// Creates a new RepositoryName from a string
    /// 
    /// # Arguments
    /// 
    /// * `name` - The repository name in "owner/repo" format
    /// 
    /// # Returns
    /// 
    /// Returns Ok(RepositoryName) if the format is valid, otherwise returns an error
    pub fn new(name: impl Into<String>) -> Result<Self, String> {
        let name = name.into();
        if name.split('/').count() == 2 && !name.contains(' ') && !name.is_empty() {
            Ok(RepositoryName(name))
        } else {
            Err(format!("Invalid repository name format: '{}'. Expected format: 'owner/repo'", name))
        }
    }
    
    /// Returns the repository name as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
    
    /// Consumes self and returns the inner String
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::fmt::Display for RepositoryName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for RepositoryName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for RepositoryName {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        RepositoryName::new(s)
    }
}

// GitHub API Types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub id: u64,
    pub owner: String,
    pub name: String,
    pub full_name: String,
    pub description: Option<String>,
    pub url: String,
    pub clone_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub language: Option<String>,
    pub fork: bool,
    pub forks_count: u32,
    pub stargazers_count: u32,
    pub open_issues_count: u32,
    pub is_template: Option<bool>,
    pub topics: Vec<String>,
    pub visibility: String,
    pub default_branch: String,
    pub permissions: Option<serde_json::Value>,
    pub license: Option<GitHubLicense>,
    pub archived: bool,
    pub disabled: bool,
}

impl GitHubRepository {
    pub fn full_id(&self) -> FullId {
        FullId::Repository(self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    pub id: u64,
    pub repository_id: FullId,
    pub number: i32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: GitHubUser,
    pub assignees: Vec<GitHubUser>,
    pub labels: Vec<GitHubLabel>,
    pub milestone: Option<GitHubMilestone>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
}

impl GitHubIssue {
    pub fn full_id(&self) -> FullId {
        FullId::Issue(self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequest {
    pub id: u64,
    pub repository_id: FullId,
    pub number: i32,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub user: GitHubUser,
    pub assignees: Vec<GitHubUser>,
    pub labels: Vec<GitHubLabel>,
    pub milestone: Option<GitHubMilestone>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub merged_at: Option<DateTime<Utc>>,
    pub head: GitHubBranch,
    pub base: GitHubBranch,
    pub draft: bool,
}

impl GitHubPullRequest {
    pub fn full_id(&self) -> FullId {
        FullId::PullRequest(self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubComment {
    pub id: u64,
    pub issue_id: FullId,
    pub user: GitHubUser,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl GitHubComment {
    pub fn full_id(&self) -> FullId {
        FullId::Comment(self.id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubUser {
    pub id: i64,
    pub login: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub html_url: String,
    pub bio: Option<String>,
    pub company: Option<String>,
    pub location: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub public_repos: i32,
    pub followers: i32,
    pub following: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLabel {
    pub id: u64,
    pub name: String,
    pub color: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubMilestone {
    pub id: u64,
    pub number: i32,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubBranch {
    pub ref_: String,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubLicense {
    pub key: String,
    pub name: String,
    pub spdx_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPullRequestFile {
    pub sha: String,
    pub filename: String,
    pub status: String,
    pub additions: i32,
    pub deletions: i32,
    pub changes: i32,
    pub patch: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FullId {
    Repository(u64),
    Issue(u64),
    PullRequest(u64),
    Comment(u64),
    User(i64),
}

impl std::fmt::Display for FullId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FullId::Repository(id) => write!(f, "repo:{}", id),
            FullId::Issue(id) => write!(f, "issue:{}", id),
            FullId::PullRequest(id) => write!(f, "pr:{}", id),
            FullId::Comment(id) => write!(f, "comment:{}", id),
            FullId::User(id) => write!(f, "user:{}", id),
        }
    }
}
