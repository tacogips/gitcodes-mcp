//! GitHub tools for interacting with repositories and code search
//!
//! This module provides tools for:
//! - Searching GitHub repositories
//! - Searching code within repositories (grep functionality)
//! - Listing branches and tags of repositories
//!
//! ## Authentication
//!
//! These tools support both authenticated and unauthenticated access to GitHub.
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
//! use gitcodes_mcp::tools::gitcodes::{github_service::GitHubService, GitHubCodeTools};
//!
//! let github_service = GitHubService::new(Some("your_github_token".to_string()));
//!
//! // Or when creating the tools wrapper
//! let github_tools = GitHubCodeTools::new(Some("your_github_token".to_string()));
//! ```
//!
//! ### GitHub Token
//!
//! - **Purpose**: Authenticates requests to GitHub API
//! - **Requirement**: Optional, but strongly recommended to avoid rate limits
//! - **Rate Limits**:
//!   - Without token: 60 requests/hour (unauthenticated)
//!   - With token: 5,000 requests/hour (authenticated)
//! - **Usage**: Set as environment variable or provide programmatically
//! - **Security**: Token is stored in memory
//! - **Permissions**: For private repositories, token must have `repo` scope
//!
//! ### When Token is NOT Required
//!
//! A GitHub token is not required if:
//! - You're only accessing public repositories
//! - You're making few requests (under 60 per hour)
//! - You don't need to access private repositories
//!
//! All public repository operations work without authentication, but with
//! significantly lower rate limits.

// Export the github_service module
pub mod github_service;
pub use github_service::{GitHubService, RepositoryManager, params::{SearchParams, SortOption, OrderOption, GrepParams}};

// Re-export the tools implementation
mod tools;
pub use tools::GitHubCodeTools;