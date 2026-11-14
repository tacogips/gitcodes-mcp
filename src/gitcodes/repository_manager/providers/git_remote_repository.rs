use crate::gitcodes::repository_manager::providers::github::{
    parse_github_repository_url, GithubRemoteInfo,
};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, strum::Display, strum::EnumString)]
pub enum GitProvider {
    #[strum(serialize = "github")]
    Github,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub enum GitRemoteRepository {
    Github(GithubRemoteInfo),
}

impl GitRemoteRepository {
    pub fn parse_url(url: &str) -> Result<Self> {
        if let Ok(remote_info) = parse_github_repository_url(url) {
            Ok(GitRemoteRepository::Github(remote_info))
        } else {
            Err(anyhow!("invalid remote git url: {}", url))
        }
    }

    pub fn clone_url(&self) -> String {
        match self {
            GitRemoteRepository::Github(github_info) => github_info.clone_url.clone(),
        }
    }

    pub fn get_unique_name(&self) -> String {
        match self {
            GitRemoteRepository::Github(github_info) => {
                format!(
                    "{}__{}",
                    github_info.repo_info.user, github_info.repo_info.repo
                )
            }
        }
    }

    pub fn get_ref_name(&self) -> Option<String> {
        match self {
            GitRemoteRepository::Github(github_info) => github_info.repo_info.ref_name.clone(),
        }
    }

    /// Get the authenticated URL for this repository
    ///
    /// If a GitHub token is provided and this is a GitHub repository,
    /// returns a URL with authentication credentials included.
    /// Otherwise, returns the regular clone URL.
    ///
    /// # Parameters
    ///
    /// * `token` - Optional GitHub authentication token
    ///
    /// # Returns
    ///
    /// * `String` - The URL to use for cloning, with authentication if applicable
    pub fn get_authenticated_url(&self, token: Option<&String>) -> String {
        let clone_url = self.clone_url();

        // If we have a token and this is a GitHub repository with an HTTPS URL,
        // insert the token into the URL
        match (token, self) {
            (Some(token), GitRemoteRepository::Github(_))
                if clone_url.starts_with("https://github.com") =>
            {
                format!(
                    "https://{}:x-oauth-basic@{}",
                    token,
                    clone_url.trim_start_matches("https://")
                )
            }
            _ => clone_url,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitRemoteRepositoryInfo {
    /// GitHub username or organization
    pub user: String,
    /// Repository name
    pub repo: String,
    /// Branch or tag name to checkout
    pub ref_name: Option<String>,
}
