use crate::gitcodes::{repository_manager, *};
use crate::services;
use rmcp::{model::*, schemars, tool, ServerHandler};
use std::path::PathBuf;

// Re-export SortOption and OrderOption from repository_manager
pub use crate::gitcodes::repository_manager::{OrderOption, SortOption, SearchParams};

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
    /// 2. Environment variable `GITCODE_MCP_GITHUB_TOKEN` (used as fallback)
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

## Authentication
You can authenticate in three ways:

### Option 1: Command Line Argument (highest priority)
```
gitcodes-mcp stdio --github-token=your_token
gitcodes-mcp http --github-token=your_token
```

### Option 2: Environment Variable (used as fallback)
```
export GITCODE_MCP_GITHUB_TOKEN=your_github_token
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
    /// - Uses the `GITCODE_MCP_GITHUB_TOKEN` if available for authentication
    /// - Without a token, limited to 60 requests/hour
    /// - With a token, allows 5,000 requests/hour
    ///
    /// # Rate Limiting
    ///
    /// GitHub API has rate limits that vary based on authentication:
    /// - Unauthenticated: 60 requests/hour
    /// - Authenticated: 5,000 requests/hour
    #[tool(
        description = "Search for repositories on Git providers (currently only GitHub is supported). Searches GitHub's API for repositories matching your query. Supports sorting by stars, forks, or update date, and pagination for viewing more results.  Example usage: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"rust http client\"}}`. With provider: `{\"name\": \"search_repositories\", \"arguments\": {\"provider\": \"github\", \"query\": \"rust web framework\"}}`. With sorting: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"game engine\", \"sort_by\": \"Stars\", \"order\": \"Descending\"}}`. With pagination: `{\"name\": \"search_repositories\", \"arguments\": {\"query\": \"machine learning\", \"per_page\": 50, \"page\": 2}}`"
    )]
    async fn search_repositories(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Git provider to search (optional, default is 'github'). Currently, only 'github' is supported as a valid provider."
        )]
        provider: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Search query (required) - keywords to search for repositories. Can include advanced search qualifiers like 'language:rust' or 'stars:>1000'. Maximum length is 256 characters."
        )]
        query: String,

        #[tool(param)]
        #[schemars(
            description = "How to sort results (optional, default is 'relevance'). Options: Stars (most starred), Forks (most forked), Updated (most recently updated). When unspecified, results are sorted by best match to the query."
        )]
        sort_by: Option<SortOption>,

        #[tool(param)]
        #[schemars(
            description = "Sort order (optional, default is 'descending'). Options: Ascending (lowest to highest), Descending (highest to lowest). For date-based sorting like 'Updated', Descending means newest first."
        )]
        order: Option<OrderOption>,

        #[tool(param)]
        #[schemars(
            description = "Results per page (optional, default is 30, max 100). Controls how many repositories are returned in a single response. Higher values provide more comprehensive results but may include less relevant items."
        )]
        per_page: Option<u8>,

        #[tool(param)]
        #[schemars(
            description = "Result page number (optional, default is 1). Used for pagination to access results beyond the first page. GitHub limits search results to 1000 items total (across all pages)."
        )]
        page: Option<u32>,
    ) -> String {
        use crate::gitcodes::repository_manager::providers::GitProvider;
        use std::str::FromStr;

        // Parse the provider string or use default (GitHub)
        let git_provider = match provider.as_deref() {
            Some(provider_str) => match GitProvider::from_str(provider_str) {
                Ok(provider) => provider,
                Err(_) => {
                    return format!(
                        "Invalid provider: '{}'. Currently only 'github' is supported.",
                        provider_str
                    );
                }
            },
            None => GitProvider::Github, // Default to GitHub if not provided
        };

        // Now we can pass the SortOption and OrderOption directly to search_repositories
        // since it accepts these types directly

        // Execute the search against the specified provider using the repository manager
        match self.manager.search_repositories(
            git_provider,
            query,
            sort_by, // Pass directly since repository_manager uses the same enum types
            order,  // Pass directly since repository_manager uses the same enum types
            per_page,
            page,
        ).await {
            Ok(result) => result,
            Err(err) => format!("Search failed: {}", err),
        }
    }

    /// Search code in a GitHub repository
    ///
    /// This tool clones or updates the repository locally, then performs a code search
    /// using the specified pattern. It supports both public and private repositories.
    ///
    /// # Authentication
    ///
    /// - For public repositories: No authentication needed
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool uses a combination of git operations and the lumin search library:
    /// 1. Repository is cloned or updated locally
    /// 2. Code search is performed on the local files
    /// 3. Results are formatted and returned
    #[tool(
        description = "Search code in a GitHub repository or local directory. For GitHub repos, clones the repository locally and searches for pattern matches. For local paths, searches directly in the specified directory. Supports public and private repositories, branch/tag selection, and regex search. The pattern is interpreted as a regular expression, and it's your responsibility to escape special characters for literal searches. Examples: Simple search: `{\"name\": \"grep_repository\", \"arguments\": {\"repository_location\": \"https://github.com/rust-lang/rust\", \"pattern\": \"fn main\"}}`. With branch: `{\"name\": \"grep_repository\", \"arguments\": {\"repository_location\": \"github:tokio-rs/tokio\", \"ref_name\": \"master\", \"pattern\": \"async fn\"}}`. Literal text search for 'file.txt': `{\"name\": \"grep_repository\", \"arguments\": {\"repository_location\": \"github:user/repo\", \"pattern\": \"file\\.txt\"}}`. With search options: `{\"name\": \"grep_repository\", \"arguments\": {\"repository_location\": \"/path/to/local/repo\", \"pattern\": \"Deserialize\", \"case_sensitive\": true, \"file_extensions\": [\"rs\"]}}`"
    )]
    async fn grep_repository(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local file path (required) - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths like '/path/to/repo'. For private repositories, the GITCODE_MCP_GITHUB_TOKEN environment variable must be set with a token having 'repo' scope. Local paths must be absolute and currently only support Linux/macOS format (Windows paths not supported)."
        )]
        repository_location: String,

        #[tool(param)]
        #[schemars(
            description = "Branch, Commit or tag (optional, default is 'main' or 'master'). Specifies which branch or tag to search in. If the specified branch doesn't exist, falls back to 'main' or 'master'."
        )]
        ref_name: Option<String>,

        #[tool(param)]
        #[schemars(
            description = "Search pattern (required) - the text pattern to search for in the code. Interpreted as a regular expression. You can use regex syntax such as: simple literals like 'function'; wildcards like 'log.txt' (matches 'log1txt' too because '.' matches any character); character classes '[0-9]+'; word boundaries '\\bword\\b'; line anchors '^function'; alternatives 'error|warning'; repetitions '.*'. For literal text search, YOU MUST escape special characters yourself. For example, to search for the literal string 'file.txt', use 'file\\.txt'; to search for 'array[0]', use 'array\\[0\\]'; to search for '2+2=4', use '2\\+2=4'. Escape the following characters when searching for them literally: '.', '*', '+', '?', '^', '$', '[', ']', '(', ')', '{', '}', '|', '\\'. You can use this logic to escape a pattern for literal search: for each character c in the pattern, if c is one of '.^$*+?()[]{}\\|', prepend it with '\\'."
        )]
        pattern: String,

        #[tool(param)]
        #[schemars(
            description = "Whether to be case-sensitive (optional, default is false). When true, matching is exact with respect to letter case. When false, matches any letter case."
        )]
        case_sensitive: Option<bool>,

        #[tool(param)]
        #[schemars(
            description = "File extensions to search (optional, e.g., [\"rs\", \"toml\"]). Limits search to files with specified extensions. Omit to search all text files."
        )]
        file_extensions: Option<Vec<String>>,

        #[tool(param)]
        #[schemars(
            description = "Directories to exclude from search (optional, e.g., [\"target\", \"node_modules\"]). Skips specified directories during search. Common build directories are excluded by default."
        )]
        exclude_dirs: Option<Vec<String>>,
    ) -> Result<String, String> {
        // Get the effective case sensitivity (default to false if not specified)
        let case_sensitive = case_sensitive.unwrap_or(false);

        // Process code search within the repository (grep)
        // Handle repository cleanup in both success and error cases
        

        match services::perform_grep_in_repository(
            &self.manager,
            &repository_location,
            pattern,
            ref_name.as_deref(),
            case_sensitive,
            file_extensions.as_ref(),
            exclude_dirs.as_ref(),
        )
        .await
        {
            Ok((result, _local_repo)) => {
                // Successful search, convert result to JSON
                let json_result = result.to_json()?;

                // Note: We don't clean up the repository here to use it as a cache
                // This improves performance for subsequent operations
                tracing::debug!("Repository kept for caching");

                Ok(json_result)
            }
            Err(err) => {
                // Search failed, try to clean up repository if it was created
                tracing::error!("Code search failed: {}", err);

                // Note: We don't clean up the repository here even on error
                // to preserve it for potential future operations
                tracing::debug!("Repository kept for caching even after error");

                // Return the original error
                Err(err)
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
    /// - For private repositories: Requires `GITCODE_MCP_GITHUB_TOKEN` with `repo` scope
    ///
    /// # Implementation Note
    ///
    /// This tool:
    /// 1. Clones or updates the repository locally
    /// 2. Fetches all branches and tags
    /// 3. Formats the results into a readable format
    #[tool(
        description = "List branches and tags for a GitHub repository. Clones the repository locally and retrieves all branches and tags. Returns a formatted list of available references. Example usage: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository_location\": \"https://github.com/rust-lang/rust\"}}`. Another example: `{\"name\": \"list_repository_refs\", \"arguments\": {\"repository_location\": \"github:tokio-rs/tokio\"}}`"
    )]
    async fn list_repository_refs(
        &self,
        #[tool(param)]
        #[schemars(
            description = "Repository URL or local file path (posix only) (required) - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths like '/path/to/repo'. For private repositories, the GITCODE_MCP_GITHUB_TOKEN environment variable must be set with a token having 'repo' scope. Local paths must be absolute and currently only support Linux/macOS format (Windows paths not supported)."
        )]
        repository_location: String,
    ) -> Result<String, String> {
        // Use the repository manager directly to handle repository refs listing
        let (refs_json, local_repo) = self.manager.list_repository_refs(&repository_location).await?;
        
        // Note: We don't clean up the repository here to use it as a cache
        // This improves performance for subsequent operations
        if local_repo.is_some() {
            tracing::debug!("Repository kept for caching");
        }
        
        // Return the JSON result
        Ok(refs_json)
    }
}
