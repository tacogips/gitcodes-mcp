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
/// Uses a dedicated directory to store cloned repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    pub github_token: Option<String>,
    pub local_repository_cache_dir_base: PathBuf,
}

impl RepositoryManager {
    /// Creates a new RepositoryManager instance with a custom repository cache directory
    ///
    /// # Parameters
    ///
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

        Ok(Self {
            github_token,
            local_repository_cache_dir_base,
        })
    }

    /// Creates a new RepositoryManager with the system's default cache directory
    ///
    /// This is a convenience method that creates a RepositoryManager with the
    /// system's temporary directory as the repository cache location.
    pub fn with_default_cache_dir() -> Self {
        Self::new(None, None).expect("Failed to initialize with system temporary directory")
    }

    pub async fn prepare_repository(
        &self,
        repo_location: RepositoryLocation,
        ref_name: Option<String>,
    ) -> Result<LocalRepository, String> {
        match repo_location {
            RepositoryLocation::LocalPath(local_path) => {
                local_path.validate()?;
                Ok(local_path)
            }
            RepositoryLocation::RemoteRepository(mut remote_repository) => {
                // If a specific ref_name was provided to this function, update the repository info
                if let Some(ref_name_str) = ref_name {
                    // Update the remote repository with the provided ref_name
                    match remote_repository {
                        GitRemoteRepository::Github(ref mut github_info) => {
                            github_info.repo_info.ref_name = Some(ref_name_str);
                        }
                    }
                }

                self.clone_repository(&remote_repository).await
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
    /// use gitcodes_mcp::tools::gitcodes::git_service::git_repository::{clone_repository, RemoteGitRepositoryInfo};
    ///
    /// async fn example() {
    ///     let repo_dir = PathBuf::from("/tmp/example_repo");
    ///     let params = RemoteGitRepositoryInfo {
    ///         user: "rust-lang".to_string(),
    ///         repo: "rust".to_string(),
    ///         ref_name: "main".to_string(),
    ///     };
    ///
    ///     match clone_repository(&repo_dir, &params).await {
    ///         Ok(()) => println!("Repository cloned successfully"),
    ///         Err(e) => eprintln!("Failed to clone repository: {}", e),
    ///     }
    /// }
    /// ```
    async fn clone_repository(
        &self,
        remote_repository: &GitRemoteRepository,
    ) -> Result<LocalRepository, String> {
        // Create a unique local repository directory based on the remote repository info
        let local_repo = LocalRepository::new_local_repository_to_clone(match remote_repository {
            GitRemoteRepository::Github(github_info) => github_info.repo_info.clone(),
        });

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
        use gix::create::Kind;
        use gix::open::Options as OpenOptions;
        use gix::clone::PrepareFetch;
        
        // Initialize a repo for fetching
        let mut fetch = match PrepareFetch::new(
            auth_url.as_str(),
            repo_dir,
            Kind::WithWorktree,  // We want a standard clone with worktree
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
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
