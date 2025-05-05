//! GitCodes MCP Library for GitHub and Rust crate documentation
//!
//! This library provides Model Context Protocol (MCP) tools for working with:
//! - GitHub repositories (search, code grep, branch/tag listing)
//! - Rust crate documentation (planned)
//!
//! ## Features
//!
//! - Search for GitHub repositories
//! - Search code within repositories (grep)
//! - List repository branches and tags
//!
//! ## Authentication
//!
//! GitHub operations support both authenticated and unauthenticated access.
//! Authentication is handled through the `GITCODE_MCP_GITHUB_TOKEN` environment variable.
//!
//! ```bash
//! # Set GitHub token for authentication (optional)
//! export GITCODE_MCP_GITHUB_TOKEN=your_github_token
//! ```
//!
//! ### GitHub Token (`GITCODE_MCP_GITHUB_TOKEN`)
//!
//! - **Purpose**: Authenticates requests to the GitHub API
//! - **Requirement**: Optional, but recommended to avoid rate limits
//! - **Rate Limits**:
//!   - Without token: 60 requests/hour (unauthenticated)
//!   - With token: 5,000 requests/hour (authenticated)
//! - **Private Repositories**: Requires token with `repo` scope
//!
//! ### When Token is NOT Required
//!
//! A GitHub token is not required if:
//! - You're only accessing public repositories
//! - You're making few requests (under 60 per hour)
//! - You don't need to access private repositories
//!
//! ## Usage
//!
//! This library can be used in several ways:
//! - As an MCP server (HTTP/SSE mode)
//! - As an MCP server (STDIN/STDOUT mode)
//! - Directly as a Rust library
//!
//! See the README.md file for usage examples.

pub mod tools;
pub mod transport;
