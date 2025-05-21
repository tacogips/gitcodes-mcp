//! Types for code search results
//!
//! This module contains the data structures used for representing code search results.

use lumin::search::SearchResultLine as LuminSearchResultLine;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parameters for creating a new CodeSearchResult
#[derive(Debug, Clone)]
pub struct CodeSearchParams {
    pub total_match_line_number: usize,
    pub search_result_lines: Vec<LuminSearchResultLine>,
    pub pattern: String,
    pub repository: PathBuf,
    pub case_sensitive: bool,
    pub file_extensions: Option<Vec<String>>,
    pub include_globs: Option<Vec<String>>,
    pub exclude_globs: Option<Vec<String>>,
    pub before_context: Option<usize>,
    pub after_context: Option<usize>,
}

/// Result of a code search operation
///
/// Contains all matches found along with the search parameters that were used.
/// This provides a complete picture of both the search configuration and results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    /// Total number of lines that matched the search pattern
    ///
    /// This is the count of all matching lines, even if some were skipped due to pagination.
    /// It can be used to determine the total number of matches without processing all the results.
    pub total_match_line_number: usize,

    /// List of search matches found
    pub matches: Vec<LuminSearchResultLine>,

    /// The search pattern that was used
    pub pattern: String,

    /// Path to the repository that was searched
    pub repository: String,

    /// Whether the search was case-sensitive
    pub case_sensitive: bool,

    /// File extensions filter that was applied (if any)
    pub file_extensions: Option<Vec<String>>,

    /// Glob patterns used to include files in the search (if any)
    /// These are patterns like "**/*.rs" or "src/**/*.md" that filter which files are searched
    pub include_globs: Option<Vec<String>>,

    /// Directories or glob patterns excluded from the search (if any)
    /// These are typically directory names that are converted to glob patterns like "**/dirname/**"
    pub exclude_globs: Option<Vec<String>>,

    /// Number of lines of context included before each match
    pub before_context: Option<usize>,

    /// Number of lines of context included after each match
    pub after_context: Option<usize>,
}

impl CodeSearchResult {
    /// Creates a new CodeSearchResult from parameters
    ///
    /// # Parameters
    ///
    /// * `params` - CodeSearchParams containing all the necessary fields
    pub fn new(params: CodeSearchParams) -> Self {
        Self {
            total_match_line_number: params.total_match_line_number,
            matches: params.search_result_lines,
            pattern: params.pattern,
            repository: params.repository.display().to_string(),
            case_sensitive: params.case_sensitive,
            file_extensions: params.file_extensions,
            include_globs: params.include_globs,
            exclude_globs: params.exclude_globs,
            before_context: params.before_context,
            after_context: params.after_context,
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
