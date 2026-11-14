//! This module defines structs for git references.

/// Object type, typically "commit" for references.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ObjectType {
    #[serde(rename = "commit")]
    Commit,
    #[serde(rename = "tag")]
    Tag,
    #[serde(rename = "tree")]
    Tree,
    #[serde(rename = "blob")]
    Blob,
}

/// The target object of a git reference, including its SHA and type.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefObject {
    /// The SHA1 hash of the target object
    pub sha: String,
    /// The type of the target object, usually "commit"
    #[serde(rename = "type")]
    pub object_type: ObjectType,
}

/// A git reference in a repository, including its name and target object.
/// 
/// This matches the GitHub API format for references.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GitRefObject {
    /// The fully qualified name of the reference (e.g., "refs/heads/main")
    #[serde(rename = "ref")]
    pub ref_name: String,
    /// The target object that this reference points to
    pub object: RefObject,
}

impl GitRefObject {
    /// Create a new GitRefObject from a reference name and target SHA
    pub fn new(ref_name: String, sha: String) -> Self {
        GitRefObject {
            ref_name,
            object: RefObject {
                sha,
                object_type: ObjectType::Commit, // Assume commit by default
            },
        }
    }
}