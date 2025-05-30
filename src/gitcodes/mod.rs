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
//! export GITCODES_MCP_GITHUB_TOKEN=your_github_token
//! ```
//!
//! ### 2. Programmatic API
//!
//! ```no_run
//! // Provide a token directly to the repository manager
//! use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
//!
//! let repository_manager = RepositoryManager::new(Some("your_github_token".to_string()), None).unwrap();
//!
//! // Or use the global repository manager instance
//! use gitcodes_mcp::gitcodes::repository_manager::instance;
//! let manager = instance::init_repository_manager(Some("your_github_token".to_string()), None);
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
//!
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
//! export GITCODES_MCP_GITHUB_TOKEN=your_github_token
//! ```
//!
//! ### 2. Programmatic API
//!
//! ```no_run
//! // Provide a token directly when creating the repository manager
//! use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
//!
//! let repository_manager = RepositoryManager::new(Some("your_github_token".to_string()), None).unwrap();
//! ```

pub mod local_repository;
pub mod repository_manager;

pub use local_repository::*;

pub use repository_manager::*;
