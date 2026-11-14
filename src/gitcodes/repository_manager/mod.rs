pub mod instance;
pub mod providers;
mod repository_location;

use std::{num::NonZeroU32, path::PathBuf};

use gix::{progress::Discard, remote::fetch::Shallow};
use providers::GitRemoteRepository;
pub use repository_location::RepositoryLocation;
use tracing;

use crate::gitcodes::local_repository::LocalRepository;

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
    /// * `github_token` - Optional GitHub token for authentication
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

        // Get the URL from the remote repository
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

        // Initialize a repo for fetching
        let mut fetch = match PrepareFetch::new(
            auth_url.as_str(),
            repo_dir,
            Kind::WithWorktree, // We want a standard clone with worktree
            gix::create::Options::default(),
            OpenOptions::default(),
        ) {
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
                        Err(format!("Failed to checkout repository: {}", e))
                    }
                }
            }
            Err(e) => {
                // Clean up failed clone attempt
                if repo_dir.exists() {
                    let _ = std::fs::remove_dir_all(repo_dir);
                }
                Err(format!("Failed to clone repository: {}", e))
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
    pub fn get_github_client(&self) -> providers::github::GithubClient {
        let client = reqwest::Client::new();
        providers::github::GithubClient::new(client, self.github_token.clone())
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
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubSortOption, GithubOrderOption};
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
    ///         Ok(results) => println!("Found repositories: {}", results),
    ///         Err(e) => eprintln!("Search failed: {}", e),
    ///     }
    ///
    ///     // Search with all parameters
    ///     match repo_manager.search_repositories(
    ///         GitProvider::Github,
    ///         "language:rust stars:>1000".to_string(),
    ///         Some(GithubSortOption::Stars),
    ///         Some(GithubOrderOption::Descending),
    ///         Some(50),
    ///         Some(1)
    ///     ).await {
    ///         Ok(results) => println!("Found top Rust repositories: {}", results),
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
        provider: providers::GitProvider,
        query: String,
        sort_by: Option<providers::github::GithubSortOption>,
        order: Option<providers::github::GithubOrderOption>,
        per_page: Option<u8>,
        page: Option<u32>,
    ) -> Result<String, String> {
        match provider {
            providers::GitProvider::Github => {
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
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::github::{GithubSearchParams, GithubSortOption, GithubOrderOption};
    ///
    /// async fn example() {
    ///     let repo_manager = RepositoryManager::default();
    ///
    ///     // Basic search with defaults
    ///     let params = GithubSearchParams {
    ///         query: "rust http client".to_string(),
    ///         sort_by: None,
    ///         order: None,
    ///         per_page: None,
    ///         page: None,
    ///     };
    ///
    ///     match repo_manager.search_github_repositories(params).await {
    ///         Ok(results) => println!("Found repositories: {}", results),
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
    ) -> Result<String, String> {
        // Get a GitHub client instance
        let github_client = self.get_github_client();

        // Execute the search and return the results
        github_client.search_repositories(params).await
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
