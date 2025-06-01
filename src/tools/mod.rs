use crate::gitcodes::{repository_manager, *};
use crate::services;
use rmcp::{Error as McpError, ServerHandler, model::*, schemars, tool};
use std::path::PathBuf;

use crate::gitcodes::repository_manager::providers::models::GitProvider;
use std::str::FromStr;
mod error;
pub mod responses;

// Re-export SortOption and OrderOption from repository_manager
pub use crate::gitcodes::repository_manager::{
    IssueSortOption, OrderOption, SearchParams, SortOption,
};

// Note on Response Types:
// Previously, the tool methods returned String or Result<String, String> directly,
// which meant that responses were plain JSON strings. This was inconsistent
// and required parsing by consumers.
//
// With the changes made in our architecture, we now maintain strongly-typed response
// structures in the responses module. These are used for internal validation and processing.
//
// MCP tool methods must return Result<CallToolResult, McpError> to be compatible
// with the framework. So we create structured response types for internal use,
// serialize them to JSON, and then wrap them in CallToolResult for the MCP framework.
//
// This approach gives us the best of both worlds:
// 1. Strongly-typed internal processing with proper validation
// 2. Consistent interface with the MCP framework
// 3. Helper methods (success_result and error_result) to ensure consistency
//
// The helper methods in this module ensure that all responses and errors are
// formatted consistently according to the MCP protocol requirements.

/// Wrapper for GitHub code tools exposed through the MCP protocol
///
/// This struct is a thin wrapper around the RepositoryManager, specifically
/// designed to expose functionality through the MCP tool protocol.
#[derive(Clone)]
pub struct GitDbTools {
    /// The underlying GitHub service implementation
    manager: RepositoryManager,
}

impl GitDbTools {
    /// Creates a new GitHubCodeTools instance with optional authentication and custom repository cache dir
    ///
    /// # Authentication
    ///
    /// Authentication can be provided in two ways:
    /// 1. Explicitly via the `github_token` parameter (highest priority)
    /// 2. Environment variable `GITCODES_MCP_GITHUB_TOKEN` (used as fallback)
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication. If None, will attempt to read from environment.
    /// * `repository_cache_dir` - Optional path to a directory for storing cloned repositories.
    ///
    /// This method initializes or reuses the global RepositoryManager instance to ensure
    /// the same process_id is used throughout the process lifetime.
    pub fn new(github_token: Option<String>, repository_cache_dir: Option<PathBuf>) -> Self {
        // Initialize the global repository manager with these parameters
        // This will only have an effect the first time it's called
        let manager = repository_manager::instance::init_repository_manager(
            github_token,
            repository_cache_dir,
        );

        Self {
            manager: manager.clone(),
        }
    }

    /// Creates a new GitHubCodeTools with the global RepositoryManager instance
    ///
    /// This method ignores the passed manager parameter and uses the global instance instead,
    /// ensuring that all instances share the same process_id.
    pub fn with_service(_manager: RepositoryManager) -> Self {
        // Get the global repository manager
        let manager = repository_manager::instance::get_repository_manager();
        Self {
            manager: manager.clone(),
        }
    }
}

#[tool(tool_box)]
impl ServerHandler for GitDbTools {
    /// Provides information about this MCP server
    ///
    /// Returns server capabilities, protocol version, and usage instructions
    fn get_info(&self) -> ServerInfo {
        // Check auth status based on github_token
        let auth_status = match &self.manager.github_token {
            Some(_) => "Authenticated with GitHub token",
            None => "Not authenticated (rate limits apply)",
        };

        let instructions = format!("{}", auth_status);

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(instructions),
        }
    }
}

#[tool(tool_box)]
impl GitDbTools {
    /// List branches and tags for a GitHub repository
    ///
    /// This tool retrieves a list of all branches and tags for the specified repository.
    /// It supports both public and private repositories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODES_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool:
    /// 1. Clones or updates the repository locally
    /// 2. Fetches all branches and tags
    /// 3. Formats the results into a readable format
    #[tool(
        description = "List all branches and tags for a repository. Clones locally to retrieve references. Example: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository_location\": \"git@github.com:user/repo.git\"}}`"
    )]
    async fn list_repository_refs(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local path (required). Supports GitHub formats: 'git@github.com:user/repo.git' (SSH, recommended), 'https://github.com/user/repo', 'github:user/repo', or absolute local paths. Private repos require GITCODES_MCP_GITHUB_TOKEN environment variable. This parameter is required and must be provided."
        )]
        repository_location: String,
    ) -> Result<CallToolResult, McpError> {
        // Use the repository manager directly to handle repository refs listing
        match self
            .manager
            .list_repository_refs(&repository_location)
            .await
        {
            Ok((repo_refs, local_repo)) => {
                // Note: We don't clean up the repository here to use it as a cache
                // This improves performance for subsequent operations
                if local_repo.is_some() {
                    tracing::debug!("Repository kept for caching");
                }

                // Convert the structured repository refs to our response format
                let response = responses::RepositoryRefsResponse {
                    branches: repo_refs
                        .branches
                        .into_iter()
                        .map(|ref_info| responses::ReferenceInfo {
                            name: ref_info.name,
                            full_ref: ref_info.full_ref,
                            commit_id: ref_info.commit_id,
                        })
                        .collect(),
                    tags: repo_refs
                        .tags
                        .into_iter()
                        .map(|ref_info| responses::ReferenceInfo {
                            name: ref_info.name,
                            full_ref: ref_info.full_ref,
                            commit_id: ref_info.commit_id,
                        })
                        .collect(),
                };

                // Serialize the response to JSON
                match serde_json::to_string(&response) {
                    Ok(json) => success_result(json),
                    Err(e) => error_result(format!("Failed to serialize repository refs: {}", e)),
                }
            }
            Err(err) => error_result(format!("Failed to list repository refs: {}", err)),
        }
    }
}

/// Helper method to create a CallToolResult for successful responses
fn success_result(json: String) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Helper method to create a CallToolResult for error responses
fn error_result(message: impl Into<String>) -> Result<CallToolResult, McpError> {
    let error_message = message.into();
    Ok(CallToolResult::error(vec![Content::text(error_message)]))
}
