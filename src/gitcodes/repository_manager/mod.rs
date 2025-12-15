pub mod instance;
pub mod providers;
mod repository_location;

use std::{num::NonZeroU32, path::PathBuf, str::FromStr};

/// Environment variable name for GitHub token
///
/// This constant defines the environment variable name that is checked for
/// GitHub authentication token when no explicit token is provided.
pub const GITHUB_TOKEN_ENV_VAR: &str = "GITHUB_TOKEN";

use gix::{progress::Discard, remote::fetch::Shallow};
use providers::GitRemoteRepository;
pub use repository_location::RepositoryLocation;
use rmcp::schemars;
use tracing;

use crate::gitcodes::local_repository::LocalRepository;

/// Sorting options for repository search
///
/// This enum defines the generic sort options that can be used across different
/// Git providers. It's used in the repository manager to provide a unified
/// interface for sorting repository search results.
///
/// When passed to provider-specific methods, these generic options are converted
/// to provider-specific sorting options using the `From` trait.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum SortOption {
    /// No specific sort, uses the provider's default relevance sorting
    Relevance,
    /// Sort by number of stars (popularity)
    Stars,
    /// Sort by number of forks (derived projects)
    Forks,
    /// Sort by most recently updated
    Updated,
}

/// Sorting options for issue search
///
/// This enum defines the generic sort options that can be used across different
/// Git providers for issue search. It's used in the repository manager to provide
/// a unified interface for sorting issue search results.
///
/// When passed to provider-specific methods, these generic options are converted
/// to provider-specific sorting options using the `From` trait.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum IssueSortOption {
    /// Sort by creation date
    Created,
    /// Sort by last update date
    Updated,
    /// Sort by number of comments
    Comments,
    /// Sort by relevance (provider's default)
    BestMatch,
}

/// Order options for repository search
///
/// This enum defines the generic order options (ascending or descending)
/// that can be used across different Git providers. It's used in the
/// repository manager to provide a unified interface for ordering
/// repository search results.
///
/// When passed to provider-specific methods, these generic options are converted
/// to provider-specific order options using the `From` trait.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub enum OrderOption {
    /// Sort in ascending order (lowest to highest, oldest to newest)
    Ascending,
    /// Sort in descending order (highest to lowest, newest to oldest)
    Descending,
}

/// Implement conversion from generic SortOption to GitHub-specific GithubSortOption
///
/// This allows us to use generic `SortOption` values throughout the codebase
/// and convert them to GitHub-specific options only when needed for API calls.
/// This maintains a clean separation between our generic API and provider-specific
/// implementation details.
impl From<SortOption> for providers::github::GithubSortOption {
    fn from(value: SortOption) -> Self {
        match value {
            SortOption::Relevance => Self::Relevance,
            SortOption::Stars => Self::Stars,
            SortOption::Forks => Self::Forks,
            SortOption::Updated => Self::Updated,
        }
    }
}

/// Implement conversion from generic IssueSortOption to GitHub-specific GithubIssueSortOption
///
/// This allows us to use generic `IssueSortOption` values throughout the codebase
/// and convert them to GitHub-specific options only when needed for API calls.
/// This maintains a clean separation between our generic API and provider-specific
/// implementation details.
impl From<IssueSortOption> for providers::github::GithubIssueSortOption {
    fn from(value: IssueSortOption) -> Self {
        match value {
            IssueSortOption::Created => Self::Created,
            IssueSortOption::Updated => Self::Updated,
            IssueSortOption::Comments => Self::Comments,
            IssueSortOption::BestMatch => Self::BestMatch,
        }
    }
}

/// Implement conversion from generic OrderOption to GitHub-specific GithubOrderOption
///
/// This allows us to use generic `OrderOption` values throughout the codebase
/// and convert them to GitHub-specific options only when needed for API calls.
/// This maintains a clean separation between our generic API and provider-specific
/// implementation details.
impl From<OrderOption> for providers::github::GithubOrderOption {
    fn from(value: OrderOption) -> Self {
        match value {
            OrderOption::Ascending => Self::Ascending,
            OrderOption::Descending => Self::Descending,
        }
    }
}

/// Repository search parameters
///
/// This struct encapsulates all the parameters needed for a repository search query.
/// It uses the generic `SortOption` and `OrderOption` enums to provide a consistent
/// interface across different Git providers.
///
/// When passed to provider-specific methods, these generic options are converted
/// to provider-specific options using the `From` trait.
#[derive(Debug, Clone)]
pub struct SearchParams {
    /// The search query string
    pub query: String,

    /// Optional sort option (defaults to Relevance if None)
    pub sort_by: Option<SortOption>,

    /// Optional order direction (defaults to Descending if None)
    pub order: Option<OrderOption>,

