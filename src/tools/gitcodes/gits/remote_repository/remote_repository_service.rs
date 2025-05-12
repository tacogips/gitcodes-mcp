// Repository information struct has been moved to git_repository.rs

/// Service for GitHub repository operations
///
/// This struct provides integrated tools for GitHub operations:
/// - Repository searching
/// - Code searching within repositories
/// - Branch and tag listing
///
/// # Authentication
///
/// The service handles GitHub authentication through the `GITCODE_MCP_GITHUB_TOKEN`
/// environment variable. This token is:
/// - Read once at startup and stored in memory
/// - Used for all GitHub API requests
/// - Optional, but recommended to avoid rate limiting (60 vs 5,000 requests/hour)
/// - Required for accessing private repositories (with `repo` scope)
#[derive(Clone)]
pub struct GitRemoteRepositoryService {
    /// HTTP client for API requests
    pub client: Client,
    /// GitHub authentication token (if provided via GITCODE_MCP_GITHUB_TOKEN)
    pub github_token: Option<String>,
}

impl Default for GitRemoteRepositoryService {
    fn default() -> Self {
        Self::with_default_cache_dir(None)
    }
}

impl GitRemoteRepositoryService {
    /// Creates a new Gitservice instance
    ///
    /// Initializes:
    /// - HTTP client for API requests
    /// - Repository manager for Git operations
    /// - GitHub token from the provided parameter or environment variable
    ///
    /// # Authentication
    ///
    /// Authentication can be provided in three ways:
    /// 1. Command line argument `--github-token` (highest priority)
    /// 2. Explicitly via the `github_token` parameter in code (second priority)
    /// 3. Environment variable `GITCODE_MCP_GITHUB_TOKEN` (used as fallback)
    ///
    /// If no token is provided through either method, the system will work with
    /// lower rate limits (60 requests/hour vs 5,000 requests/hour).
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication. If None, will attempt to read from environment.
    /// * `repository_cache_dir` - Optional custom directory path for storing cloned repositories.
    ///
    /// # Returns
    ///
    /// A new GitHubService instance or panics if the repository manager cannot be initialized.
    pub fn new(github_token: Option<String>, repository_cache_dir: Option<PathBuf>) -> Self {
        Self {
            client: Client::new(),
            github_token,
        }
    }

    /// Creates a new GitHub service instance with the default repository cache directory
    ///
    /// This is a convenience constructor that uses the system's temporary directory
    /// for storing repositories.
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication.
    pub fn with_default_cache_dir(github_token: Option<String>) -> Self {
        Self::new(github_token, None)
    }

    /// Get the authentication status for display
    pub fn get_auth_status(&self) -> String {
        if self.github_token.is_some() {
            "Authenticated GitHub API access enabled (5,000 requests/hour)".to_string()
        } else {
            "Unauthenticated GitHub API access (60 requests/hour limit). Set GITCODE_MCP_GITHUB_TOKEN for higher limits.".to_string()
        }
    }

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
    pub async fn search_repositories(&self, params: SearchParams) -> String {
        //TODO(tacogips) this method should return anyhow::Result<String> instead of String
        // Execute the search request
        github_api::execute_search_request(&params, &self.client, self.github_token.as_ref()).await
    }
}
