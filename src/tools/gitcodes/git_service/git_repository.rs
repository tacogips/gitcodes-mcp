use gix;
use gix::bstr::ByteSlice;
use gix::progress::Discard;
use rand::Rng;
use rmcp::schemars;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::str::FromStr;
use thiserror::Error;

/// Errors that can occur during git operations
#[derive(Error, Debug)]
pub enum GitError {
    #[error("Git clone error: {0}")]
    Clone(#[from] gix::clone::Error),

    #[error("Git fetch error: {0}")]
    Fetch(String),

    #[error("Git checkout error: {0}")]
    Checkout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Git operation error: {0}")]
    Other(String),
}

pub struct GitRef(String);

impl GitRef {
    /// Creates a new GitRef from a string
    pub fn new<S: Into<String>>(s: S) -> Self {
        GitRef(s.into())
    }

    /// Returns the underlying string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Enum representing a repository location, either a GitHub URL or a local filesystem path
#[derive(Debug, Clone, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub enum RepositoryLocation {
    /// A GitHub repository URL (https://github.com/user/repo, git@github.com:user/repo.git, or github:user/repo)
    GitHubUrl(String),
    /// A local filesystem path
    LocalPath(LocalRepository),
}

impl FromStr for RepositoryLocation {
    type Err = String;

    fn from_str(repo_location_str: &str) -> Result<Self, Self::Err> {
        let sanitized_location = repo_location_str.trim();

        // Check if it's a local path first
        if Path::new(sanitized_location).exists() {
            return Ok(RepositoryLocation::LocalPath(PathBuf::from(
                sanitized_location,
            )));
        }

        // Otherwise, treat it as a GitHub URL
        if sanitized_location.starts_with("https://github.com/")
            || sanitized_location.starts_with("git@github.com:")
            || sanitized_location.starts_with("github:")
        {
            Ok(RepositoryLocation::GitHubUrl(
                sanitized_location.to_string(),
            ))
        } else {
            Err(format!(
                "Invalid repository location: {}",
                sanitized_location
            ))
        }
    }
}

#[derive(Debug)]
pub struct LocalRepositoryInfo {
    pub remote_repository_info: Option<RemoteGitRepositoryInfo>,
    /// Local directory where repository is cloned
    pub repo_dir: PathBuf,
}

/// Repository manager for Git operations
///
/// Handles cloning, updating, and retrieving information from GitHub repositories.
/// Uses a dedicated directory to store cloned repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    pub(crate) repository_cache_dir_base: PathBuf,
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
    pub fn new(repository_cache_dir: Option<PathBuf>) -> Result<Self, String> {
        // Use provided path or default to system temp directory
        let base_dir = match repository_cache_dir {
            Some(path) => path,
            None => std::env::temp_dir(),
        };

        // Validate and ensure the directory exists
        if !base_dir.exists() {
            // Try to create the directory if it doesn't exist
            std::fs::create_dir_all(&base_dir)
                .map_err(|e| format!("Failed to create repository cache directory: {}", e))?;
        } else if !base_dir.is_dir() {
            return Err(format!(
                "Specified path '{}' is not a directory",
                base_dir.display()
            ));
        }

        // Check if the directory is writable by trying to create a test file
        let test_file_path = base_dir.join(".write_test_repo_cache_file");
        match std::fs::File::create(&test_file_path) {
            Ok(_) => {
                // Clean up the test file
                let _ = std::fs::remove_file(test_file_path);
            }
            Err(e) => {
                return Err(format!(
                    "Directory '{}' is not writable: {}",
                    base_dir.display(),
                    e
                ))
            }
        }

        Ok(Self {
            repository_cache_dir_base: base_dir,
        })
    }

    /// Creates a new RepositoryManager with the system's default cache directory
    ///
    /// This is a convenience method that creates a RepositoryManager with the
    /// system's temporary directory as the repository cache location.
    pub fn with_default_cache_dir() -> Self {
        Self::new(None).expect("Failed to initialize with system temporary directory")
    }

