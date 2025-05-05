use rand::Rng;

/// Repository manager for Git operations
///
/// Handles cloning, updating, and retrieving information from GitHub repositories.
/// Uses system temporary directories to store cloned repositories.
#[derive(Clone)]
pub struct RepositoryManager {
    pub(crate) temp_dir_base: String,
}

impl RepositoryManager {
    /// Creates a new RepositoryManager instance
    ///
    /// Initializes a repository manager with the system's temporary directory
    /// as the base location for storing cloned repositories.
    pub fn new() -> Self {
        let system_temp = std::env::temp_dir().to_string_lossy().to_string();
        Self {
            temp_dir_base: system_temp,
        }
    }
}

impl Default for RepositoryManager {
    fn default() -> Self {
        Self::new()
    }
}

// Parse repository URL to extract user and repo name
pub fn parse_repository_url(
    _manager: &RepositoryManager,
    url: &str,
) -> Result<(String, String), String> {
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

// Generate a unique directory name for the repository
pub fn get_repo_dir(manager: &RepositoryManager, user: &str, repo: &str) -> String {
    format!(
        "{}/mcp_github_{}_{}_{}",
        manager.temp_dir_base,
        user,
        repo,
        rand::thread_rng().gen::<u32>() % 10000
    )
}

// Check if repository is already cloned
pub async fn is_repo_cloned(_manager: &RepositoryManager, dir: &str) -> bool {
    tokio::fs::metadata(dir).await.is_ok()
}

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
pub async fn clone_repository(
    repo_dir: &str,
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
    let repo_dir_clone = repo_dir.to_string();
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
pub async fn update_repository(
    repo_dir: &str,
    ref_name: &str,
) -> Result<(), String> {
    // Repository exists, update it
    let repo_dir_clone = repo_dir.to_string();
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