use std::{
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::gitcodes::LocalRepository;

use super::providers::GitRemoteRepository;

/// Enum representing a repository location, either a remote repository URL or an absolute local filesystem path
///
/// # Valid repository locations
///
/// This enum supports two types of repository locations:
///
/// 1. **Remote Repository URLs**:
///    - GitHub formats: `github:user/repo`, `git@github.com:user/repo.git`, `https://github.com/user/repo`
///
/// 2. **Local File System Paths**:
///    - Must be absolute paths (e.g., '/path/to/repo' on Unix or 'C:\\repos\\project' on Windows)
///    - Paths must refer to an existing Git repository directory
///    - Relative paths are not supported for security reasons
///    - File URLs (e.g., 'file:///path/to/repo' or 'file:/path/to/repo') are also supported and converted to local paths
///
/// When creating from a string using `from_str`, the function will first check if the string is
/// a file URL and convert it to a local path. Then it will check if the path exists and is absolute,
/// in which case it will be treated as a local repository. Otherwise, it will attempt to parse it as a remote repository URL.
#[derive(Debug, Clone, serde::Deserialize)]
pub enum RepositoryLocation {
    RemoteRepository(GitRemoteRepository),
    /// A local filesystem path (must be an absolute path)
    ///
    /// Relative paths are not supported for security reasons.
    /// The path must exist and be a valid Git repository directory.
    /// File URLs (e.g., 'file:///path/to/repo') are automatically converted to local paths.
    LocalPath(LocalRepository),
}

impl FromStr for RepositoryLocation {
    type Err = String;

    fn from_str(repo_location_path_or_url: &str) -> Result<Self, Self::Err> {
        let sanitized_location = repo_location_path_or_url.trim();

        // Handle file:// URLs by converting them to local paths
        let path_str = if sanitized_location.starts_with("file:") {
            // Strip the file: or file:// prefix to get the actual path
            let path_part = sanitized_location
                .strip_prefix("file://")
                .or_else(|| sanitized_location.strip_prefix("file:"))
                .unwrap_or(sanitized_location);
            path_part
        } else {
            sanitized_location
        };

        // Check if it's a local path
        let path = Path::new(path_str);
        if path.exists() {
            // Verify it's an absolute path
            if path.is_absolute() {
                Ok(RepositoryLocation::LocalPath(LocalRepository::new(
                    PathBuf::from(path_str),
                )))
            } else {
                Err(format!(
                    "Invalid repository location: '{}'. Local paths must be absolute paths",
                    sanitized_location
                ))
            }
        } else {
            // Try to parse as remote repository URL
            let remote_repository = GitRemoteRepository::parse_url(repo_location_path_or_url)
                .map_err(|e| format!("Invalid repository location: {}. Valid formats include absolute local paths or remote URLs (https://github.com/user/repo, git@github.com:user/repo.git, github:user/repo)", e))?;
            Ok(RepositoryLocation::RemoteRepository(remote_repository))
        }
    }
}
