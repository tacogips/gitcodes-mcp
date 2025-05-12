use rand::Rng;
use rmcp::schemars;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use gix;
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

/// Enum representing a repository location, either a GitHub URL or a local filesystem path
#[derive(Debug, Clone, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub enum RepositoryLocation {
    /// A GitHub repository URL (https://github.com/user/repo, git@github.com:user/repo.git, or github:user/repo)
    GitHubUrl(String),
    /// A local filesystem path
    LocalPath(PathBuf),
}

impl FromStr for RepositoryLocation {
    type Err = String;

    fn from_str(repo_location_str: &str) -> Result<Self, Self::Err> {
        let sanitized_location = repo_location_str.trim();
        
        // Check if it's a local path first
        if Path::new(sanitized_location).exists() {
            return Ok(RepositoryLocation::LocalPath(PathBuf::from(sanitized_location)));
        }
        
        // Otherwise, treat it as a GitHub URL
        if sanitized_location.starts_with("https://github.com/") || 
           sanitized_location.starts_with("git@github.com:") || 
           sanitized_location.starts_with("github:") {
            Ok(RepositoryLocation::GitHubUrl(sanitized_location.to_string()))
        } else {
            Err(format!("Invalid repository location: {}", sanitized_location))
        }
    }
}

/// Repository information after URL parsing and preparation
#[derive(Debug)]
pub struct RepositoryInfo {
    /// GitHub username or organization
    pub user: String,
    /// Repository name
    pub repo: String,
    /// Local directory where repository is cloned
    pub repo_dir: PathBuf,
    /// Branch or tag name to use
    pub ref_name: String,
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
            return Err(format!("Specified path '{}' is not a directory", base_dir.display()));
        }
        
        // Check if the directory is writable by trying to create a test file
        let test_file_path = base_dir.join(".write_test_repo_cache_file");
        match std::fs::File::create(&test_file_path) {
            Ok(_) => {
                // Clean up the test file
                let _ = std::fs::remove_file(test_file_path);
            },
            Err(e) => return Err(format!("Directory '{}' is not writable: {}", base_dir.display(), e)),
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
    
    /// Generate a unique directory name for the repository
    fn get_repo_dir(&self, user: &str, repo: &str) -> PathBuf {
        let random_suffix = rand::thread_rng().gen::<u32>() % 10000;
        let dir_name = format!("mcp_github_{}_{}_{}", user, repo, random_suffix);
        self.repository_cache_dir_base.join(dir_name)
    }
    
    /// Check if repository is already cloned
    async fn is_repo_cloned(&self, dir: &Path) -> bool {
        tokio::fs::metadata(dir).await.is_ok()
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
    ) -> Result<RepositoryInfo, String> {
        match repo_location {
            RepositoryLocation::LocalPath(local_path) => {
                // For local paths, use the path directly
                if !local_path.is_dir() {
                    return Err(format!("Local path '{}' is not a directory", local_path.display()));
                }
                
                // For local repositories, we don't need to use ref_name
                // Just use a placeholder or default
                let actual_ref_name = ref_name.unwrap_or_else(|| "local".to_string());
                
                Ok(RepositoryInfo {
                    user: "local".to_string(),
                    repo: "repository".to_string(),
                    repo_dir: local_path.clone(),
                    ref_name: actual_ref_name,
                })
            },
            RepositoryLocation::GitHubUrl(_) => {
                // Handle GitHub repository URLs
                // Parse repository URL
                let (user, repo) = match parse_repository_url(repo_location) {
                    Ok(result) => result,
                    Err(e) => return Err(format!("Error: {}", e)),
                };

                // Default branch if not specified
                let ref_name = ref_name.unwrap_or_else(|| "main".to_string());

                // Get a temporary directory for the repository
                let repo_dir = self.get_repo_dir(&user, &repo);

                // Check if repo is already cloned
                let is_cloned = self.is_repo_cloned(&repo_dir).await;

                // If repo is not cloned, clone it
                if !is_cloned {
                    // We've already matched this as GitHubUrl above, so no need to extract the URL again
                    let clone_params = RemoteGitRepositoryInfo {
                        user: user.clone(),
                        repo: repo.clone(),
                        ref_name: ref_name.clone(),
                    };
                    clone_repository(&repo_dir, &clone_params).await?
                } else {
                    update_repository(&repo_dir, &ref_name).await?
                }

                Ok(RepositoryInfo {
                    user,
                    repo,
                    repo_dir,
                    ref_name,
                })
            }
        }
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::with_default_cache_dir()
    }
}