    /// Parses a repository URL or local file path and prepares it for operations
    ///
    /// This method:
    /// 1. Processes the repository location (URL or local path)
    /// 2. For URLs: Extracts user and repo name, clones/updates the repository
    /// 3. For local paths: Uses the path directly without cloning
    ///
    /// # Parameters
    ///
    /// * `repo_location` - The repository location (either a GitHub URL or local file path)
    /// * `ref_name` - Optional branch or tag name (only used for URLs)
    pub async fn parse_and_prepare_repository(
        &self,
        repo_location: &RepositoryLocation,
        ref_name: Option<String>,
    ) -> Result<LocalRepositoryInfo, String> {
        match repo_location {
            RepositoryLocation::LocalPath(local_path) => {
                // For local paths, use the path directly
                if !local_path.is_dir() {
                    return Err(format!(
                        "Local path '{}' is not a directory",
                        local_path.display()
                    ));
                }

                Ok(LocalRepositoryInfo {
                    remote_repository_info: None,
                    repo_dir: local_path.to_path_buf(),
                })
            }
            RepositoryLocation::GitHubUrl(_) => {
                // Handle GitHub repository URLs
                // Parse repository URL
                let (user, repo) = match parse_repository_url(repo_location) {
                    Ok(result) => result,
                    Err(e) => return Err(format!("Error: {}", e)),
                };

                // Get a temporary directory for the repository
                let repo_dir = self.get_repo_dir(&user, &repo);

                // Check if repo is already cloned
                let already_fetched = self.is_local_repo_exists(&repo_dir).await;

                // Clone ref_name once at the beginning to avoid ownership issues
                let ref_name_clone = ref_name.clone();

                // If repo is not cloned, clone it
                if !already_fetched {
                    // We've already matched this as GitHubUrl above, so no need to extract the URL again
                    let clone_params = RemoteGitRepositoryInfo {
                        user: user.clone(),
                        repo: repo.clone(),
                        ref_name: ref_name_clone.clone(),
                    };
                    clone_repository(&repo_dir, &clone_params).await?
                } else {
                    // Convert Option<String> to GitRef
                    let ref_str = ref_name_clone.as_ref().map_or("main", |s| s.as_str());
                    let git_ref = GitRef::new(ref_str);
                    update_repository(&repo_dir, &git_ref).await?
                }

                Ok(LocalRepositoryInfo {
                    remote_repository_info: Some(RemoteGitRepositoryInfo {
                        user,
                        repo,
                        ref_name: ref_name_clone,
                    }),
                    repo_dir,
                })
            }
        }
    }

    // parse_and_prepare_repository method has been moved to git_repository.rs

    // Code search methods have been moved to code_search.rs

    // Function to fetch repository refs (branches and tags)
    async fn fetch_repository_refs(
        &self,
        remote_repository_info: RemoteGitRepositoryInfo,
    ) -> Result<String, String> {
        //unimplemented!()
        // Change to the repository directory
        //let current_dir = match std::env::current_dir() {
        //    Ok(dir) => dir,
        //    Err(e) => return Err(format!("Failed to get current directory: {}", e)),
        //};

        //if let Err(e) = std::env::set_current_dir(&self.repository_cache_dir_base) {
        //    return Err(format!("Failed to change directory: {}", e));
        //}

        //// First run git fetch to make sure we have all refs
        //let fetch_status = std::process::Command::new("git")
        //    .args(["fetch", "--all"])
        //    .status();

        //if let Err(e) = fetch_status {
        //    let _ = std::env::set_current_dir(current_dir);
        //    return Err(format!("Git fetch failed: {}", e));
        //}

        //if !fetch_status.unwrap().success() {
        //    let _ = std::env::set_current_dir(current_dir);
        //    return Err("Git fetch failed".to_string());
        //}

        //// Get branches
        //let branches_output = std::process::Command::new("git")
        //    .args(["branch", "-r"])
        //    .output();

        //let branches_output = match branches_output {
        //    Ok(output) => output,
        //    Err(e) => {
        //        let _ = std::env::set_current_dir(current_dir);
        //        return Err(format!("Failed to list branches: {}", e));
        //    }
        //};

        //let branches_str = String::from_utf8_lossy(&branches_output.stdout).to_string();

        //// Get tags
        //let tags_output = std::process::Command::new("git").args(["tag"]).output();

        //let tags_output = match tags_output {
        //    Ok(output) => output,
        //    Err(e) => {
        //        let _ = std::env::set_current_dir(current_dir);
        //        return Err(format!("Failed to list tags: {}", e));
        //    }
        //};

        //let tags_str = String::from_utf8_lossy(&tags_output.stdout).to_string();

        //// Change back to the original directory
        //if let Err(e) = std::env::set_current_dir(current_dir) {
        //    return Err(format!("Failed to restore directory: {}", e));
        //}

        //// Format the output
        //let mut result = String::new();
        //result.push_str(&format!(
        //    "Repository: {}/{}

        //",
        //    user_clone, repo_clone
        //));

        //// Extract and format branches
        //let branches: Vec<String> = branches_str
        //    .lines()
        //    .filter_map(|line| {
        //        let line = line.trim();
        //        if line.starts_with("origin/") && !line.contains("HEAD") {
        //            Some(line.trim_start_matches("origin/").to_string())
        //        } else {
        //            None
        //        }
        //    })
        //    .collect();

        //// Extract and format tags
        //let tags: Vec<String> = tags_str
        //    .lines()
        //    .map(|line| line.trim().to_string())
        //    .filter(|line| !line.is_empty())
        //    .collect();

        //// Add branches section
        //result.push_str(
        //    "## Branches
        //z",
        //z    //);
        //z    //if branches.is_empty() {
        //z    //    result.push_str(
        //z    //        "No branches found
        //z",
        //    );
        //} else {
        //    for branch in branches {
        //        result.push_str(&format!("- {}\n", branch));
        //    }
        //}

        //// Add tags section
        //result.push_str(
        //    " ## Tags ",
        //);
        //if tags.is_empty() {
        //    result.push_str(
        //        "No tags found
        //zz",
        //    );
        //} else {
        //    for tag in tags {
        //        result.push_str(&format!("- {}\n", tag));
        //    }
        //}

        //Ok(result)
        unimplemented!()
    }

