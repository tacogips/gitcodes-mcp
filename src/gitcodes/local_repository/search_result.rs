//! Types for code search results
//!
//! This module contains the data structures used for representing code search results.

use lumin::search::SearchResult as LuminSearchResult;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Result of a code search operation
///
/// Contains all matches found along with the search parameters that were used.
/// This provides a complete picture of both the search configuration and results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    /// List of search matches found
    pub matches: Vec<LuminSearchResult>,

    /// The search pattern that was used
    pub pattern: String,

    /// Path to the repository that was searched
    pub repository: String,

    /// Whether the search was case-sensitive
    pub case_sensitive: bool,

    /// File extensions filter that was applied (if any)
    pub file_extensions: Option<Vec<String>>,

    /// Directories that were excluded from the search (if any)
    pub exclude_dirs: Option<Vec<String>>,
}

impl CodeSearchResult {
    /// Creates a new CodeSearchResult
    ///
    /// # Parameters
    ///
    /// * `matches` - Vector of search matches
    /// * `pattern` - Search pattern used
    /// * `repository` - Path to the repository
    /// * `case_sensitive` - Whether search was case-sensitive
    /// * `file_extensions` - Optional file extensions filter
    /// * `exclude_dirs` - Optional directories excluded from search
    pub fn new(
        matches: Vec<LuminSearchResult>,
        pattern: &str,
        repository: PathBuf,
        case_sensitive: bool,
        file_extensions: Option<Vec<String>>,
        exclude_dirs: Option<Vec<String>>,
    ) -> Self {
        Self {
            matches,
            pattern: pattern.to_string(),
            repository: repository.display().to_string(),
            case_sensitive,
            file_extensions,
            exclude_dirs,
        }
    }

    /// Converts the search result to a JSON string
    ///
    /// This is useful for backward compatibility or when a JSON representation
    /// is needed for interoperability with other systems.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to convert search results to JSON: {}", e))
    }
}
