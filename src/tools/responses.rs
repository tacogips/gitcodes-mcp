//! Response types for the GitHub code tools MCP server
//!
//! This module defines structured response types that are returned by the
//! MCP tool methods. These types help ensure that the JSON responses are
//! consistently formatted and properly typed.

use crate::gitcodes::CodeSearchResult;
use lumin::view::FileContents;
use serde::{Deserialize, Serialize};

/// Response for the search_repositories tool
///
/// Wraps the raw GitHub API response in a structured format
/// with consistent field names and types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositorySearchResponse {
    /// Total number of matching repositories
    pub total_count: u64,
    
    /// Indicates if the result was incomplete due to rate limiting
    pub incomplete_results: bool,
    
    /// List of repository items
    pub items: Vec<RepositoryItem>,
}

/// A repository item in the search results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryItem {
    /// Unique numeric GitHub ID
    pub id: u64,
    
    /// Repository node ID (for GitHub GraphQL API)
    pub node_id: String,
    
    /// Repository name (without owner)
    pub name: String,
    
    /// Full repository name with owner (e.g., "owner/repo")
    pub full_name: String,
    
    /// Whether the repository is private
    pub private: bool,
    
    /// Repository owner information
    pub owner: RepositoryOwner,
    
    /// HTML URL for viewing the repository in a browser
    pub html_url: String,
    
    /// Repository description
    pub description: Option<String>,
    
    /// Whether the repository is a fork
    pub fork: bool,
    
    /// URL for repository homepage or documentation
    pub homepage: Option<String>,
    
    /// Size of the repository in kilobytes
    pub size: u64,
    
    /// Number of stargazers (stars)
    pub stargazers_count: u64,
    
    /// Number of watchers
    pub watchers_count: u64,
    
    /// Primary programming language
    pub language: Option<String>,
    
    /// Whether the repository has issues enabled
    pub has_issues: bool,
    
    /// Whether the repository has projects enabled
    pub has_projects: bool,
    
    /// Whether the repository has downloads enabled
    pub has_downloads: bool,
    
    /// Whether the repository has wiki enabled
    pub has_wiki: bool,
    
    /// Whether the repository has pages enabled
    pub has_pages: bool,
    
    /// Number of forks
    pub forks_count: u64,
    
    /// Whether the repository is archived
    pub archived: bool,
    
    /// Whether the repository is disabled
    pub disabled: bool,
    
    /// Number of open issues
    pub open_issues_count: u64,
    
    /// Repository license information
    pub license: Option<RepositoryLicense>,
    
    /// Whether the repository is a template
    pub is_template: Option<bool>,
    
    /// Repository topics
    pub topics: Option<Vec<String>>,
    
    /// Default branch name
    pub default_branch: String,
    
    /// Number of subscribers
    pub subscribers_count: Option<u64>,
    
    /// Score (relevance to search query)
    pub score: f64,
}

/// Repository owner information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOwner {
    /// Owner's login name
    pub login: String,
    
    /// Owner's ID
    pub id: u64,
    
    /// Owner's node ID
    pub node_id: String,
    
    /// URL to owner's avatar image
    pub avatar_url: String,
    
    /// URL to owner's GitHub profile
    pub html_url: String,
    
    /// Owner type (User or Organization)
    pub type_field: String,
    
    /// Whether the owner is a GitHub site admin
    pub site_admin: bool,
}

/// Repository license information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryLicense {
    /// License key (identifier)
    pub key: String,
    
    /// License name
    pub name: String,
    
    /// SPDX ID for the license
    pub spdx_id: Option<String>,
    
    /// URL to the license
    pub url: Option<String>,
    
    /// License node ID
    pub node_id: String,
}

/// Response for the grep_repository tool
///
/// This type directly uses the CodeSearchResult struct
/// from the gitcodes crate to ensure consistency.
pub type CodeSearchResponse = CodeSearchResult;

/// Response for the list_repository_refs tool
///
/// Contains lists of branches and tags available in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRefsResponse {
    /// List of branch references
    pub branches: Vec<ReferenceInfo>,
    
    /// List of tag references
    pub tags: Vec<ReferenceInfo>,
}

/// Information about a git reference (branch or tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceInfo {
    /// Reference name (branch or tag name)
    pub name: String,
    
    /// Full reference path (e.g., "refs/heads/main")
    pub full_ref: String,
    
    /// Commit SHA this reference points to
    pub commit_id: String,
}

/// Response for the show_file_contents tool
///
/// This type directly uses the FileContents enum from the
/// lumin crate to ensure consistency with the internal implementation.
pub type FileContentsResponse = FileContents;