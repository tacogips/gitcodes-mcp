//! GitHub service for interacting with repositories and code search
//!
//! This module provides a service for:
//! - Searching GitHub repositories
//! - Searching code within repositories (grep functionality)
//! - Listing branches and tags of repositories
//!
//! ## Authentication
//!
//! The service supports both authenticated and unauthenticated access to GitHub.
//! Authentication can be provided in two ways:
//!
//! ### 1. Environment Variable
//!
//! ```bash
//! # Authentication is optional but recommended to avoid rate limiting
//! export GITCODE_MCP_GITHUB_TOKEN=your_github_token
//! ```
//!
//! ### 2. Programmatic API
//!
//! ```no_run
//! // Provide a token directly when creating the service
//! use gitcodes_mcp::tools::gitcodes::git_service::GitHubService;
//!
//! let git_service = GitHubService::new(Some("your_github_token".to_string()), None);
//! ```

mod local_repository;
pub mod remote_repository;
mod repository_manager;
use local_repository::LocalRepository;

pub use remote_repository::*;
pub use repository_manager::*;

use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

/// Enum representing a repository location, either a GitHub URL or a local filesystem path
#[derive(Debug, Clone, serde::Deserialize)]
pub enum RepositoryLocation {
    /// A GitHub repository URL (https://github.com/user/repo, git@github.com:user/repo.git, or github:user/repo)
    GitHubUrl(),
    /// A local filesystem path
    LocalPath(LocalRepository),
}

impl FromStr for RepositoryLocation {
    type Err = String;

    fn from_str(repo_location_str: &str) -> Result<Self, Self::Err> {
        let sanitized_location = repo_location_str.trim();

        // Check if it's a local path first
        //TODO(exists) check if it's a local path
        if Path::new(sanitized_location).exists() {
            return Ok(RepositoryLocation::LocalPath(LocalRepository::new(
                PathBuf::from(sanitized_location),
            )));
        }

        // Otherwise, treat it as a GitHub URL
        if sanitized_location.starts_with("https://github.com/")
            || sanitized_location.starts_with("git@github.com:")
            || sanitized_location.starts_with("github:")
        {
            Ok(RepositoryLocation::GitHubUrl(
                sanitized_location.to_string(),
            ))
        } else {
            Err(format!(
                "Invalid repository location: {}",
                sanitized_location
            ))
        }
    }
}
