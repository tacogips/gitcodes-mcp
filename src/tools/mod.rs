use crate::gitcodes::{repository_manager, *};
use crate::services;
use rmcp::{model::*, schemars, tool, Error as McpError, ServerHandler};
use std::path::PathBuf;

use crate::gitcodes::repository_manager::providers::models::GitProvider;
use std::str::FromStr;
mod error;
pub mod responses;

// Re-export SortOption and OrderOption from repository_manager
pub use crate::gitcodes::repository_manager::{OrderOption, SearchParams, SortOption};

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
pub struct GitHubCodeTools {
    /// The underlying GitHub service implementation
    manager: RepositoryManager,
}

impl GitHubCodeTools {
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

    /// Creates a new GitHubCodeTools instance with default repository cache directory
    ///
    /// This is a convenience constructor that uses the system's temporary directory.
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication.
    pub fn with_default_cache_dir(github_token: Option<String>) -> Self {
        Self::new(github_token, None)
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

impl Default for GitHubCodeTools {
    fn default() -> Self {
        Self::with_default_cache_dir(None)
    }
}

#[tool(tool_box)]
impl ServerHandler for GitHubCodeTools {
    /// Provides information about this MCP server
    ///
    /// Returns server capabilities, protocol version, and usage instructions
    fn get_info(&self) -> ServerInfo {
        // Check auth status based on github_token
        let auth_status = match &self.manager.github_token {
            Some(_) => "Authenticated with GitHub token",
            None => "Not authenticated (rate limits apply)",
        };

        let instructions = format!(
            "# GitHub and Rust Documentation MCP Server

## Authentication Status
{}

## Available Tools
- `search_repositories`: Search for GitHub repositories
- `grep_repository`: Search code within a GitHub repository
- `list_repository_refs`: List branches and tags for a repository
- `show_file_contents`: View file contents in compact format with concatenated lines and enhanced metadata
- `get_repository_tree`: Get the directory tree structure of a repository

## Response Format Updates

### show_file_contents Compact Format
The `show_file_contents` tool now returns a more concise format:
```json
{{
  \"type\": \"text\",
  \"line_contents\": \"1:## User Guide\\n2:\\n3:This guide explains...\",
  \"metadata\": {{
    \"file_path\": \"README.md\",
    \"line_count\": 100,
    \"size\": 1234
  }}
}}
```

Key improvements:
- Line contents are concatenated into a single string with line numbers
- Metadata includes full file path instead of just filename
- Size field replaces char_count for consistency
- Significantly reduced JSON verbosity

## New File Filtering Feature
The `grep_repository` tool now supports powerful glob pattern filtering with the include_globs parameter. This allows you to precisely specify which files to search by pattern.

## Authentication
You can authenticate in three ways:

### Option 1: Command Line Argument (highest priority)
```
gitcodes-cli stdio --github-token=your_token
gitcodes-cli http --github-token=your_token
```

### Option 2: Environment Variable (used as fallback)
```
export GITCODES_MCP_GITHUB_TOKEN=your_github_token
```

### Option 3: Programmatic via new() method
Initialize with token using:
GitHubCodeTools::new(Some(\"token\"))

GitHub token is optional for public repositories but required for:
- Higher rate limits (5,000 vs 60 requests/hour)
- Accessing private repositories (requires 'repo' scope)
",
            auth_status
        );

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(instructions),
        }
    }
}

#[tool(tool_box)]
impl GitHubCodeTools {
    /// Search for GitHub repositories using the GitHub API
    ///
    /// This method searches for repositories on GitHub based on the provided query.
    /// It supports sorting, pagination, and uses GitHub's search API.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODES_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour
    /// - With a token, allows 5,000 requests/hour
    ///
    /// # Rate Limiting
    ///
    /// GitHub API has rate limits that vary based on authentication:
    /// - Unauthenticated: 60 requests/hour
    /// - Authenticated: 5,000 requests/hour
    #[tool(
        description = "Search GitHub repositories by query. Supports sorting and pagination. Example: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"rust http client\"}}`. With sorting: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"game engine\", \"sort_by\": \"Stars\"}}`"
    )]
    async fn search_repositories(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Git provider (optional, default 'github'). Currently only 'github' is supported. When omitted, defaults to GitHub. Example: 'github'."
        )]
        provider: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Search query for repositories (required). Supports GitHub search qualifiers like 'language:rust' or 'stars:>1000'. Max 256 characters. This parameter is required and must be provided for the search to execute."
        )]
        query: String,

        #[tool(param)]
        #[schemars(
            description = "Sort results by (optional, default 'relevance'). Valid options: 'Stars' (sort by star count), 'Forks' (sort by fork count), 'Updated' (sort by last update time). When omitted, sorts by relevance score."
        )]
        sort_by: Option<SortOption>,

        #[tool(param)]
        #[schemars(
            description = "Sort order (optional, default 'descending'). Valid options: 'Ascending' (lowest to highest), 'Descending' (highest to lowest). When omitted, uses descending order."
        )]
        order: Option<OrderOption>,

        #[tool(param)]
        #[schemars(
            description = "Results per page (optional, default 5, max 100). Must be between 1 and 100. Controls pagination size for search results."
        )]
        per_page: Option<u8>,

        #[tool(param)]
        #[schemars(
            description = "Page number for pagination (optional, default 1). Must be positive integer. GitHub limits total results to 1000 items, so max effective page depends on per_page value."
        )]
        page: Option<u32>,
    ) -> Result<CallToolResult, McpError> {
        inner_search_repositories(
            &self.manager,
            provider,
            query,
            sort_by,
            order,
            per_page,
            page,
        )
        .await
    }

    /// Search code in a GitHub repository
    ///
    /// This tool clones or updates the repository locally, then performs a code search
    /// using the specified pattern. It supports both public and private repositories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODES_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool uses a combination of git operations and the lumin search library:
    /// 1. Repository is cloned or updated locally
    /// 2. Code search is performed on the local files
    /// 3. Results are formatted and returned
    #[tool(
        description = "Search code in GitHub repositories or local directories using regex patterns. Clones repos locally for searching. Supports private repos, branch selection, and context lines. Example: `{\"name\": \"grep_repository\", \"arguments\": {\"repository_location\": \"git@github.com:rust-lang/rust.git\", \"pattern\": \"fn main\"}}`. With options: `{\"name\": \"grep_repository\", \"arguments\": {\"repository_location\": \"github:user/repo\", \"pattern\": \"async fn\", \"case_sensitive\": true}}`"
    )]
    #[allow(clippy::too_many_arguments)]
    async fn grep_repository(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local path (required). Supports GitHub formats: 'git@github.com:user/repo.git' (SSH, recommended), 'https://github.com/user/repo', 'github:user/repo', or absolute local paths. Private repos require GITCODES_MCP_GITHUB_TOKEN environment variable. This parameter is required and must be provided."
        )]
        repository_location: String,

        #[tool(param)]
        #[schemars(
            description = "Branch, commit, or tag (optional, default 'main'/'master'). Can be branch name (e.g. 'develop'), commit hash (full or short), or tag name (e.g. 'v1.0.0'). Falls back to repository's default branch if specified ref doesn't exist."
        )]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Regular expression pattern to search for (required). Escape special regex characters for literal searches: '.^$*+?()[]{}\\|' should be prefixed with backslash for literal matching. This parameter is required and must be provided."
        )]
        pattern: String,

        #[tool(param)]
        #[schemars(
            description = "Case-sensitive matching (optional, default false). When true, pattern matching distinguishes between uppercase and lowercase characters. When false or omitted, performs case-insensitive search."
        )]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "[DEPRECATED] File extensions to search. Use include_globs instead."
        )]
        file_extensions: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Glob patterns to include (optional). Filters files to search using glob syntax. Examples: [\"**/*.rs\"] (all Rust files), [\"src/**/*.md\"] (Markdown files in src), [\"*.json\", \"*.yaml\"] (config files). When omitted, searches all text files."
        )]
        include_globs: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Directories to exclude (optional). List of directory names to skip during search. Common examples: [\"target\", \"node_modules\"] (build artifacts), [\".git\", \".svn\"] (version control), [\"dist\", \"build\"] (output directories). When omitted, respects .gitignore patterns."
        )]
        exclude_dirs: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Lines of context before each match (optional, default 0). Number of lines to show before matching line for context. Must be non-negative integer. Useful for understanding match context."
        )]
        before_context: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Lines of context after each match (optional, default 0). Number of lines to show after matching line for context. Must be non-negative integer. Useful for understanding match context."
        )]
        after_context: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Number of results to skip for pagination (optional). Must be non-negative integer. Use with 'take' parameter to implement pagination. Example: skip=20, take=10 gets results 21-30."
        )]
        skip: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Maximum number of results to return (optional). Must be positive integer. Controls result set size to prevent overwhelming responses. Use with 'skip' parameter for pagination."
        )]
        take: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Maximum number of characters to show from matched content (optional, default 150). When matches contain very long lines, this parameter truncates the content to the specified number of characters to keep responses manageable. Set to None to show full content without truncation."
        )]
        match_content_omit_num: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let grep_params = services::GrepParams {
            repository_location_str: repository_location.clone(),
            pattern,
            ref_name: ref_name.clone(),
            case_sensitive: case_sensitive.unwrap_or(false),
            file_extensions: file_extensions.clone(),
            include_globs: include_globs.clone(),
            exclude_dirs: exclude_dirs.clone(),
            before_context,
            after_context,
            skip,
            take,
            match_content_omit_num,
        };

        let result = inner_grep_repositories(&self.manager, grep_params).await;

        match result {
            Ok((result, _local_repo)) => {
                // Note: We don't clean up the repository here to use it as a cache
                // This improves performance for subsequent operations
                tracing::debug!("Repository kept for caching");

                // Serialize the result to JSON and return it
                match serde_json::to_string(&result) {
                    Ok(json) => success_result(json),
                    Err(e) => error_result(format!("Failed to serialize search results: {}", e)),
                }
            }
            Err(err) => {
                // Search failed, try to clean up repository if it was created
                tracing::error!("Code search failed: {}", err);

                // Note: We don't clean up the repository here even on error
                // to preserve it for potential future operations
                tracing::debug!("Repository kept for caching even after error");

                // Return the original error as an error result
                error_result(format!("Code search failed: {}", err))
            }
        }
    }

    /// Returns only the total number of matching lines for a code search
    ///
    /// This method is a lightweight alternative to `grep_repository` when you only need
    /// the count of matching lines rather than the full match details. It performs the
    /// same search operation but returns just a number indicating the total matches found.
    ///
    /// This is useful for estimating the size of a potential result set before doing a more
    /// detailed search, or for scenarios where only the count matters (e.g., checking if a
    /// pattern exists at all in a repository).
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODES_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This method uses the same internal search mechanism as `grep_repository` but only
    /// returns the total count of matches rather than the full result details.
    #[tool(
        description = "Count matching lines in repository code search. Works like grep_repository but returns only the total count. Example: `{\"name\": \"grep_repository_match_line_number\", \"arguments\": {\"repository_location\": \"git@github.com:user/repo.git\", \"pattern\": \"fn main\"}}`"
    )]
    #[allow(clippy::too_many_arguments)]
    async fn grep_repository_match_line_number(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local path (required). Supports GitHub formats: 'git@github.com:user/repo.git' (SSH, recommended), 'https://github.com/user/repo', 'github:user/repo', or absolute local paths. Private repos require GITCODES_MCP_GITHUB_TOKEN environment variable. This parameter is required and must be provided."
        )]
        repository_location: String,

        #[tool(param)]
        #[schemars(
            description = "Branch, commit, or tag (optional, default 'main'/'master'). Can be branch name (e.g. 'develop'), commit hash (full or short), or tag name (e.g. 'v1.0.0'). Falls back to repository's default branch if specified ref doesn't exist."
        )]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Regular expression pattern to search for (required). Escape special regex characters for literal searches: '.^$*+?()[]{}\\|' should be prefixed with backslash for literal matching. This parameter is required and must be provided."
        )]
        pattern: String,

        #[tool(param)]
        #[schemars(
            description = "Case-sensitive matching (optional, default false). When true, pattern matching distinguishes between uppercase and lowercase characters. When false or omitted, performs case-insensitive search."
        )]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "[DEPRECATED] File extensions to search. Use include_globs instead."
        )]
        file_extensions: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Glob patterns to include (optional). Filters files to search using glob syntax. Examples: [\"**/*.rs\"] (all Rust files), [\"src/**/*.md\"] (Markdown files in src), [\"*.json\", \"*.yaml\"] (config files). When omitted, searches all text files."
        )]
        include_globs: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Directories to exclude (optional). List of directory names to skip during search. Common examples: [\"target\", \"node_modules\"] (build artifacts), [\".git\", \".svn\"] (version control), [\"dist\", \"build\"] (output directories). When omitted, respects .gitignore patterns."
        )]
        exclude_dirs: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Lines of context before each match (optional, default 0). Number of lines to show before matching line for context. Must be non-negative integer. Useful for understanding match context."
        )]
        before_context: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Lines of context after each match (optional, default 0). Number of lines to show after matching line for context. Must be non-negative integer. Useful for understanding match context."
        )]
        after_context: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Number of results to skip for pagination (optional). Must be non-negative integer. Use with 'take' parameter to implement pagination. Example: skip=20, take=10 gets results 21-30."
        )]
        skip: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Maximum number of results to return (optional). Must be positive integer. Controls result set size to prevent overwhelming responses. Use with 'skip' parameter for pagination."
        )]
        take: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Maximum number of characters to show from matched content (optional, default 150). When matches contain very long lines, this parameter truncates the content to the specified number of characters to keep responses manageable. Set to None to show full content without truncation."
        )]
        match_content_omit_num: Option<usize>,
    ) -> Result<CallToolResult, McpError> {
        let grep_params = services::GrepParams {
            repository_location_str: repository_location.clone(),
            pattern,
            ref_name: ref_name.clone(),
            case_sensitive: case_sensitive.unwrap_or(false),
            file_extensions: file_extensions.clone(),
            include_globs: include_globs.clone(),
            exclude_dirs: exclude_dirs.clone(),
            before_context,
            after_context,
            skip,
            take,
            match_content_omit_num,
        };

        let result = inner_grep_repositories(&self.manager, grep_params).await;

        match result {
            Ok((result, _local_repo)) => {
                // Note: We don't clean up the repository here to use it as a cache
                // This improves performance for subsequent operations
                tracing::debug!("Repository kept for caching");

                // Serialize the result to JSON and return it
                // Just return the total number of matches as a simple number
                match serde_json::to_string(&result.total_match_line_number) {
                    Ok(json) => success_result(json),
                    Err(e) => error_result(format!("Failed to serialize match count: {}", e)),
                }
            }
            Err(err) => {
                // Search failed, try to clean up repository if it was created
                tracing::error!("Code search failed: {}", err);

                // Note: We don't clean up the repository here even on error
                // to preserve it for potential future operations
                tracing::debug!("Repository kept for caching even after error");

                // Return the original error as an error result
                error_result(format!("Code search failed: {}", err))
            }
        }
    }

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

    /// Show contents of a file in a GitHub repository in compact format
    ///
    /// This tool clones or updates the repository locally, then retrieves the contents of the specified file.
    /// It supports both public and private repositories and returns content in a compact format.
    ///
    /// # Response Format
    ///
    /// Returns a compact JSON structure with concatenated line contents:
    /// ```json
    /// {
    ///   "type": "text|binary|image",
    ///   "line_contents": "1:line content\n2:another line",
    ///   "metadata": {
    ///     "file_path": "path/to/file.ext",
    ///     "line_count": 100,
    ///     "size": 1234
    ///   }
    /// }
    /// ```
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODES_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool:
    /// 1. Repository is cloned or updated locally
    /// 2. File contents are retrieved and processed
    /// 3. Results are converted to compact format with concatenated lines and enhanced metadata
    /// 4. Response includes full file path and size information for better usability
    #[tool(
        description = "View file contents from repositories or local directories in compact format. Returns concatenated line contents with line numbers and enhanced metadata including file path. Supports line ranges and branch selection. Example: `{\"name\": \"show_file_contents\", \"arguments\": {\"repository_location\": \"git@github.com:user/repo.git\", \"file_path\": \"README.md\"}}`. With range: `{\"name\": \"show_file_contents\", \"arguments\": {\"repository_location\": \"github:user/repo\", \"file_path\": \"src/lib.rs\", \"line_from\": 10, \"line_to\": 12}}`. Returns format: `{\"type\": \"text\", \"line_contents\": \"10:content at line 10\\n11:content at line 11\\n12:content at line 12\", \"metadata\": {\"file_path\": \"src/lib.rs\", \"line_count\": 3, \"size\": 1234}}`"
    )]
    #[allow(clippy::too_many_arguments)]
    async fn show_file_contents(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local path (required). Supports GitHub formats: 'git@github.com:user/repo.git' (SSH, recommended), 'https://github.com/user/repo', 'github:user/repo', or absolute local paths. Private repos require GITCODES_MCP_GITHUB_TOKEN environment variable. This parameter is required and must be provided."
        )]
        repository_location: String,

        #[tool(param)]
        #[schemars(
            description = "Branch, commit, or tag (optional, default 'main'/'master'). Can be branch name (e.g. 'develop'), commit hash (full or short), or tag name (e.g. 'v1.0.0'). Falls back to repository's default branch if specified ref doesn't exist."
        )]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "File path relative to repository root (required). Paths with '..' are rejected for security. This parameter is required and must be provided to specify which file to view."
        )]
        file_path: String,

        #[tool(param)]
        #[schemars(
            description = "Maximum file size in bytes (optional). Must be positive integer. Prevents loading extremely large files that could cause memory issues. Example: 1048576 for 1MB limit. When omitted, uses reasonable default limit."
        )]
        max_size: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Start line number (optional, 1-indexed). Must be positive integer. Use with line_to to view specific file sections. Example: line_from=10 starts from line 10. When omitted, starts from beginning of file."
        )]
        line_from: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "End line number (optional, 1-indexed, inclusive). Must be positive integer and >= line_from. Use with line_from to view specific file sections. Example: line_to=20 ends at line 20. When omitted, reads to end of file."
        )]
        line_to: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Show content without line numbers (optional, default false). When true, returns plain file content. When false or omitted, includes line numbers for easier reference. Useful for copying code snippets."
        )]
        without_line_numbers: Option<bool>,
    ) -> Result<CallToolResult, McpError> {
        // Process file viewing within the repository
        // Handle repository cleanup in both success and error cases

        let show_params = services::ShowFileParams {
            repository_location_str: repository_location.clone(),
            file_path: file_path.clone(),
            ref_name: ref_name.clone(),
            max_size,
            line_from,
            line_to,
            without_line_numbers,
        };

        match services::show_file_contents(&self.manager, show_params).await {
            Ok((file_contents, _local_repo, _without_line_numbers)) => {
                // Note: We don't clean up the repository here to use it as a cache
                // This improves performance for subsequent operations
                tracing::debug!("Repository kept for caching");

                // Convert to compact format with full file path
                let compact_response = responses::CompactFileContentsResponse::from_file_contents(
                    file_contents,
                    file_path.clone(),
                );

                // Serialize the compact response to JSON and return it
                match serde_json::to_string(&compact_response) {
                    Ok(json) => success_result(json),
                    Err(e) => error_result(format!("Failed to serialize file contents: {}", e)),
                }
            }
            Err(err) => {
                // File viewing failed, try to clean up repository if it was created
                tracing::error!("File viewing failed: {}", err);

                // Note: We don't clean up the repository here even on error
                // to preserve it for potential future operations
                tracing::debug!("Repository kept for caching even after error");

                // Return the original error as an error result
                error_result(format!("File viewing failed: {}", err))
            }
        }
    }

    /// Get the directory tree structure of a repository
    ///
    /// This method returns a hierarchical representation of all files and directories
    /// in the repository, showing the repository's structure in tree format.
    ///
    /// # Authentication
    ///
    /// - Uses the `GITCODES_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour for GitHub repositories
    /// - With a token, allows 5,000 requests/hour for GitHub repositories
    /// - Local repositories don't require authentication
    ///
    /// # Rate Limiting
    ///
    /// GitHub API has rate limits that vary based on authentication:
    /// - Unauthenticated: 60 requests/hour
    /// - Authenticated: 5,000 requests/hour
    #[tool(
        description = "Get repository directory tree in hierarchical format. Supports depth limits, .gitignore filtering, and relative paths. Example: `{\"name\": \"get_repository_tree\", \"arguments\": {\"repository_location\": \"github:user/repo\"}}`. With options: `{\"name\": \"get_repository_tree\", \"arguments\": {\"repository_location\": \"git@github.com:user/repo.git\", \"depth\": 2, \"search_relative_path\": \"src\"}}`"
    )]
    #[allow(clippy::too_many_arguments)]
    async fn get_repository_tree(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local path (required). Supports GitHub formats: 'git@github.com:user/repo.git' (SSH, recommended), 'https://github.com/user/repo', 'github:user/repo', or absolute local paths. Private repos require GITCODES_MCP_GITHUB_TOKEN environment variable. This parameter is required and must be provided."
        )]
        repository_location: String,

        #[tool(param)]
        #[schemars(
            description = "Branch, commit, or tag (optional, default 'main'/'master'). Can be branch name (e.g. 'develop'), commit hash (full or short), or tag name (e.g. 'v1.0.0'). Falls back to repository's default branch if specified ref doesn't exist."
        )]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Case-sensitive path matching (optional, default false). When true, file and directory name matching distinguishes between uppercase and lowercase. When false or omitted, uses case-insensitive path matching."
        )]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "Respect .gitignore files (optional, default true). When true or omitted, excludes files and directories listed in .gitignore. When false, includes all files regardless of .gitignore rules. Useful for seeing complete repository structure."
        )]
        respect_gitignore: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "Maximum traversal depth (optional, default unlimited). Must be positive integer. Depth 1 shows only top-level items, depth 2 includes one level of subdirectories, etc. When omitted, traverses entire directory tree. Useful for limiting large directory structures."
        )]
        depth: Option<usize>,

        #[tool(param)]
        #[schemars(
            description = "Strip repository path prefix (optional, default true). When true or omitted, shows relative paths from repository root. When false, shows full absolute filesystem paths. Relative paths are usually more readable and portable."
        )]
        strip_path_prefix: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "Relative path to start tree generation from (optional). Path relative to repository root where tree traversal begins. Examples: 'src' (start from src directory), 'docs/api' (start from docs/api subdirectory). When omitted, starts from repository root. Useful for focusing on specific parts of large repositories."
        )]
        search_relative_path: Option<String>,
    ) -> Result<CallToolResult, McpError> {
        // Process tree retrieval within the repository
        // Handle repository cleanup in both success and error cases

        let tree_params = services::TreeServiceParams {
            repository_location_str: repository_location.clone(),
            ref_name: ref_name.clone(),
            case_sensitive,
            respect_gitignore,
            depth,
            strip_path_prefix,
            search_relative_path: search_relative_path.map(std::path::PathBuf::from),
        };

        match services::get_repository_tree(&self.manager, tree_params).await {
            Ok((tree, _local_repo)) => {
                // Note: We don't clean up the repository here to use it as a cache
                // This improves performance for subsequent operations
                tracing::debug!("Repository kept for caching");

                // Serialize the tree to JSON and return it
                match serde_json::to_string(&tree) {
                    Ok(json) => success_result(json),
                    Err(e) => error_result(format!("Failed to serialize repository tree: {}", e)),
                }
            }
            Err(err) => {
                // Tree retrieval failed, try to clean up repository if it was created
                tracing::error!("Tree retrieval failed: {}", err);

                // Note: We don't clean up the repository here even on error
                // to preserve it for potential future operations
                tracing::debug!("Repository kept for caching even after error");

                // Return the original error as an error result
                error_result(format!("Tree retrieval failed: {}", err))
            }
        }
    }
}

