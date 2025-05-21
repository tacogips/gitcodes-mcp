//! Tree representation for repository directory structures
//!
//! This module provides types for representing and working with directory tree structures,
//! wrapping functionality from the lumin library with repository-specific adaptations.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Parameters for repository tree operations
///
/// This struct encapsulates the configuration options for retrieving
/// a repository's directory tree structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeParams {
    /// Whether file path matching should be case sensitive (default: false)
    pub case_sensitive: Option<bool>,

    /// Optional relative path within the repository to search from
    ///
    /// When specified, the tree generation will start from this path instead of the repository root.
    /// The path should be relative to the repository root directory.
    ///
    /// # Examples
    /// - `Some(PathBuf::from("src"))` - Generate tree starting from the src directory
    /// - `Some(PathBuf::from("src/utils"))` - Generate tree starting from src/utils
    /// - `None` - Generate tree from repository root (default)
    pub search_relative_path: Option<PathBuf>,

    /// Whether to respect .gitignore files when generating the directory tree (default: true)
    ///
    /// This setting controls whether the tree generation should honor .gitignore rules:
    ///
    /// - `Some(true)` or `None` (default): Respects .gitignore files
    ///   - Files and directories listed in .gitignore will be excluded from the tree
    ///   - Standard gitignore patterns are applied (wildcards, directory patterns, etc.)
    ///   - Nested .gitignore files in subdirectories are also respected
    ///   - Results in a cleaner tree showing only tracked/relevant files
    ///
    /// - `Some(false)`: Ignores .gitignore files
    ///   - All files and directories are included in the tree regardless of .gitignore rules
    ///   - Shows the complete filesystem structure including build artifacts, logs, etc.
    ///   - Useful for debugging or when you need to see ignored files
    ///   - May result in larger trees with temporary/generated files
    ///
    /// # Common Use Cases
    ///
    /// **Default behavior (respecting .gitignore):**
    /// ```rust
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::local_repository::TreeParams;
    ///
    /// let params = TreeParams {
    ///     case_sensitive: None,
    ///     search_relative_path: None,
    ///     respect_gitignore: None, // or Some(true)
    ///     depth: None,
    ///     strip_path_prefix: None,
    /// };
    /// // Will exclude: target/, *.log, .env files, etc.
    /// ```
    ///
    /// **Including all files (ignoring .gitignore):**
    /// ```rust
    /// use std::path::PathBuf;
    /// use gitcodes_mcp::gitcodes::local_repository::TreeParams;
    ///
    /// let params = TreeParams {
    ///     case_sensitive: None,
    ///     search_relative_path: None,
    ///     respect_gitignore: Some(false),
    ///     depth: None,
    ///     strip_path_prefix: None,
    /// };
    /// // Will include: target/, build artifacts, log files, etc.
    /// ```
    ///
    /// # Performance Considerations
    ///
    /// Setting `respect_gitignore: Some(false)` may result in:
    /// - Larger tree structures due to inclusion of build artifacts
    /// - Longer processing times when scanning large ignored directories
    /// - Higher memory usage for storing the complete tree structure
    pub respect_gitignore: Option<bool>,

    /// Maximum depth of directory traversal (default: unlimited)
    pub depth: Option<usize>,

    /// Whether to strip the repository path prefix from results (default: true)
    pub strip_path_prefix: Option<bool>,
}

impl TreeParams {
    /// Convert TreeParams to lumin's TreeOptions
    ///
    /// This method converts our API parameters to the format needed by
    /// the underlying lumin library, with appropriate defaults.
    pub fn to_tree_options(&self, repo_path: &Path) -> lumin::tree::TreeOptions {
        let omit_path_prefix = if self.strip_path_prefix.unwrap_or(true) {
            Some(repo_path.to_path_buf())
        } else {
            None
        };

        lumin::tree::TreeOptions {
            case_sensitive: self.case_sensitive.unwrap_or(false),
            respect_gitignore: self.respect_gitignore.unwrap_or(true),
            depth: self.depth,
            omit_path_prefix,
        }
    }
}

/// Repository directory tree structure
///
/// This struct represents the hierarchical file/directory structure of a repository
/// and is used to visualize the contents of a repository in a tree format.
/// It wraps the lumin library's DirectoryTree for serialization and future extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryTree {
    /// Path to the directory, relative to the repository root
    pub dir: String,
    /// List of entries (files and subdirectories) in this directory
    pub entries: Vec<TreeEntry>,
}

/// Entry in a repository directory tree
///
/// This enum represents either a file or a directory in the repository tree structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "name")]
pub enum TreeEntry {
    /// A file entry with just a name
    File(String),
    /// A directory entry with just a name
    Directory(String),
}

impl From<lumin::tree::DirectoryTree> for RepositoryTree {
    fn from(tree: lumin::tree::DirectoryTree) -> Self {
        RepositoryTree {
            dir: tree.dir,
            entries: tree.entries.into_iter().map(TreeEntry::from).collect(),
        }
    }
}

impl From<lumin::tree::Entry> for TreeEntry {
    fn from(entry: lumin::tree::Entry) -> Self {
        match entry {
            lumin::tree::Entry::File { name } => TreeEntry::File(name),
            lumin::tree::Entry::Directory { name } => TreeEntry::Directory(name),
        }
    }
}
