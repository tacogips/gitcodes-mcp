mod providers;
mod repository_location;

use std::{num::NonZeroU32, path::PathBuf, sync::atomic::AtomicBool};

use gix::{bstr::ByteSlice, progress::Discard};
use providers::GitRemoteRepository;
use repository_location::RepositoryLocation;
use tracing;

use crate::gitcodes::local_repository::LocalRepository;

/// Repository manager for Git operations
///
/// Handles cloning, updating, and retrieving information from GitHub repositories.
/// Uses a dedicated directory to store cloned repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    github_token: Option<String>,
    pub(crate) local_repository_cache_dir_base: PathBuf,
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
        Self::new(None).expect("Failed to initialize with system temporary directory")
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
        // Generate a unique directory name for this repository
        let repo_name = remote_repository.get_unique_name();
        let repo_dir = self.local_repository_cache_dir_base.join(repo_name);
        
        // Check if repository already exists at this location
        if repo_dir.exists() {
            // If the repository already exists, check if it's valid
            let git_dir = if repo_dir.join(".git").exists() {
                repo_dir.join(".git")
            } else if repo_dir.join("HEAD").exists() {
                // It's a bare repository
                repo_dir.clone()
            } else {
                // Directory exists but doesn't appear to be a git repository
                return Err(format!("Directory exists but doesn't appear to be a valid git repository: {}", repo_dir.display()));
            };
            
            // It's a valid repository, just return it
            tracing::info!("Repository already exists at {}, reusing", repo_dir.display());
            return Ok(LocalRepository::new(repo_dir));
        }
        
        // Create directory if it doesn't exist (parent directories)
        if let Err(e) = std::fs::create_dir_all(&self.local_repository_cache_dir_base) {
            return Err(format!("Failed to create base repository directory: {}", e));
        }
        
        // Get the clone URL and ensure it's a valid Git URL
        let clone_url = remote_repository.clone_url();
        
        // Validate the URL format
        if !clone_url.starts_with("https://") && !clone_url.starts_with("git@") && !clone_url.starts_with("git://") {
            return Err(format!("Invalid Git URL format: {}", clone_url));
        }
        
        let ref_name = remote_repository.get_ref_name();
        
        // Set up shallow clone depth (default to 1)
        let depth = NonZeroU32::new(1).expect("Depth must be at least 1");
        
        // Use atomic boolean for interrupt flag (we won't actually interrupt)
        let interrupt_flag = AtomicBool::new(false);
        
        // Create options for clone with isolated configuration to avoid system config influence
        let mut options = gix::open::Options::isolated();
        
        // Set protocol version to v2 for better performance if available
        options = match options.config_overrides(["protocol.version=2"]) {
            Ok(options) => options,
            Err(e) => {
                // If setting protocol version fails, just continue with default options
                tracing::warn!("Failed to set protocol version: {}", e);
                options
            }
        };
        
        // Setup fetch options for shallow clone
        let mut fetch_options = gix::remote::fetch::Options::default();
        fetch_options.shallow = Some(gix::remote::fetch::Shallow::DepthAtRemote(depth));
        
        // Set tags behavior - fetch everything
        fetch_options.tags = gix::remote::fetch::Tags::All;
        
        // Configure clone options
        let mut clone_config = gix::clone::fetch::Configuration::default();
        clone_config.fetch_options = fetch_options;
        clone_config.remote_name = "origin".to_string(); // Default remote name
        
        // Create PrepareFetch instance
        let mut prepare = gix::clone::PrepareFetch::new(
            clone_url.as_bytes().as_bstr(), // Source URL as bstr
            &repo_dir,                      // Target directory 
            gix::create::Kind::WithWorktree, // Create a repository with worktree
            clone_config,                    // Clone configuration
            options,                         // Repository options
        )
        .map_err(|e| format!("Failed to prepare repository clone: {}", e))?;
        
        // If a specific reference (branch/tag) was provided, set it for checkout
        if let Some(ref_to_checkout) = ref_name {
            // Format the reference name correctly - try to account for various formats
            let formatted_ref = if ref_to_checkout.starts_with("refs/") {
                // It's already a fully qualified reference
                ref_to_checkout
            } else if ref_to_checkout.starts_with("heads/") || 
                      ref_to_checkout.starts_with("tags/") {
                // It's a partial reference path, make it fully qualified
                format!("refs/{}", ref_to_checkout)
            } else {
                // Assume it's a branch name, create a fully qualified branch reference
                format!("refs/heads/{}", ref_to_checkout)
            };
            
            prepare = prepare.with_reference_to_checkout(formatted_ref);
            tracing::info!("Setting checkout reference to: {}", formatted_ref);
        }
        
        // Perform fetch and checkout
        let (mut checkout, output) = prepare
            .fetch_then_checkout(Discard, &interrupt_flag)
            .map_err(|e| {
                // Clean up any partially created directories
                let _ = std::fs::remove_dir_all(&repo_dir);
                format!("Failed to fetch repository: {}", e)
            })?;
        
        // Complete the checkout process to get repository handle
        let (repo, stats) = checkout
            .main_worktree(Discard, &interrupt_flag)
            .map_err(|e| {
                // Clean up any partially created directories
                let _ = std::fs::remove_dir_all(&repo_dir);
                format!("Failed to checkout repository: {}", e)
            })?;
        
        // Verify that the repository is valid
        if !repo_dir.join(".git").exists() && !repo_dir.join("HEAD").exists() {
            return Err(format!("Repository clone appears to be incomplete at {}", repo_dir.display()));
        }
        
        // Return a LocalRepository instance pointing to the cloned repository
        Ok(LocalRepository::new(repo_dir))
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
