//! Response types for the GitHub code tools MCP server
//!
//! This module defines structured response types that are returned by the
//! MCP tool methods. These types help ensure that the JSON responses are
//! consistently formatted and properly typed.

use crate::gitcodes::CodeSearchResult;
use lumin::view::FileContents;
use serde::{Deserialize, Serialize};

/// Response for the grep_repository tool
///
/// This type directly uses the CodeSearchResult struct
/// from the gitcodes crate to ensure consistency.
#[allow(dead_code)]
pub type CodeSearchResponse = CodeSearchResult;

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

/// Response for the show_file_contents tool
///
/// This type directly uses the FileContents enum from the
/// lumin crate to ensure consistency with the internal implementation.
#[allow(dead_code)]
pub type FileContentsResponse = FileContents;
