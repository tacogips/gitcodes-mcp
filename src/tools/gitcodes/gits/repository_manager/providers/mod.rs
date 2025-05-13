mod github;
use anyhow::Result;
use github::GithubClient;

enum GitProvider {
    Github(GithubClient),
}

impl GitProvider {
    fn parse_url(&url: &str) -> Result<Self> {
        if let Ok(user, repo) = parse_github_url() {}
        Err(error!("invalid remote git url: {}", url))
    }
}

/// Parse a GitHub URL to extract the user and repository name
///
/// This function handles various GitHub URL formats including:
/// - https://github.com/user/repo
/// - git@github.com:user/repo
/// - github:user/repo
///
/// # Parameters
///
/// * `url` - The GitHub URL to parse
///
/// # Returns
///
/// * `Result<(String, String), String>` - A tuple containing (user, repo) or an error message
fn parse_github_url(url: &str) -> Result<(String, String), String> {
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
