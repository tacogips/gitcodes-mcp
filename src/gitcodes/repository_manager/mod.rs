mod providers;
mod repository_location;

use std::{num::NonZeroU32, path::PathBuf};

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
                Ok(local_path.clone())
            }
            RepositoryLocation::RemoteRepository(remote_repository) => match remote_repository {
                github_repo @ GitRemoteRepository::Github(remote_info) => {
                    self.clone_repository(&github_repo)
                }
            },
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
        let repo_dir = self.local_repository_cache_dir_base;
        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(repo_dir) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Build the clone URL
        let clone_url = remote_repository.clone_url();

        // Create a repository using gitoxide
        // First convert to byte slice for parsing
        let url_bstr = clone_url.as_bytes().as_bstr();

        // Parse the URL using gix's URL parser
        let url = gix::url::parse(url_bstr).map_err(|e| format!("Failed to parse URL: {}", e))?;

        // Create options for the clone operation
        let mut clone_options = gix::clone::PrepareFetch::new();

        // Set up shallow clone
        let depth = NonZeroU32::new(1).unwrap();
        clone_options.remote_configuration.fetch_options.shallow =
            Some(gix::remote::fetch::Shallow::DepthAtRemote(depth));

        // Create a new repository (with worktree) at the specified path
        let repo = gix::create_with_options(
            repo_dir,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
        )
        .map_err(|e| format!("Failed to create repository: {}", e))?;

        // Add a remote named "origin" pointing to the GitHub repository
        let mut remote = repo
            .remote_add("origin", clone_url.as_str())
            .map_err(|e| format!("Failed to add remote: {}", e))?;

        // Fetch from the remote to get the branch data
        let progress = Discard;
        let refspecs = Vec::new(); // Empty refspecs means fetch defaults (usually all branches)
        let fetch_result = remote
            .fetch_with_options(
                &refspecs,
                Some(&clone_options.remote_configuration.fetch_options),
                Some(&progress),
            )
            .map_err(|e| format!("Failed to fetch repository: {}", e))?;

        // Now check out the specified branch
        // First try to find the reference
        let ref_name_full = format!("refs/remotes/origin/{}", params.ref_name);

        // Try to find the requested branch or tag
        let found_ref = repo
            .try_find_reference(&ref_name_full)
            .map_err(|e| format!("Failed to find reference: {}", e))?;

        if let Some(found_ref) = found_ref {
            // We found the branch, now create a local branch pointing to it
            let local_branch = format!("refs/heads/{}", params.ref_name);

            // Get the commit ID from the remote reference
            let commit_id = found_ref
                .peel_to_id()
                .map_err(|e| format!("Failed to resolve reference: {}", e))?;

            // Create local branch
            repo.reference_create(
                &local_branch,
                commit_id.detach(),
                false,
                format!("Clone: Setting up branch '{}'", params.ref_name),
            )
            .map_err(|e| format!("Failed to create branch: {}", e))?;

            // Success! Branch is set up
            Ok(())
        } else {
            // Branch wasn't found, but the repository is cloned
            // Let's try to check if it's a tag
            let tag_ref = format!("refs/tags/{}", params.ref_name);
            let found_tag = repo
                .try_find_reference(&tag_ref)
                .map_err(|e| format!("Failed to find tag: {}", e))?;

            if found_tag.is_some() {
                // We found a tag, that's fine
                Ok(())
            } else {
                // Neither branch nor tag found
                Err(format!("Branch or tag '{}' not found", params.ref_name))
            }
        }
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}
