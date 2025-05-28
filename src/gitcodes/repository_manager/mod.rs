pub mod instance;
pub mod providers;
mod repository_location;

use std::{num::NonZeroU32, path::PathBuf, str::FromStr};

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

    /// Use legacy REST API instead of GraphQL
    /// When true, forces the use of REST API instead of the default GraphQL
    pub legacy: Option<bool>,

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
    ///                    to read from the GITCODES_MCP_GITHUB_TOKEN environment variable.
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
        // If no github_token is provided, check environment variable
        let github_token = github_token.or_else(|| std::env::var("GITCODES_MCP_GITHUB_TOKEN").ok());
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

        // For GitHub repositories with HTTPS URLs, use SSH format to avoid HTTP redirect issues with gitoxide
        // Get the appropriate URL based on the repository type
        let clone_url = if remote_repository
            .clone_url()
            .starts_with("https://github.com")
        {
            // Use SSH URL format for GitHub HTTPS URLs to avoid redirect issues
            let original_url = remote_repository.clone_url();
            let ssh_url = remote_repository.to_ssh_url();
            tracing::info!(
                "Converting GitHub HTTPS URL '{}' to SSH format '{}' to avoid HTTP redirect issues",
                original_url,
                ssh_url
            );
            tracing::debug!("DEBUG: This message will only show with --debug flag");
            ssh_url
        } else {
            // For non-GitHub or already SSH URLs, use the original URL
            remote_repository.clone_url()
        };

        tracing::info!("Using clone URL: {}", clone_url);
        let ref_name = remote_repository.get_ref_name();
        tracing::info!("Reference name: {:?}", ref_name);

        tracing::info!(
            "Cloning repository from {} to {}{}",
            clone_url,
            repo_dir.display(),
            ref_name
                .as_ref()
                .map(|r| format!(" (ref: {})", r))
                .unwrap_or_default()
        );

        // Setup authentication for GitHub if token is available
        let mut auth_url = clone_url.clone();
        if let Some(token) = &self.github_token {
            // Add token to URL for authentication if it's a GitHub HTTPS URL
            if clone_url.starts_with("https://github.com") {
                auth_url = format!(
                    "https://{}:x-oauth-basic@{}",
                    token,
                    clone_url.trim_start_matches("https://")
                );
            }
        }

        // Initialize repository creation options
        use gix::clone::PrepareFetch;
        use gix::create::Kind;
        use gix::open::Options as OpenOptions;

        // Format URL - ensure the URL is in a format gitoxide can handle
        let normalized_url = if auth_url.starts_with("https://github.com") {
            // For GitHub HTTPS URLs, make sure we have a proper path format
            // and ensure the URL doesn't end with .git (GitHub API handles this automatically)

            // Handle both formats: with and without trailing slash
            let github_path = if auth_url.starts_with("https://github.com/") {
                auth_url.trim_start_matches("https://github.com/")
            } else {
                auth_url.trim_start_matches("https://github.com")
            };

            // Remove .git suffix if present (GitHub handles this automatically but it causes redirect issues with gitoxide)
            let github_path = github_path.trim_end_matches(".git");

            // Ensure path doesn't start with a slash (could happen if URL was https://github.com/user)
            let github_path = github_path.trim_start_matches('/');

            // Further normalize by ensuring there are no trailing slashes
            let github_path = github_path.trim_end_matches('/');

            if let Some(token) = &self.github_token {
                format!("https://{}:x-oauth-basic@github.com/{}", token, github_path)
            } else {
                format!("https://github.com/{}", github_path)
            }
        } else {
            auth_url
        };

        // Log the normalized URL with any sensitive information redacted
        let log_url = if normalized_url.contains('@') {
            // Redact authentication tokens in logs
            let parts: Vec<&str> = normalized_url.splitn(2, '@').collect();
            if parts.len() == 2 {
                format!("https://[REDACTED]@{}", parts[1])
            } else {
                "[REDACTED URL]".to_string()
            }
        } else {
            normalized_url.clone()
        };

        tracing::info!("Using normalized URL: {}", log_url);

        // Initialize repository creation options
        let mut fetch_result = PrepareFetch::new(
            normalized_url.as_str(),
            repo_dir.clone(),
            Kind::WithWorktree, // We want a standard clone with worktree
            gix::create::Options::default(),
            OpenOptions::default(),
        );

        // Configure HTTP redirect handling if it's a PrepareFetch instance
        if let Ok(mut prepare_fetch) = fetch_result {
            // Add custom configuration for HTTP URL handling
            tracing::info!("Configuring HTTP redirect handling for PrepareFetch");

            // Configure the remote to follow redirects and fetch all tags
            prepare_fetch = prepare_fetch.configure_remote(|remote| {
                tracing::info!("Configuring remote to follow redirects");

                // Make it follow all redirects
                let remote = remote.with_fetch_tags(gix::remote::fetch::Tags::All);

                // Return the modified remote
                Ok(remote)
            });

            // Basic configuration for all URLs
            prepare_fetch = prepare_fetch.with_in_memory_config_overrides([
                "http.followRedirects=true",
                "http.lowSpeedLimit=1000",
                "http.lowSpeedTime=30",
            ]);

            /* NOTE FOR FUTURE DEVELOPERS:
             * The code below has comprehensive HTTP redirect handling for HTTPS GitHub URLs.
             * This was previously used to try to solve the HTTPS URL redirect issue with gitoxide,
             * but we've since switched to automatically converting GitHub HTTPS URLs to SSH format.
             *
             * If you want to implement proper HTTPS URL handling in the future when gitoxide's
             * HTTP redirect handling is improved, this configuration can be a starting point:
             *
             * if url.starts_with("https://") {
             *     // Try a more comprehensive set of HTTP configuration options
             *     prepare_fetch = prepare_fetch.with_in_memory_config_overrides([
             *         // Set followRedirects=true to be parsed as FollowRedirects::All
             *         "http.followRedirects=true",
             *
             *         // Alternative notation as a backup
             *         "http.followRedirects=all",
             *
             *         // Add several performance settings to improve reliability
             *         "http.lowSpeedLimit=1000",      // Increase timeout threshold
             *         "http.lowSpeedTime=30",        // Wait longer for slow connections
             *         "http.maxRequests=5",          // Allow multiple simultaneous connections
             *
             *         // Set user agent to mimic a standard git client
             *         "http.userAgent=git/2.37.0",
             *
             *         // Enable verbose HTTP logging for debugging
             *         "http.curlVerbose=true",
             *
             *         // Add extra networking timeouts
             *         "http.connectTimeout=30",
             *         "core.askPass=",               // Disable credential prompting
             *     ]);
             * }
             *
             * See https://github.com/GitoxideLabs/gitoxide/issues/974 for updates on the gitoxide HTTP redirect issue.
             */

            // Reassign to fetch_result
            fetch_result = Ok(prepare_fetch);
            tracing::info!("Successfully configured HTTP redirect handling");
        } else {
            tracing::warn!(
                "Failed to configure HTTP redirect handling: {:?}",
                fetch_result.as_ref().err()
            );
        }

        // If HTTPS URL fails, try the direct-append approach first before falling back to SSH
        if fetch_result.is_err() && clone_url.starts_with("https://github.com") {
            let fetch_error = fetch_result.err();
            tracing::warn!("HTTPS clone failed with error: {:?}", fetch_error);
            tracing::warn!(
                "**** FALLBACK MECHANISM TRIGGERED - First attempting .git suffix approach ****"
            );

            // First attempt alternative: Try using the HTTPS URL with .git explicitly appended
            // This works around some redirect issues by bypassing GitHub's redirect to the canonical URL
            tracing::info!("Attempting alternative HTTPS URL format with explicit .git suffix");

            // Clean up any partial clone directory
            if repo_dir.exists() {
                let _ = std::fs::remove_dir_all(repo_dir);
            }

            // Construct a URL with explicit .git suffix
            let github_path = if clone_url.starts_with("https://github.com/") {
                clone_url.trim_start_matches("https://github.com/")
            } else {
                clone_url.trim_start_matches("https://github.com")
            };

            // Normalize path and ensure it has .git suffix
            let github_path = github_path.trim_end_matches(".git").trim_start_matches('/');
            let explicit_git_url = format!("https://github.com/{}.git", github_path);

            tracing::info!(
                "Trying HTTPS URL with explicit .git suffix: {}",
                explicit_git_url
            );

            // Try with explicit .git suffix
            fetch_result = PrepareFetch::new(
                explicit_git_url.as_str(),
                repo_dir.clone(),
                Kind::WithWorktree,
                gix::create::Options::default(),
                OpenOptions::default(),
            );

            // Configure this attempt with explicit URL formatting
            if let Ok(mut prepare_fetch) = fetch_result {
                prepare_fetch = prepare_fetch.with_in_memory_config_overrides([
                    "http.followRedirects=all", // Alternative string format
                    "http.lowSpeedLimit=1000",
                    "http.lowSpeedTime=30",
                    "http.curlVerbose=true", // Enable verbose HTTP logging
                ]);

                fetch_result = Ok(prepare_fetch);
            }

            // If that fails too, fall back to SSH
            if fetch_result.is_err() {
                tracing::warn!("Alternative HTTPS approach failed, falling back to SSH URL format");
                tracing::warn!("**** FALLBACK MECHANISM - Second stage: SSH URL fallback ****");

                // Clean up any partial clone directory
                if repo_dir.exists() {
                    let _ = std::fs::remove_dir_all(repo_dir);
                }

                // Convert to SSH URL format which is more reliable with gitoxide
                // Use the to_ssh_url method from GitHub info
                // We already know this is a GitHub repository from the URL check
                let ssh_url = match remote_repository {
                    GitRemoteRepository::Github(github_info) => github_info.to_ssh_url(),
                };

                tracing::info!("Trying SSH URL: {}", ssh_url);

                // Try again with SSH URL
                fetch_result = PrepareFetch::new(
                    ssh_url.as_str(),
                    repo_dir.clone(),
                    Kind::WithWorktree,
                    gix::create::Options::default(),
                    OpenOptions::default(),
                );

                if fetch_result.is_err() {
                    tracing::warn!(
                        "Both HTTPS approaches and SSH URL format failed to clone the repository"
                    );

                    // Clean up any partial clone directory
                    if repo_dir.exists() {
                        let _ = std::fs::remove_dir_all(repo_dir);
                    }

                    // Return error indicating that all approaches failed
                    return Err("Failed to clone repository: All URL formats failed (HTTPS, HTTPS with .git, and SSH)".to_string());
                }
            }
        }

        // Handle the result of our fetch preparation (either initial or fallback)
        let mut fetch = match fetch_result {
            Ok(fetch) => fetch,
            Err(e) => return Err(format!("Failed to prepare repository for fetching: {}", e)),
        };

        // Configure the reference to fetch if specified
        if let Some(ref_name) = ref_name {
            fetch = match fetch.with_ref_name(Some(&ref_name)) {
                Ok(f) => f,
                Err(e) => return Err(format!("Invalid reference name: {}", e)),
            };
        }

        // Set up shallow clone
        let depth = NonZeroU32::new(1).unwrap();
        fetch = fetch.with_shallow(Shallow::DepthAtRemote(depth));

        // Clone the repository
        match fetch.fetch_then_checkout(&mut Discard, &gix::interrupt::IS_INTERRUPTED) {
            Ok((mut checkout, _fetch_outcome)) => {
                // Finalize the checkout process
                match checkout.main_worktree(Discard, &gix::interrupt::IS_INTERRUPTED) {
                    Ok((_repo, _outcome)) => {
                        tracing::info!("Successfully cloned repository to {}", repo_dir.display());
                        Ok(local_repo)
                    }
                    Err(e) => {
                        // Clean up failed checkout attempt
                        if repo_dir.exists() {
                            let _ = std::fs::remove_dir_all(repo_dir);
                        }

                        // Provide more descriptive error for checkout failures
                        let error_message = if e.to_string().contains("reference")
                            || e.to_string().contains("ref")
                        {
                            format!("Failed to checkout repository: {}. The specified branch or tag may not exist", e)
                        } else {
                            format!("Failed to checkout repository: {}", e)
                        };

                        Err(error_message)
                    }
                }
            }
            Err(e) => {
                // Clean up failed clone attempt
                if repo_dir.exists() {
                    let _ = std::fs::remove_dir_all(repo_dir);
                }

                // Provide more specific error messages based on error type
                let error_details = format!("{}", e);
                let error_message = if error_details.contains("I/O error")
                    || error_details.contains("io error")
                    || error_details.contains("talking to the server")
                {
                    if clone_url.starts_with("https://github.com") {
                        format!(
                            "Failed to clone repository via HTTPS: {}\n\nSuggestion: There was an issue cloning via HTTPS. The system attempted to use SSH URL format as a fallback, but that also failed. Try the following:\n  - For GitHub URLs, try the format: 'https://github.com/user/repo' (without .git suffix)\n  - Or try with explicit .git suffix: 'https://github.com/user/repo.git'\n  - As an alternative, use SSH URL format directly: 'git@github.com:user/repo.git'\n  - Check your network connection or firewall settings\n  - If you're behind a proxy, ensure it's properly configured\n  - Make sure your SSH keys are set up properly for GitHub",
                            e
                        )
                    } else {
                        format!("Failed to clone repository (network error): {}\n\nSuggestion: Check your network connection and verify the repository URL is correct.", e)
                    }
                } else if error_details.contains("authentication")
                    || error_details.contains("credential")
                    || error_details.contains("unauthorized")
                    || error_details.contains("permission")
                {
                    format!(
                        "Failed to clone repository (authentication error): {}\n\nSuggestion: Authentication failed. The system attempted to use both HTTPS and SSH URL formats. Try the following:\n  - Ensure you've provided a valid GitHub token if this is a private repository\n  - For public repositories, verify your SSH keys are properly set up for GitHub access\n  - If using HTTPS, check if your token has the correct permissions\n  - Try using the SSH URL format directly: 'git@github.com:user/repo.git'",
                        e
                    )
                } else if error_details.contains("redirect")
                    || error_details.contains("301")
                    || error_details.contains("302")
                {
                    format!(
                        "Failed to clone repository (redirect error): {}\n\nSuggestion: GitHub uses redirects for repository URLs, which caused an issue. The system attempted to use SSH URL format as a fallback, but that also failed. Try the following:\n  - We've already attempted multiple URL formats (with/without .git suffix and SSH)\n  - Use the SSH URL format directly: 'git@github.com:user/repo.git'\n  - This is a known issue with gitoxide (gix) when handling GitHub HTTPS redirects\n  - Verify your SSH keys are properly set up for GitHub access\n  - Check issue #974 in the gitoxide repository for updates: https://github.com/GitoxideLabs/gitoxide/issues/974",
                        e
                    )
                } else {
                    format!("Failed to clone repository: {}\n\nSuggestion: The system automatically tried multiple URL formats including SSH format as a fallback, but all attempts failed. Try the following:\n  - Use the SSH URL format directly: 'git@github.com:user/repo.git'\n  - Verify your SSH keys are properly set up for GitHub access\n  - If you need to use HTTPS, check if there are updates to gitoxide that might fix GitHub HTTPS URL handling\n  - For more details on the gitoxide HTTPS issue, see: https://github.com/GitoxideLabs/gitoxide/issues/974", e)
                };

                Err(error_message)
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
    ///         legacy: None,
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
    ///         legacy: None,
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
                    advanced_search: if params.legacy.unwrap_or(false) {
                        Some(false)
                    } else {
                        None
                    },
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
