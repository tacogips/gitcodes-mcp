
use rand::Rng;


use crate::tools::gitcodes::RepositoryManager;

// Public functions for use with RepositoryManager

// Parse repository URL to extract user and repo name
pub fn parse_repository_url(_manager: &RepositoryManager, url: &str) -> Result<(String, String), String> {
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