async fn inner_search_repositories(
    repository_manager: &RepositoryManager,
    provider: Option<String>,
    query: String,
    sort_by: Option<SortOption>,
    order: Option<OrderOption>,
    per_page: Option<u8>,
    page: Option<u32>,
) -> Result<CallToolResult, McpError> {
    // Parse the provider string or use default (GitHub)
    let git_provider = match provider.as_deref() {
        Some(provider_str) => match GitProvider::from_str(provider_str) {
            Ok(provider) => provider,
            Err(_) => {
                return error_result(format!(
                    "Invalid provider: '{}'. Currently only 'github' is supported.",
                    provider_str
                ));
            }
        },
        None => GitProvider::Github, // Default to GitHub if not provided
    };

    // Now we can pass the SortOption and OrderOption directly to search_repositories
    // since it accepts these types directly

    // Execute the search against the specified provider using the repository manager

    match repository_manager
        .search_repositories(
            git_provider,
            query,
            sort_by, // Pass directly since repository_manager uses the same enum types
            order,   // Pass directly since repository_manager uses the same enum types
            per_page,
            page,
        )
        .await
    {
        Ok(search_results) => {
            // Serialize the structured result to JSON
            match serde_json::to_string(&search_results) {
                Ok(json_result) => success_result(json_result),
                Err(e) => error_result(format!("Failed to serialize search results: {}", e)),
            }
        }
        Err(err) => error_result(format!("Search failed: {}", err)),
    }
}

async fn inner_grep_repositories(
    repository_manager: &RepositoryManager,
    grep_params: services::GrepParams,
) -> Result<
    (
        CodeSearchResult,
        crate::gitcodes::local_repository::LocalRepository,
    ),
    String,
> {
    services::perform_grep_in_repository(repository_manager, grep_params).await
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
