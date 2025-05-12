use rand::Rng;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Enum representing a repository location, either a GitHub URL or a local filesystem path
#[derive(Debug, Clone)]
pub enum RepositoryLocation {
    /// A GitHub repository URL (https://github.com/user/repo, git@github.com:user/repo.git, or github:user/repo)
    GitHubUrl(String),
    /// A local filesystem path
    LocalPath(PathBuf),
}

impl FromStr for RepositoryLocation {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Check if it's a local path first
        if Path::new(s).exists() {
            return Ok(RepositoryLocation::LocalPath(PathBuf::from(s)));
        }
        
        // Otherwise, treat it as a GitHub URL
        if s.starts_with("https://github.com/") || 
           s.starts_with("git@github.com:") || 
           s.starts_with("github:") {
            Ok(RepositoryLocation::GitHubUrl(s.to_string()))
        } else {
            Err(format!("Invalid repository location: {}", s))
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
                    clone_repository(&repo_dir, &user, &repo, &ref_name).await?
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

/// Clone a repository from GitHub
///
/// Creates a directory and performs a shallow clone of the specified repository.
///
/// # Parameters
///
/// * `repo_dir` - The directory where the repository should be cloned
/// * `user` - GitHub username or organization
/// * `repo` - Repository name
/// * `ref_name` - Branch or tag name to checkout
async fn clone_repository(
    repo_dir: &Path,
    user: &str,
    repo: &str,
    ref_name: &str,
) -> Result<(), String> {
    // Create directory if it doesn't exist
    if let Err(e) = tokio::fs::create_dir_all(repo_dir).await {
        return Err(format!("Failed to create directory: {}", e));
    }

    // Clone repository
    let clone_url = format!("https://github.com/{}/{}.git", user, repo);

    // Clone with git command
    let repo_dir_clone = repo_dir.to_string_lossy().to_string();
    let ref_name_clone = ref_name.to_string();
    let clone_result = tokio::task::spawn_blocking(move || {
        let status = std::process::Command::new("git")
            .args([
                "clone",
                "--depth=1",
                "--branch",
                &ref_name_clone,
                &clone_url,
                &repo_dir_clone,
            ])
            .status();

        match status {
            Ok(exit_status) if exit_status.success() => Ok(()),
            Ok(exit_status) => Err(format!("Git clone failed with status: {}", exit_status)),
            Err(e) => Err(format!("Failed to execute git clone: {}", e)),
        }
    })
    .await;

    // Handle errors during cloning
    if let Err(e) = clone_result {
        return Err(format!("Failed to run git clone: {}", e));
    }

    clone_result.unwrap()
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
    // Repository exists, update it
    let repo_dir_clone = repo_dir.to_string_lossy().to_string();
    let ref_name_clone = ref_name.to_string();
    let update_result = tokio::task::spawn_blocking(move || {
        // Change to the repository directory
        let current_dir = match std::env::current_dir() {
            Ok(dir) => dir,
            Err(e) => return Err(format!("Failed to get current directory: {}", e)),
        };

        if let Err(e) = std::env::set_current_dir(&repo_dir_clone) {
            return Err(format!("Failed to change directory: {}", e));
        }

        // Fetch updates
        let fetch_status = std::process::Command::new("git")
            .args(["fetch", "--depth=1", "origin"])
            .status();

        if let Err(e) = fetch_status {
            let _ = std::env::set_current_dir(current_dir);
            return Err(format!("Git fetch failed: {}", e));
        }

        if !fetch_status.unwrap().success() {
            let _ = std::env::set_current_dir(current_dir);
            return Err("Git fetch failed".to_string());
        }

        // Try to checkout the requested branch
        let checkout_status = std::process::Command::new("git")
            .args(["checkout", &ref_name_clone])
            .status();

        if let Err(e) = checkout_status {
            let _ = std::env::set_current_dir(current_dir);
            return Err(format!("Git checkout failed: {}", e));
        }

        if !checkout_status.unwrap().success() {
            // Try origin/branch_name
            let origin_checkout = std::process::Command::new("git")
                .args(["checkout", &format!("origin/{}", ref_name_clone)])
                .status();

            if let Err(e) = origin_checkout {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Git checkout failed: {}", e));
            }

            if !origin_checkout.unwrap().success() {
                let _ = std::env::set_current_dir(current_dir);
                return Err(format!("Branch/tag not found: {}", ref_name_clone));
            }
        }

        // Change back to the original directory
        if let Err(e) = std::env::set_current_dir(current_dir) {
            return Err(format!("Failed to restore directory: {}", e));
        }

        Ok(())
    })
    .await;

    // Handle update errors
    if let Err(e) = update_result {
        return Err(format!("Failed to update repository: {}", e));
    }

    update_result.unwrap()
}
