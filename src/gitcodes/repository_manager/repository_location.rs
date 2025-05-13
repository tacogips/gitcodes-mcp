use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::gitcodes::LocalRepository;

use super::providers::GitRemoteRepository;

/// Enum representing a repository location, either a GitHub URL or a local filesystem path
#[derive(Debug, Clone, serde::Deserialize)]
pub enum RepositoryLocation {
    RemoteRepository(GitRemoteRepository),
    /// A local filesystem path
    LocalPath(LocalRepository),
}

impl FromStr for RepositoryLocation {
    type Err = String;

    fn from_str(repo_location_path_or_url: &str) -> Result<Self, Self::Err> {
        let sanitized_location = repo_location_path_or_url.trim();

        // Check if it's a local path first
        //TODO(exists) check if it's a local path
        if Path::new(sanitized_location).exists() {
            return Ok(RepositoryLocation::LocalPath(LocalRepository::new(
                PathBuf::from(sanitized_location),
            )));
        } else {
            let remote_repository = GitRemoteRepository::parse_url(repo_location_path_or_url)?;
            Ok(RepositoryLocation::RemoteRepository(remote_repository))
        }
    }
}