// Parse repository URL to extract user and repo name
fn parse_repository_url(repo_location: &RepositoryLocation) -> Result<(String, String), String> {
    match repo_location {
        RepositoryLocation::LocalPath(_) => {
            // Return placeholder values for user and repo for local paths
            Ok(("local".to_string(), "repository".to_string()))
        },
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

// These functions have been converted to methods of RepositoryManager

/// Parameters for GitHub repository cloning
///
/// Contains all the parameters needed for cloning a GitHub repository.
/// This struct encapsulates repository parameters for the clone_repository function.
///
/// # Examples
///
/// ```
/// use gitcodes_mcp::tools::gitcodes::github_service::git_repository::RemoteGitRepositoryInfo;
///
/// // Basic clone parameters
/// let params = RemoteGitRepositoryInfo {
///     user: "rust-lang".to_string(),
///     repo: "rust".to_string(),
///     ref_name: "main".to_string(),
/// };
/// ```
#[derive(Debug, Clone, schemars::JsonSchema, serde::Serialize, serde::Deserialize)]
pub struct RemoteGitRepositoryInfo {
    /// GitHub username or organization
    #[schemars(description = "The GitHub username or organization owning the repository. Must be the exact username as it appears in GitHub URLs.")]
    pub user: String,
    /// Repository name
    #[schemars(description = "The name of the repository to clone. Must be the exact repository name as it appears in GitHub URLs.")]
    pub repo: String,
    /// Branch or tag name to checkout
    #[schemars(description = "The branch or tag name to checkout after cloning. Defaults to 'main' if not specified.")]
    pub ref_name: String,
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
/// use gitcodes_mcp::tools::gitcodes::github_service::git_repository::{clone_repository, RemoteGitRepositoryInfo};
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
    repo_dir: &Path,
    params: &RemoteGitRepositoryInfo,
) -> Result<(), String> {
    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(repo_dir) {
        return Err(format!("Failed to create directory: {}", e));
    }

    // Clone repository
    let clone_url = format!("https://github.com/{}/{}.git", params.user, params.repo);
    
    // Configure clone options with depth=1 and branch to match our original git command
    let mut process = gix::clone::PrepareFetch::new(
        &clone_url, 
        repo_dir, 
        gix::clone::PrepareType::Shallow,
    )
    .branch_to_checkout(&params.ref_name);
    
    // Execute the clone operation
    match process.fetch_only(gix::progress::Discard) {
        Ok(fetch_ok) => {
            // Clone was successful
            match fetch_ok.repository {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Repository preparation failed: {}", e)),
            }
        },
        Err(e) => Err(format!("Git clone failed: {}", e)),
    }
}

/// Update an existing repository
///
/// Fetches the latest changes and checks out the specified branch/tag.
///
/// # Parameters
///
/// * `repo_dir` - The directory containing the repository
/// * `ref_name` - Branch or tag name to checkout
async fn update_repository(repo_dir: &Path, ref_name: &str) -> Result<(), String> {
    // Open the existing repository
    let repo = match gix::open(repo_dir) {
        Ok(repo) => repo,
        Err(e) => return Err(format!("Failed to open repository: {}", e)),
    };
    
    // Fetch the latest changes
    if let Err(e) = remote_fetch(&repo) {
        return Err(format!("Failed to fetch updates: {}", e));
    }
    
    // Checkout the branch or tag
    // Try to find the reference directly
    let maybe_ref = repo.try_find_reference(ref_name);
    if let Ok(Some(reference)) = maybe_ref {
        // Reference exists, try to follow and peel it
        if let Ok(_) = reference.peel_to_id_in_place() {
            return Ok(());
        }
    }
    
    // Try with origin/ prefix if direct reference wasn't found
    let origin_ref_name = format!("refs/remotes/origin/{}", ref_name);
    let maybe_origin_ref = repo.try_find_reference(&origin_ref_name);
    if let Ok(Some(reference)) = maybe_origin_ref {
        // Origin reference exists, try to follow and peel it
        if let Ok(_) = reference.peel_to_id_in_place() {
            return Ok(());
        }
    }
    
    Err(format!("Branch/tag not found: {}", ref_name))
}

/// Fetch updates from the remote
fn remote_fetch(repo: &gix::Repository) -> Result<(), String> {
    // Find the 'origin' remote
    let remote = match repo.find_remote("origin") {
        Ok(remote) => remote,
        Err(e) => return Err(format!("Could not find origin remote: {}", e)),
    };
    
    // Configure fetch
    let mut prepare_fetch = remote.prepare_fetch();
    prepare_fetch.depth(1); // Shallow fetch
    
    // Execute fetch
    prepare_fetch
        .fetch(gix::progress::Discard)
        .map_err(|e| format!("Fetch failed: {}", e))?;
    
    Ok(())
}
