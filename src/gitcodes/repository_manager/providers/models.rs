//! Common domain models for Git providers
//!
//! This module contains domain models that are common across different Git providers.
//! These models provide a vendor-agnostic representation of Git repository data,
//! allowing the rest of the application to work with a consistent data model
//! regardless of which Git provider is being used.

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};
use url::Url;

/// Git Provider enum
///
/// Enumerates the supported Git providers for repository search.
/// Currently only GitHub is supported, but this allows for future expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display, EnumString, AsRefStr)]
#[strum(serialize_all = "lowercase")]
pub enum GitProvider {
    /// GitHub.com
    #[strum(serialize = "github")]
    Github,
}

/// Common domain model for repository search results
///
/// This struct provides a vendor-agnostic representation of repository search results.
/// It can be used with any Git provider and converted to/from provider-specific formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySearchResults {
    /// Total number of matching repositories
    pub total_count: u64,

    /// Indicates if the result was incomplete due to rate limiting
    pub incomplete_results: bool,

    /// List of repository items
    pub items: Vec<RepositoryItem>,
}

/// A repository item in search results
///
/// Represents a single repository in search results, with common properties
/// that are available across all Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryItem {
    /// Repository ID (unique within the provider)
    pub id: String,

    /// Repository name (without owner)
    pub name: String,

    /// Whether the repository is private
    pub private: Option<bool>,

    /// Repository owner information
    pub owner: RepositoryOwner,

    /// URL for viewing the repository in a browser
    pub html_url: Option<Url>,

    /// Repository description
    pub description: Option<String>,

    /// Whether the repository is a fork
    pub fork: Option<bool>,

    /// URL for repository homepage or documentation
    pub homepage: Option<String>,

    /// Size of the repository in kilobytes
    pub size: Option<u64>,

    /// Number of stargazers (stars)
    pub stargazers_count: Option<u64>,

    /// Number of watchers
    pub watchers_count: Option<u64>,

    /// Primary programming language
    pub language: Option<String>,

    /// Number of forks
    pub forks_count: Option<u64>,

    /// Whether the repository is archived
    pub archived: Option<bool>,

    /// Number of open issues
    pub open_issues_count: Option<u64>,

    /// Repository license information
    pub license: Option<RepositoryLicense>,

    /// Repository topics/tags
    pub topics: Option<Vec<String>>,

    /// Default branch name
    pub default_branch: Option<String>,

    /// Score (relevance to search query)
    /// Only available when using REST API, None for GraphQL searches
    pub score: Option<f64>,

    /// When the repository was created
    pub created_at: Option<String>,

    /// When the repository was last updated
    pub updated_at: Option<String>,

    /// When the repository was last pushed to
    pub pushed_at: Option<String>,
}

/// Repository owner information
///
/// Common representation of a repository owner that works across Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOwner {
    /// Owner's ID (as a string to support different ID formats)
    pub id: Option<String>,

    /// Owner type (User or Organization)
    pub type_field: Option<String>,
}

/// Repository license information
///
/// Common representation of a repository license that works across Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryLicense {
    /// License key/identifier
    pub key: String,

    /// License name
    pub name: String,
}

/// Common domain model for repository references
///
/// This struct provides a vendor-agnostic representation of repository references.
/// It separates branches and tags into separate collections for easier consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRefs {
    /// List of branch references
    pub branches: Vec<ReferenceInfo>,

    /// List of tag references
    pub tags: Vec<ReferenceInfo>,
}

/// Information about a git reference (branch or tag)
///
/// Common representation of a git reference that works across Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceInfo {
    /// Reference name (branch or tag name without path prefix)
    pub name: String,

    /// Full reference path (e.g., "refs/heads/main")
    pub full_ref: String,

    /// Commit SHA this reference points to
    pub commit_id: String,
}

/// Common domain model for issue search results
///
/// This struct provides a vendor-agnostic representation of issue search results.
/// It can be used with any Git provider and converted to/from provider-specific formats.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSearchResults {
    /// Total number of matching issues
    pub total_count: u64,

    /// Indicates if the result was incomplete due to rate limiting
    pub incomplete_results: bool,

    /// List of issue items
    pub items: Vec<IssueItem>,
}

/// An issue item in search results
///
/// Represents a single issue in search results, with common properties
/// that are available across all Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueItem {
    /// Issue ID (unique within the provider)
    pub id: String,

    /// Issue number within the repository
    pub number: u64,

    /// Issue title
    pub title: String,

    /// Issue body/description
    pub body: Option<String>,

    /// Current state of the issue (open, closed)
    pub state: String,

    /// Issue author information
    pub user: IssueUser,

    /// Issue assignee information
    pub assignee: Option<IssueUser>,

    /// List of assignees for the issue
    pub assignees: Vec<IssueUser>,

    /// List of labels attached to the issue
    pub labels: Vec<IssueLabel>,

    /// Number of comments on the issue
    pub comments: u32,

    /// URL for viewing the issue in a browser
    pub html_url: String,

    /// When the issue was created
    pub created_at: String,

    /// When the issue was last updated
    pub updated_at: String,

    /// When the issue was closed (if applicable)
    pub closed_at: Option<String>,

    /// Score (relevance to search query)
    /// Only available when using REST API, None for GraphQL searches
    pub score: Option<f64>,

    /// Repository information where the issue exists
    pub repository: IssueRepository,
}

/// Issue user information
///
/// Common representation of an issue user that works across Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueUser {
    /// User's login name
    pub login: String,

    /// User's ID (as a string to support different ID formats)
    pub id: String,
}

/// Issue label information
///
/// Common representation of an issue label that works across Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueLabel {
    /// Label ID
    pub id: String,

    /// Label name
    pub name: String,

    /// Label color (hex color code)
    pub color: String,

    /// Label description
    pub description: Option<String>,
}

/// Issue milestone information
///
/// Common representation of an issue milestone that works across Git providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueMilestone {
    /// Milestone ID
    pub id: String,

    /// Milestone number
    pub number: u64,

    /// Milestone title
    pub title: String,

    /// Milestone description
    pub description: Option<String>,

    /// Current state of the milestone (open, closed)
    pub state: String,

    /// When the milestone was created
    pub created_at: String,

    /// When the milestone was last updated
    pub updated_at: String,

    /// Due date for the milestone
    pub due_on: Option<String>,

    /// When the milestone was closed (if applicable)
    pub closed_at: Option<String>,
}

/// Repository information for issues
///
/// Common representation of repository information in issue search results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueRepository {
    /// Repository ID
    pub id: String,

    /// Repository name (without owner)
    pub name: String,

    /// Repository owner information
    pub owner: RepositoryOwner,

    /// Whether the repository is private
    pub private: bool,

    /// URL for viewing the repository in a browser
    pub html_url: String,

    /// Repository description
    pub description: Option<String>,
}
