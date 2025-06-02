use serde::{Deserialize, Serialize};

/// Response for the list_repository_refs tool
///
/// Contains lists of branches and tags available in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryRefsResponse {
    /// List of branch references
    pub branches: Vec<ReferenceInfo>,

    /// List of tag references
    pub tags: Vec<ReferenceInfo>,
}

/// Information about a git reference (branch or tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceInfo {
    /// Reference name (branch or tag name)
    pub name: String,

    /// Full reference path (e.g., "refs/heads/main")
    pub full_ref: String,

    /// Commit SHA this reference points to
    pub commit_id: String,
}