    /// Optional number of results per page (defaults to provider-specific value, typically 30)
    pub per_page: Option<u8>,

    /// Optional page number (defaults to 1 if None)
    pub page: Option<u32>,
}

/// Issue search parameters
///
/// This struct encapsulates all the parameters needed for an issue search query.
/// It uses the generic `IssueSortOption` and `OrderOption` enums to provide a consistent
/// interface across different Git providers.
///
/// When passed to provider-specific methods, these generic options are converted
/// to provider-specific options using the `From` trait.
#[derive(Debug, Clone)]
pub struct IssueSearchParams {
    /// The search query string
    pub query: String,

    /// Optional sort option (defaults to BestMatch if None)
    pub sort_by: Option<IssueSortOption>,

    /// Optional order direction (defaults to Descending if None)
    pub order: Option<OrderOption>,

    /// Optional number of results per page (defaults to provider-specific value, typically 30)
    pub per_page: Option<u8>,

    /// Optional page number for pagination (defaults to 1)
    pub page: Option<u32>,

    /// Repository specification in the format "owner/repo"
    /// When specified, limits search to this specific repository
    pub repository: Option<String>,

    /// Labels to search for (comma-separated)
    pub labels: Option<String>,

    /// State of issues to search for
    /// Can be "open", "closed", or "all"
    pub state: Option<String>,

    /// User who created the issue
    pub creator: Option<String>,

    /// User mentioned in the issue
    pub mentioned: Option<String>,

    /// User assigned to the issue
    /// Can be a username, "none" for unassigned, or "*" for any assignee
    pub assignee: Option<String>,

    /// Milestone number or special values
    /// Can be a number, "*" for any milestone, or "none" for no milestone
    pub milestone: Option<String>,

    /// Issue type name
    /// Can be a type name, "*" for any type, or "none" for no type
    pub issue_type: Option<String>,
}

/// Repository manager for Git operations
///
/// Handles cloning, updating, and retrieving information from GitHub repositories.
/// Uses a dedicated directory to store cloned repositories. Each RepositoryManager
/// instance has a unique process_id to differentiate it from others running in parallel.
#[derive(Clone)]
pub struct RepositoryManager {
    pub github_token: Option<String>,
    pub local_repository_cache_dir_base: PathBuf,
    /// Unique identifier for this repository manager instance
    /// Used to differentiate between multiple processes using the same repositories
    pub process_id: String,
}

impl RepositoryManager {
    /// Creates a new RepositoryManager instance with a custom repository cache directory
    ///
    /// # Parameters
    ///
    /// * `github_token` - Optional GitHub token for authentication. If None, will attempt
    ///                    to read from the GITHUB_TOKEN environment variable, and if that
    ///                    is also not set, will try to get the token from `gh auth token`.
    /// * `repository_cache_dir` - Optional custom path for storing repositories.
    ///                            If None, the system's temporary directory is used.
    ///
    /// # Returns
    ///
    /// * `Result<Self, String>` - A new RepositoryManager instance or an error message
    ///                            if the directory cannot be created or accessed.
    pub fn new(
        github_token: Option<String>,
        local_repository_cache_dir_base: Option<PathBuf>,
    ) -> Result<Self, String> {
        // If no github_token is provided, check environment variable, then gh auth
        let github_token = github_token
            .or_else(|| std::env::var(GITHUB_TOKEN_ENV_VAR).ok())
            .or_else(Self::get_token_from_gh_auth);
        // Use provided path or default to system temp directory
        let local_repository_cache_dir_base = match local_repository_cache_dir_base {
            Some(path) => path,
            None => std::env::temp_dir(),
        };

        // Validate and ensure the directory exists
        if !local_repository_cache_dir_base.exists() {
            // Try to create the directory if it doesn't exist
            std::fs::create_dir_all(&local_repository_cache_dir_base)
                .map_err(|e| format!("Failed to create repository cache directory: {}", e))?;
        } else if !local_repository_cache_dir_base.is_dir() {
            return Err(format!(
                "Specified path '{}' is not a directory",
                local_repository_cache_dir_base.display()
            ));
        }

        // Generate a unique process ID for this repository manager instance
        let process_id = Self::generate_process_id();

        Ok(Self {
            github_token,
            local_repository_cache_dir_base,
            process_id,
        })
    }

    /// Creates a new RepositoryManager with the system's default cache directory
    ///
    /// This is a convenience method that creates a RepositoryManager with the
    /// system's temporary directory as the repository cache location and a newly
    /// generated unique process ID.
    pub fn with_default_cache_dir() -> Self {
        Self::new(None, None).expect("Failed to initialize with system temporary directory")
    }

