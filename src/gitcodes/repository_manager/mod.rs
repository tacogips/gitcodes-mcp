mod providers;
mod repository_location;

use std::{num::NonZeroU32, path::PathBuf, sync::atomic::AtomicBool};

use gix::{bstr::ByteSlice, progress::Discard};
use providers::GitRemoteRepository;
use repository_location::RepositoryLocation;

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
            RepositoryLocation::RemoteRepository(remote_repository) => {
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
            // If the repository already exists, just return it
            return Ok(LocalRepository::new(repo_dir));
        }
        
        // Create directory if it doesn't exist (parent directories)
        if let Err(e) = std::fs::create_dir_all(&self.local_repository_cache_dir_base) {
            return Err(format!("Failed to create base repository directory: {}", e));
        }
        
        // Get the clone URL
        let clone_url = remote_repository.clone_url();
        let ref_name = remote_repository.get_ref_name();
        
        // Set up shallow clone depth
        let depth = NonZeroU32::new(1).unwrap();
        
        // Use atomic boolean for interrupt flag (we won't actually interrupt)
        let interrupt_flag = AtomicBool::new(false);
        
        // Create options for clone with shallow depth
        let options = gix::open::Options::isolated();
        
        // Setup fetch options for shallow clone
        let fetch_options = gix::remote::fetch::Options {
            shallow: Some(gix::remote::fetch::Shallow::DepthAtRemote(depth)),
            ..Default::default()
        };
        
        // Configure clone options
        let clone_config = gix::clone::fetch::Configuration {
            fetch_options,
            ..Default::default()
        };
        
        // Create PrepareFetch instance
        let mut prepare = gix::clone::PrepareFetch::new(
            &clone_url,
            &repo_dir,
            gix::create::Kind::WithWorktree,
            clone_config,
            options,
        )
        .map_err(|e| format!("Failed to prepare repository clone: {}", e))?;
        
        // If a specific reference (branch/tag) was provided, set it for checkout
        if let Some(ref_to_checkout) = ref_name {
            prepare = prepare.with_reference_to_checkout(ref_to_checkout);
        }
        
        // Perform fetch and checkout
        let (mut checkout, _output) = prepare
            .fetch_then_checkout(Discard, &interrupt_flag)
            .map_err(|e| format!("Failed to fetch repository: {}", e))?;
        
        // Complete the checkout process to get repository handle
        let (_repo, _stats) = checkout
            .main_worktree(Discard, &interrupt_flag)
            .map_err(|e| format!("Failed to checkout repository: {}", e))?;
        
        // Return a LocalRepository instance pointing to the cloned repository
        Ok(LocalRepository::new(repo_dir))
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
