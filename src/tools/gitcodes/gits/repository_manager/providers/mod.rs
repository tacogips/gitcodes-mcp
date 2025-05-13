mod github;

use crate::tools::gitcodes::gits::repository_manager::providers::github::{
    parse_github_url, GithubClient, GithubRemoteInfo,
};
use anyhow::{anyhow, Result};

enum GitRemoteRepository {
    Github(GithubRemoteInfo),
}

impl GitRemoteRepository {
    fn parse_url(url: &str) -> Result<Self> {
        if let Ok(remote_info) = parse_github_url(&url) {
            Ok(GitRemoteRepository::Github((remote_info)))
        } else {
            Err(anyhow!("invalid remote git url: {}", url))
        }
    }
}