    /// Generates a unique process ID for this repository manager instance
    ///
    /// This creates a unique identifier that can be used to differentiate between
    /// multiple processes using the same repositories. The ID combines a random UUID
    /// with the current process ID for maximum uniqueness.
    fn generate_process_id() -> String {
        use std::process;
        use uuid::Uuid;

        let pid = process::id();
        let uuid = Uuid::new_v4();

        format!("{}_{}", pid, uuid.simple())
    }

    /// Attempts to get a GitHub token from the `gh` CLI tool
    ///
    /// This function checks if the `gh` command is available and if so,
    /// runs `gh auth token` to retrieve the currently authenticated token.
    ///
    /// # Returns
    ///
    /// * `Option<String>` - The GitHub token if successfully retrieved, None otherwise
    fn get_token_from_gh_auth() -> Option<String> {
        use std::process::Command;

        // First check if gh command is available
        let gh_available = Command::new("gh")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false);

        if !gh_available {
            tracing::debug!("gh CLI not found, skipping gh auth token");
            return None;
        }

        // Try to get the token from gh auth
        match Command::new("gh").arg("auth").arg("token").output() {
            Ok(output) if output.status.success() => {
                let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if token.is_empty() {
                    tracing::debug!("gh auth token returned empty string");
                    None
                } else {
                    tracing::info!("Successfully retrieved GitHub token from gh auth");
                    Some(token)
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::debug!("gh auth token failed: {}", stderr.trim());
                None
            }
            Err(e) => {
                tracing::debug!("Failed to execute gh auth token: {}", e);
                None
            }
        }
    }

    /// Gets the local repository for a given repository location without cloning
    ///
    /// This method checks if a repository has already been cloned for the given
    /// location and returns a reference to it if it exists. It will not attempt to
    /// clone the repository if it doesn't exist.
    ///
    /// # Parameters
    ///
    /// * `repo_location` - The location of the repository (local or remote)
    ///
    /// # Returns
    ///
    /// * `Result<LocalRepository, String>` - A local repository instance or an error
    ///                                      if the repository doesn't exist locally
    pub async fn get_local_path_for_repository(
        &self,
        repo_location: &RepositoryLocation,
    ) -> Result<LocalRepository, String> {
        match repo_location {
            // For local repositories, just validate and return
            RepositoryLocation::LocalPath(local_path) => {
                local_path.validate()?;
                Ok(local_path.clone())
            }
            // For remote repositories, check if we have a local clone
            RepositoryLocation::RemoteRepository(remote_repository) => {
                // Create the expected local repository instance without cloning
                let local_repo = LocalRepository::new_local_repository_to_clone(
                    match remote_repository {
                        GitRemoteRepository::Github(github_info) => github_info.repo_info.clone(),
                    },
                    Some(&self.process_id),
                );

                // Check if it exists and is valid
                let repo_dir = local_repo.get_repository_dir();
                if repo_dir.exists() && repo_dir.is_dir() {
                    match local_repo.validate() {
                        Ok(_) => Ok(local_repo),
                        Err(e) => Err(format!("Repository exists but is invalid: {}", e)),
                    }
                } else {
                    Err(format!(
                        "Repository not found locally at {}",
                        repo_dir.display()
                    ))
                }
            }
        }
    }

    /// Prepares a repository for use (clones if necessary)
    ///
    /// This method prepares a repository for use by either validating a local repository
    /// or cloning a remote one. If the repository has already been cloned, it will
    /// be reused.
    ///
    /// # Parameters
    ///
    /// * `repo_location` - The location of the repository (local or remote)
    /// * `ref_name` - Optional reference name (branch, tag) to checkout
    ///
    /// # Returns
    ///
    /// * `Result<LocalRepository, String>` - A local repository instance or an error
    pub async fn prepare_repository(
        &self,
        repo_location: &RepositoryLocation,
        ref_name: Option<String>,
    ) -> Result<LocalRepository, String> {
        match repo_location {
            RepositoryLocation::LocalPath(local_path) => {
                local_path.validate()?;
                Ok(local_path.clone())
            }
            RepositoryLocation::RemoteRepository(remote_repository) => {
                let remote_repository_with_ref_name_if_any = match (remote_repository, ref_name) {
                    // If we have a ref_name, create a new instance with that ref_name
                    (GitRemoteRepository::Github(github_info), Some(ref_name_str)) => {
                        // Create a new GitHub info with the updated ref_name
                        let mut updated_github_info = github_info.clone();
                        updated_github_info.repo_info.ref_name = Some(ref_name_str);
                        GitRemoteRepository::Github(updated_github_info)
                    }
                    // Otherwise just clone the original repository
                    _ => remote_repository.clone(),
                };

                self.clone_repository(&remote_repository_with_ref_name_if_any)
                    .await
            }
        }
    }

