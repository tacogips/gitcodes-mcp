use crate::gitcodes::repository_manager::providers::github::{
    parse_github_repository_url, GithubClient, GithubRemoteInfo,
};
use anyhow::{anyhow, Result};

pub enum GitRemoteRepository {
    Github(GithubRemoteInfo),
}

impl GitRemoteRepository {
    fn parse_url(url: &str) -> Result<Self> {
        if let Ok(remote_info) = parse_github_repository_url(&url) {
            Ok(GitRemoteRepository::Github(remote_info))
        } else {
            Err(anyhow!("invalid remote git url: {}", url))
        }
    }
    fn clone_url(&self) -> String {

        //TODO imple
    }
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
pub struct GitRemoteRepositoryInfo {
    /// GitHub username or organization
    pub user: String,
    /// Repository name
    pub repo: String,
    /// Branch or tag name to checkout
    pub ref_name: Option<String>,
}

impl GitRemoteRepositoryInfo {
    /// Returns a GitRef for the reference name
    pub fn git_ref(&self) -> GitRef {
        match &self.ref_name {
            Some(ref_name) => GitRef::new(ref_name),
            None => GitRef::new("main"),
        }
    }
}