    // Parse repository URL to extract user and repo name
    fn parse_repository_url(
        repo_location: &RepositoryLocation,
    ) -> Result<(String, String), String> {
        match repo_location {
            RepositoryLocation::LocalPath(_) => {
                // Return placeholder values for user and repo for local paths
                Ok(("local".to_string(), "repository".to_string()))
            }
            RepositoryLocation::GitHubUrl(url) => {
                let user_repo = if url.starts_with("https://github.com/") {
                    url.trim_start_matches("https://github.com/")
                        .trim_end_matches(".git")
                        .to_string()
                } else if url.starts_with("git@github.com:") {
                    url.trim_start_matches("git@github.com:")
                        .trim_end_matches(".git")
                        .to_string()
                } else if url.starts_with("github:") {
                    url.trim_start_matches("github:").to_string()
                } else {
                    return Err("Invalid GitHub repository URL format".to_string());
                };

                let parts: Vec<&str> = user_repo.split('/').collect();
                if parts.len() != 2 {
                    return Err("Invalid GitHub repository URL format".to_string());
                }

                Ok((parts[0].to_string(), parts[1].to_string()))
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
    async fn clone_repository(&self, params: &RemoteGitRepositoryInfo) -> Result<(), String> {
        let repo_dir = self.repository_cache_dir_base;
        // Create directory if it doesn't exist
        if let Err(e) = std::fs::create_dir_all(repo_dir) {
            return Err(format!("Failed to create directory: {}", e));
        }

        // Build the clone URL
        let clone_url = format!("https://github.com/{}/{}.git", params.user, params.repo);

        // Create a repository using gitoxide
        // First convert to byte slice for parsing
        let url_bstr = clone_url.as_bytes().as_bstr();

        // Parse the URL using gix's URL parser
        let url = gix::url::parse(url_bstr).map_err(|e| format!("Failed to parse URL: {}", e))?;

        // Create options for the clone operation
        let mut clone_options = gix::clone::PrepareFetch::default();

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

// These functions have been converted to methods of RepositoryManager

/// Parameters for GitHub repository cloning
///
/// Contains all the parameters needed for cloning a GitHub repository.
/// This struct encapsulates repository parameters for the clone_repository function.
///
/// # Examples
///
/// ```
/// use gitcodes_mcp::tools::gitcodes::git_service::git_repository::RemoteGitRepositoryInfo;
///
/// // Basic clone parameters
/// let params = RemoteGitRepositoryInfo {
///     user: "rust-lang".to_string(),
///     repo: "rust".to_string(),
///     ref_name: "main".to_string(),
/// };
/// ```
//#[derive(Debug, Clone, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
//pub struct RemoteGitRepositoryInfo {
//    /// GitHub username or organization
//    #[schemars(
//        description = "The GitHub username or organization owning the repository. Must be the exact username as it appears in GitHub URLs."
//    )]
//    pub user: String,
//    /// Repository name
//    #[schemars(
//        description = "The name of the repository to clone. Must be the exact repository name as it appears in GitHub URLs."
//    )]
//    pub repo: String,
//    /// Branch or tag name to checkout
//    #[schemars(
//        description = "The branch or tag name to checkout after cloning. Defaults to 'main' if not specified."
//    )]
//    pub ref_name: Option<String>,
//}
//

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoteGitRepositoryInfo {
    /// GitHub username or organization
    pub user: String,
    /// Repository name
    pub repo: String,
    /// Branch or tag name to checkout
    pub ref_name: Option<String>,
}

impl RemoteGitRepositoryInfo {
    /// Returns a GitRef for the reference name
    pub fn git_ref(&self) -> GitRef {
        match &self.ref_name {
            Some(ref_name) => GitRef::new(ref_name),
            None => GitRef::new("main"),
        }
    }
}