    /// Clone a repository from GitHub
    ///
    /// Creates a directory and performs a shallow clone of the specified repository.
    /// Uses a structured RemoteGitRepositoryInfo object to encapsulate all required clone parameters.
    ///
    /// # Parameters
    ///
    /// * `repo_dir` - The directory where the repository should be cloned
    /// * `params` - RemoteGitRepositoryInfo struct containing user, repo, and ref_name
    ///
    /// # Returns
    ///
    /// * `Result<(), String>` - Success or an error message if the clone operation fails
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::GitRemoteRepositoryInfo;
    ///
    /// async fn example() {
    ///     let repo_dir = PathBuf::from("/tmp/example_repo");
    ///     let params = GitRemoteRepositoryInfo {
    ///         user: "rust-lang".to_string(),
    ///         repo: "rust".to_string(),
    ///         ref_name: Some("main".to_string()),
    ///     };
    ///
    ///     // Example code has been updated to match the current API
    ///     println!("Repository cloned successfully with params: {:?}", params);
    /// }
    /// ```
    async fn clone_repository(
        &self,
        remote_repository: &GitRemoteRepository,
    ) -> Result<LocalRepository, String> {
        // Create a unique local repository directory based on the remote repository info
        // Include the process_id to differentiate between multiple processes
        let local_repo = LocalRepository::new_local_repository_to_clone(
            match remote_repository {
                GitRemoteRepository::Github(github_info) => github_info.repo_info.clone(),
            },
            Some(&self.process_id),
        );

        // Ensure the destination directory doesn't exist already
        let repo_dir = local_repo.get_repository_dir();
        if repo_dir.exists() {
            if repo_dir.is_dir() {
                // Repository already exists, let's validate it
                match local_repo.validate() {
                    Ok(_) => {
                        tracing::info!(
                            "Repository already exists at {}, reusing it",
                            repo_dir.display()
                        );
                        return Ok(local_repo);
                    }
                    Err(e) => {
                        // Directory exists but is not a valid repository, clean it up
                        tracing::warn!(
                            "Found invalid repository at {}, removing it: {}",
                            repo_dir.display(),
                            e
                        );
                        if let Err(e) = std::fs::remove_dir_all(repo_dir) {
                            return Err(format!(
                                "Failed to remove invalid repository directory: {}",
                                e
                            ));
                        }
                    }
                }
            } else {
                return Err(format!(
                    "Destination path exists but is not a directory: {}",
                    repo_dir.display()
                ));
            }
        }

        // Create parent directory if it doesn't exist
        if let Some(parent) = repo_dir.parent() {
            if !parent.exists() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return Err(format!("Failed to create parent directories: {}", e));
                }
            }
        }

        // For GitHub repositories, we try HTTPS first (with token auth if available),
        // and only fall back to SSH if HTTPS fails. This supports users who authenticate
        // via gh auth / git credential helper instead of SSH keys.
        let clone_url = remote_repository.clone_url();
        let ref_name = remote_repository.get_ref_name();

        tracing::info!(
            "Cloning repository from {} to {}{}",
            clone_url,
            repo_dir.display(),
            ref_name
                .as_ref()
                .map(|r| format!(" (ref: {})", r))
                .unwrap_or_default()
        );

        // Build the list of URLs to try in order of preference
        let urls_to_try = self.build_clone_urls(remote_repository);

        // Try each URL in order until one succeeds
        let mut last_error: Option<String> = None;

        for (url_index, (url, url_description)) in urls_to_try.iter().enumerate() {
            // Log URL (redact token if present)
            let log_url = Self::redact_token_from_url(url);
            tracing::info!(
                "Attempt {}/{}: Trying {} ({})",
                url_index + 1,
                urls_to_try.len(),
                url_description,
                log_url
            );

            // Clean up any partial clone directory from previous attempts
            if repo_dir.exists() {
                let _ = std::fs::remove_dir_all(repo_dir);
            }

            // Try to clone with this URL
            match self
                .try_clone_with_url(url, repo_dir, ref_name.as_deref())
                .await
            {
                Ok(()) => {
                    tracing::info!(
                        "Successfully cloned repository to {} using {}",
                        repo_dir.display(),
                        url_description
                    );
                    return Ok(local_repo);
                }
                Err(e) => {
                    tracing::warn!("{} failed: {}", url_description, e);
                    last_error = Some(e);
                    // Continue to next URL
                }
            }
        }

        // All URLs failed - clean up and return error
        if repo_dir.exists() {
            let _ = std::fs::remove_dir_all(repo_dir);
        }

        let error_msg = last_error.unwrap_or_else(|| "No URLs available to try".to_string());
        Err(format!(
            "Failed to clone repository: All URL formats failed.\n\nLast error: {}\n\n\
            Suggestion: Tried {} URL format(s). If you're using GitHub:\n  \
            - For HTTPS with gh auth / credential helper, ensure 'gh auth status' shows you're logged in\n  \
            - For SSH, ensure your SSH keys are properly set up: 'ssh -T git@github.com'\n  \
            - You can also try providing a GitHub token via GITHUB_TOKEN environment variable",
            error_msg,
            urls_to_try.len()
        ))
    }

    /// Build a list of URLs to try for cloning, in order of preference
    ///
    /// For GitHub HTTPS URLs, we try:
    /// 1. HTTPS with .git suffix (and token if available) - works with credential helpers
    /// 2. HTTPS without .git suffix (and token if available) - alternative format
    /// 3. SSH URL - fallback for users with SSH keys configured
    ///
    /// For SSH URLs or non-GitHub URLs, we just use the original URL.
    fn build_clone_urls(&self, remote_repository: &GitRemoteRepository) -> Vec<(String, String)> {
        let clone_url = remote_repository.clone_url();
        let mut urls = Vec::new();

        if clone_url.starts_with("https://github.com") {
            // Extract the path from the URL
            let github_path = clone_url
                .trim_start_matches("https://github.com/")
                .trim_start_matches("https://github.com")
                .trim_start_matches('/')
                .trim_end_matches('/')
                .trim_end_matches(".git");

            // For public repositories, try unauthenticated HTTPS first since
            // embedding tokens in URLs can cause issues with URL encoding.
            // For private repos, users should use SSH or git credential helpers.

            // 1. HTTPS without embedded token (works for public repos and with credential helpers)
            let https_with_git = format!("https://github.com/{}.git", github_path);
            urls.push((https_with_git, "HTTPS with .git suffix".to_string()));

            // 2. HTTPS without .git suffix (alternative format)
            let https_without_git = format!("https://github.com/{}", github_path);
            urls.push((https_without_git, "HTTPS without .git suffix".to_string()));

            // 3. SSH URL (fallback for users with SSH keys, also works for private repos)
            let ssh_url = match remote_repository {
                GitRemoteRepository::Github(github_info) => github_info.to_ssh_url(),
            };
            urls.push((ssh_url, "SSH".to_string()));

            // 4. HTTPS with embedded token (last resort for private repos without SSH)
            // Note: URL encoding issues with some HTTP backends may cause this to fail
            if let Some(token) = &self.github_token {
                urls.push((
                    format!(
                        "https://{}:x-oauth-basic@github.com/{}.git",
                        token, github_path
                    ),
                    "HTTPS with embedded token".to_string(),
                ));
            }
        } else if clone_url.starts_with("git@") {
            // SSH URL - try it directly, then try HTTPS as fallback
            urls.push((clone_url.clone(), "SSH (original)".to_string()));

            // For SSH URLs, also try HTTPS as fallback
            let GitRemoteRepository::Github(github_info) = remote_repository;
            let https_url = format!(
                "https://github.com/{}/{}.git",
                github_info.repo_info.user, github_info.repo_info.repo
            );
            if let Some(token) = &self.github_token {
                urls.push((
                    format!(
                        "https://{}:x-oauth-basic@github.com/{}/{}.git",
                        token, github_info.repo_info.user, github_info.repo_info.repo
                    ),
                    "HTTPS with token (fallback)".to_string(),
                ));
            } else {
                urls.push((https_url, "HTTPS (fallback)".to_string()));
            }
        } else {
            // Non-GitHub URL - just use as-is
            urls.push((clone_url, "Original URL".to_string()));
        }

        urls
    }

    /// Redact token from URL for logging
    fn redact_token_from_url(url: &str) -> String {
        if url.contains('@') && url.contains("x-oauth-basic") {
            let parts: Vec<&str> = url.splitn(2, '@').collect();
            if parts.len() == 2 {
                return format!("https://[REDACTED]@{}", parts[1]);
            }
        }
        url.to_string()
    }

    /// Try to clone a repository with a specific URL
    ///
    /// Returns Ok(()) if successful, Err with error message if failed.
    async fn try_clone_with_url(
        &self,
        url: &str,
        repo_dir: &std::path::Path,
        ref_name: Option<&str>,
    ) -> Result<(), String> {
        use gix::clone::PrepareFetch;
        use gix::create::Kind;
        use gix::open::Options as OpenOptions;

        // Initialize repository creation
        let fetch_result = PrepareFetch::new(
            url,
            repo_dir,
            Kind::WithWorktree,
            gix::create::Options::default(),
            OpenOptions::default(),
        );

        let mut fetch = match fetch_result {
            Ok(fetch) => fetch,
            Err(e) => return Err(format!("Failed to prepare fetch: {}", e)),
        };

        // Configure HTTP settings for HTTPS URLs
        if url.starts_with("https://") {
            fetch = fetch.configure_remote(|remote| {
                Ok(remote.with_fetch_tags(gix::remote::fetch::Tags::All))
            });

            fetch = fetch.with_in_memory_config_overrides([
                "http.followRedirects=true",
                "http.lowSpeedLimit=1000",
                "http.lowSpeedTime=30",
            ]);
        }

        // Configure the reference to fetch if specified
        if let Some(ref_name) = ref_name {
            fetch = match fetch.with_ref_name(Some(ref_name)) {
                Ok(f) => f,
                Err(e) => return Err(format!("Invalid reference name: {}", e)),
            };
        }

        // Set up shallow clone
        let depth = NonZeroU32::new(1).unwrap();
        fetch = fetch.with_shallow(Shallow::DepthAtRemote(depth));

        // Perform the actual clone
        match fetch.fetch_then_checkout(&mut Discard, &gix::interrupt::IS_INTERRUPTED) {
            Ok((mut checkout, _fetch_outcome)) => {
                // Finalize the checkout process
                match checkout.main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED) {
                    Ok((_repo, _outcome)) => Ok(()),
                    Err(e) => {
                        let error_message = if e.to_string().contains("reference")
                            || e.to_string().contains("ref")
                        {
                            format!(
                                "Checkout failed: {}. The specified branch or tag may not exist",
                                e
                            )
                        } else {
                            format!("Checkout failed: {}", e)
                        };
                        Err(error_message)
                    }
                }
            }
            Err(e) => {
                let error_details = format!("{}", e);
                if error_details.contains("I/O error")
                    || error_details.contains("io error")
                    || error_details.contains("talking to the server")
                {
                    Err(format!("Network error: {}", e))
                } else if error_details.contains("authentication")
                    || error_details.contains("credential")
                    || error_details.contains("unauthorized")
                    || error_details.contains("permission")
                {
                    Err(format!("Authentication error: {}", e))
                } else if error_details.contains("redirect")
                    || error_details.contains("301")
                    || error_details.contains("302")
                {
                    Err(format!("Redirect error: {}", e))
                } else {
                    Err(format!("Clone error: {}", e))
                }
            }
        }
    }

    /// Returns a GitHub API client instance
    ///
    /// Creates a new GitHub client with the manager's authentication token
    /// for interacting with the GitHub API.
    ///
    /// # Returns
    ///
    /// A GitHub client instance configured with the manager's authentication token
    fn get_github_client(&self) -> Result<providers::github::GithubClient, String> {
        providers::github::GithubClient::new(self.github_token.clone())
    }

    /// Lists all references (branches and tags) for a given repository using the GitHub API
    ///
    /// This method handles the entire refs listing process:
    /// 1. Parses a repository location string into a RepositoryLocation
    /// 2. For GitHub repositories, uses the GitHub API to fetch refs
    /// 3. For local repositories:
    ///    a. Prepares the repository using the repository manager
    ///    b. Fetches the latest updates from remote
    ///    c. Lists refs from the local repository
    ///
    /// # Parameters
    ///
    /// * `repository_location_str` - The repository location string to parse (e.g., "github:user/repo" or "/path/to/local/repo")
    ///
    /// # Returns
    ///
    /// * `Result<(String, Option<LocalRepository>), String>` - A tuple containing the JSON results string and optionally a local repository reference
    ///
    /// # Errors
    ///
    /// This method returns an error if:
    /// - The repository location string cannot be parsed
    /// - The repository cannot be accessed or prepared
    /// - The API request fails (for GitHub repositories)
    /// - The git command fails (for local repositories)
    pub async fn list_repository_refs(
        &self,
        repository_location_str: &str,
    ) -> Result<(providers::RepositoryRefs, Option<LocalRepository>), String> {
        // Parse the repository location string
        let repository_location = RepositoryLocation::from_str(repository_location_str)
            .map_err(|e| format!("Failed to parse repository location: {}", e))?;

        // Different handling based on repository type
        match &repository_location {
            RepositoryLocation::RemoteRepository(remote_repo) => {
                // Currently only GitHub repositories are supported
                match remote_repo {
                    GitRemoteRepository::Github(github_repo_info) => {
                        // For GitHub repositories, use the GitHub API
                        let github_client = self.get_github_client()?;
                        let refs = github_client
                            .list_repository_refs(&github_repo_info.repo_info)
                            .await?;

                        // Return the structured refs result without a local repository reference
                        Ok((refs, None))
                    }
                }
            }
            local_repository @ RepositoryLocation::LocalPath(_) => {
                // For local repositories, prepare the repository and use git commands
                let local_repo = self.prepare_repository(local_repository, None).await?;

                // Fetch updates from remote before listing refs to ensure we have the latest changes
                // Ignore fetch errors as we can still list existing refs even if fetch fails
                if let Err(e) = local_repo.fetch_remote().await {
                    eprintln!("Warning: Failed to fetch latest updates from remote: {}", e);
                    // Continue with listing refs despite fetch failure
                }

                // Use the local repository to list refs
                let refs = local_repo.list_repository_refs().await?;

                // Return both the structured refs and the local repository reference
                Ok((refs, Some(local_repo)))
            }
        }
    }

    /// Search for repositories across different Git providers
    ///
    /// This method performs a search for repositories on the specified Git provider
    /// based on the provided query and search parameters. It abstracts the provider-specific
    /// implementation details and provides a unified interface for searching repositories.
    ///
    /// # Parameters
    ///
    /// * `provider` - The Git provider to search (currently only GitHub is supported)
    /// * `query` - The search query string
    /// * `sort_by` - Optional sort option for results
    /// * `order` - Optional sort direction
    /// * `per_page` - Optional number of results per page (1-100)
    /// * `page` - Optional page number
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - JSON string containing search results or an error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
    /// use gitcodes_mcp::gitcodes::repository_manager::{SortOption, OrderOption};
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;
    ///
    /// async fn example() {
    ///     let repo_manager = RepositoryManager::default();
    ///
    ///     // Basic search with minimal parameters
    ///     match repo_manager.search_repositories(
    ///         GitProvider::Github,
    ///         "rust http client".to_string(),
    ///         None,
    ///         None,
    ///         None,
    ///         None
    ///     ).await {
    ///         Ok(results) => println!("Found repositories: {:?}", results),
    ///         Err(e) => eprintln!("Search failed: {}", e),
    ///     }
    ///
    ///     // Search with all parameters
    ///     match repo_manager.search_repositories(
    ///         GitProvider::Github,
    ///         "language:rust stars:>1000".to_string(),
    ///         Some(SortOption::Stars),    // Use enum directly from this module
    ///         Some(OrderOption::Descending),     // Use enum directly from this module
    ///         Some(50),
    ///         Some(1)
    ///     ).await {
    ///         Ok(results) => println!("Found top Rust repositories: {:?}", results),
    ///         Err(e) => eprintln!("Search failed: {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// # Authentication
    ///
    /// Uses the provider-specific token configured in the RepositoryManager instance.
    /// Authentication increases rate limits and enables access to private repositories.
    pub async fn search_repositories(
        &self,
        provider: providers::models::GitProvider,
        query: String,
        sort_option: Option<SortOption>, // Generic sort option from this module
        order_option: Option<OrderOption>, // Generic order option from this module
        per_page: Option<u8>,
        page: Option<u32>,
    ) -> Result<providers::RepositorySearchResults, String> {
        match provider {
            providers::models::GitProvider::Github => {
                // Convert generic SortOption to GitHub-specific GithubSortOption
                let sort_by = sort_option.map(providers::github::GithubSortOption::from);

                // Convert generic OrderOption to GitHub-specific GithubOrderOption
                let order = order_option.map(providers::github::GithubOrderOption::from);

                // Create GitHub search parameters
                let params = providers::github::GithubSearchParams {
                    query,
                    sort_by,
                    order,
                    per_page,
                    page,
                };

                // Use the GitHub client to perform the search
                self.search_github_repositories(params).await
            } // Add more provider implementations here in the future
        }
    }

    /// Search for GitHub repositories matching the specified query
    ///
    /// This method performs a search for repositories on GitHub based on the provided
    /// search parameters. It handles authentication and API communication internally.
    ///
    /// # Parameters
    ///
    /// * `params` - GitHub search parameters including query, sort options, and pagination
    ///
    /// # Returns
    ///
    /// * `Result<String, String>` - JSON string containing search results or an error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
    /// use gitcodes_mcp::gitcodes::repository_manager::{SortOption, OrderOption};
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;
    ///
    /// async fn example() {
    ///     let repo_manager = RepositoryManager::default();
    ///
    ///     // Search using the public method
    ///     match repo_manager.search_repositories(
    ///         GitProvider::Github,
    ///         "rust http client".to_string(),
    ///         Some(SortOption::Stars),    // Use enum from this module
    ///         Some(OrderOption::Descending),     // Use enum from this module
    ///         Some(10),
    ///         Some(1)
    ///     ).await {
    ///         Ok(results) => println!("Found repositories: {:?}", results),
    ///         Err(e) => eprintln!("Search failed: {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// # Authentication
    ///
    /// Uses the GitHub token configured in the RepositoryManager instance.
    /// Without a token, limited to 60 requests/hour.
    /// With a token, allows 5,000 requests/hour.
    async fn search_github_repositories(
        &self,
        params: providers::github::GithubSearchParams,
    ) -> Result<providers::RepositorySearchResults, String> {
        // Get a GitHub client instance
        let github_client = self.get_github_client()?;

        // Execute the search and return the results
        github_client.search_repositories(params).await
    }

    /// Search for issues across different Git providers
    ///
    /// This method performs a search for issues on the specified Git provider
    /// based on the provided query and search parameters. It abstracts the provider-specific
    /// implementation details and provides a unified interface for searching issues.
    ///
    /// # Parameters
    ///
    /// * `provider` - The Git provider to search (currently only GitHub is supported)
    /// * `query` - The search query string
    /// * `sort_by` - Optional sort option for results
    /// * `order` - Optional sort direction
    /// * `per_page` - Optional number of results per page (1-100)
    /// * `page` - Optional page number
    ///
    /// # Returns
    ///
    /// * `Result<providers::IssueSearchResults, String>` - Issue search results or an error message
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use gitcodes_mcp::gitcodes::repository_manager::RepositoryManager;
    /// use gitcodes_mcp::gitcodes::repository_manager::{IssueSearchParams, IssueSortOption, OrderOption};
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::models::GitProvider;
    ///
    /// async fn example() {
    ///     let repo_manager = RepositoryManager::default();
    ///
    ///     // Basic search with minimal parameters
    ///     let basic_params = IssueSearchParams {
    ///         query: "repo:rust-lang/rust state:open label:bug".to_string(),
    ///         sort_by: None,
    ///         order: None,
    ///         per_page: None,
    ///         page: None,
    ///         repository: None,
    ///         labels: None,
    ///         state: None,
    ///         creator: None,
    ///         mentioned: None,
    ///         assignee: None,
    ///         milestone: None,
    ///         issue_type: None,
    ///     };
    ///
    ///     match repo_manager.search_issues(GitProvider::Github, basic_params).await {
    ///         Ok(results) => println!("Found issues: {:?}", results),
    ///         Err(e) => eprintln!("Search failed: {}", e),
    ///     }
    ///
    ///     // Search with all parameters
    ///     let detailed_params = IssueSearchParams {
    ///         query: "label:enhancement state:open".to_string(),
    ///         sort_by: Some(IssueSortOption::Updated),
    ///         order: Some(OrderOption::Descending),
    ///         per_page: Some(50),
    ///         page: Some(1),
    ///         repository: None,
    ///         labels: None,
    ///         state: None,
    ///         creator: None,
    ///         mentioned: None,
    ///         assignee: None,
    ///         milestone: None,
    ///         issue_type: None,
    ///     };
    ///
    ///     match repo_manager.search_issues(GitProvider::Github, detailed_params).await {
    ///         Ok(results) => println!("Found enhancement issues: {:?}", results),
    ///         Err(e) => eprintln!("Search failed: {}", e),
    ///     }
    /// }
    /// ```
    ///
    /// # Authentication
    ///
    /// Uses the provider-specific token configured in the RepositoryManager instance.
    /// Authentication increases rate limits and enables access to private repositories.
    pub async fn search_issues(
        &self,
        provider: providers::models::GitProvider,
        params: IssueSearchParams,
    ) -> Result<providers::IssueSearchResults, String> {
        match provider {
            providers::models::GitProvider::Github => {
                // Convert generic IssueSortOption to GitHub-specific GithubIssueSortOption
                let sort_by = params
                    .sort_by
                    .map(providers::github::GithubIssueSortOption::from);

                // Convert generic OrderOption to GitHub-specific GithubOrderOption
                let order = params.order.map(providers::github::GithubOrderOption::from);

                // Create GitHub issue search parameters
                let github_params = providers::github::GithubIssueSearchParams {
                    query: params.query,
                    sort_by,
                    order,
                    per_page: params.per_page,
                    page: params.page,
                    repository: params.repository,
                    labels: params.labels,
                    state: params.state,
                    creator: params.creator,
                    mentioned: params.mentioned,
                    assignee: params.assignee,
                    milestone: params.milestone,
                    issue_type: params.issue_type,
                };

                // Use the GitHub client to perform the search
                self.search_github_issues(github_params).await
            }
        }
    }

    /// Search for GitHub issues matching the specified query
    ///
    /// This method performs a search for issues on GitHub based on the provided
    /// search parameters. It handles authentication and API communication internally.
    ///
    /// # Parameters
    ///
    /// * `params` - GitHub issue search parameters including query, sort options, and pagination
    ///
    /// # Returns
    ///
    /// * `Result<providers::IssueSearchResults, String>` - Issue search results or an error message
    ///
    /// # Authentication
    ///
    /// Uses the GitHub token configured in the RepositoryManager instance.
    /// Without a token, limited to 60 requests/hour.
    /// With a token, allows 5,000 requests/hour.
    async fn search_github_issues(
        &self,
        params: providers::github::GithubIssueSearchParams,
    ) -> Result<providers::IssueSearchResults, String> {
        // Get a GitHub client instance
        let github_client = self.get_github_client()?;

        // Execute the search and return the results
        github_client.search_issues(params).await
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
