//! Response types for the GitHub code tools MCP server
//!
//! This module defines structured response types that are returned by the
//! MCP tool methods. These types help ensure that the JSON responses are
//! consistently formatted and properly typed.
//!
//! # Response Types Overview
//!
//! ## Code Search Responses
//! - [`CodeSearchResponse`]: Direct alias to `CodeSearchResult` (legacy format)
//! - [`CompactCodeSearchResponse`]: New compact format that groups results by file
//!
//! ## Issue Search Responses
//! - [`CompactIssueSearchResponse`]: Compact format with simplified issue structure
//! - [`CompactIssueItem`]: Simplified issue item with flattened user, labels, and repository data
//!
//! ## File Content Responses  
//! - [`FileContentsResponse`]: Direct alias to `FileContents` (legacy format)
//! - [`CompactFileContentsResponse`]: Compact format with concatenated line contents
//!
//! ## Repository Information Responses
//! - [`RepositoryRefsResponse`]: Lists branches and tags for a repository
//! - [`ReferenceInfo`]: Information about individual git references
//!
//! # Compact Response Formats
//!
//! The compact response types provide more efficient JSON representations by:
//! - Grouping related data together (e.g., lines by file)
//! - Concatenating line contents with line numbers
//! - Reducing overall JSON verbosity while preserving all essential information
//!
//! These formats are particularly useful for MCP tool responses where network
//! efficiency and readability are important considerations.

use crate::gitcodes::CodeSearchResult;
use crate::gitcodes::repository_manager::providers::IssueSearchResults;
use lumin::view::FileContents;
use serde::{Deserialize, Serialize};

/// Response for the grep_repository tool (legacy format)
///
/// This type directly uses the CodeSearchResult struct from the gitcodes crate
/// to ensure consistency. This is the original verbose format that includes
/// individual SearchResultLine entries.
///
/// **Note**: The `grep_repository` tool now uses [`CompactCodeSearchResponse`]
/// by default for more efficient JSON output. This type is kept for compatibility.
#[allow(dead_code)]
pub type CodeSearchResponse = CodeSearchResult;

/// Compact response for the grep_repository tool
///
/// This provides a more concise format where search results are grouped by file
/// and line contents are concatenated into a single string with line numbers.
///
/// # Format
///
/// The response structure is:
/// ```json
/// {
///   "total_match_line_number": 5,
///   "matches": [
///     {"file_path": "src/main.rs", "lines": "10:fn main() {\n11:    println!(\"Hello, world!\");"}
///   ],
///   "pattern": "main",
///   "case_sensitive": false,
///   "file_extensions": null,
///   "include_globs": ["**/*.rs"],
///   "exclude_globs": ["**/target/**"],
///   "before_context": 0,
///   "after_context": 1
/// }
/// ```
///
/// # Line Format
///
/// Lines are formatted as `"{line_number}:{content}"` and joined with newline characters.
/// Both actual matches and context lines are included in the concatenated string.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactCodeSearchResponse {
    /// Total number of lines that matched the search pattern
    pub total_match_line_number: usize,

    /// List of search matches grouped by file
    pub matches: Vec<CompactFileMatch>,

    /// The search pattern that was used
    pub pattern: String,

    /// Whether the search was case-sensitive
    pub case_sensitive: bool,

    /// File extensions filter that was applied (if any)
    pub file_extensions: Option<Vec<String>>,

    /// Glob patterns used to include files in the search (if any)
    pub include_globs: Option<Vec<String>>,

    /// Directories or glob patterns excluded from the search (if any)
    pub exclude_globs: Option<Vec<String>>,

    /// Number of lines of context included before each match
    pub before_context: Option<usize>,

    /// Number of lines of context included after each match
    pub after_context: Option<usize>,
}

/// A file match containing grouped lines for the compact response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactFileMatch {
    /// Path to the file containing the matches
    pub file_path: String,

    /// Concatenated line contents with line numbers
    /// Format: "{line_number}:{content}\n{line_number}:{content}..."
    pub lines: String,
}

/// Compact response for the search_issues tool
///
/// This provides a more concise format where issue data is simplified by
/// flattening nested user, label, and repository information into simple strings.
///
/// # Format
///
/// The response structure is:
/// ```json
/// {
///   "total_count": 2,
///   "incomplete_results": false,
///   "items": [
///     {
///       "id": "1234567890",
///       "number": 123,
///       "title": "Memory leak in async task handling",
///       "body": "When using async tasks...",
///       "state": "open",
///       "user": "developer123",
///       "labels": ["bug", "memory"],
///       "comments": 5,
///       "html_url": "https://github.com/example/repo/issues/123",
///       "created_at": "2024-01-15T10:30:00Z",
///       "updated_at": "2024-01-20T14:45:00Z",
///       "closed_at": null,
///       "score": 95.5,
///       "repository_url": "https://github.com/example-org/awesome-project",
///       "repository_name": "awesome-project"
///     }
///   ]
/// }
/// ```
///
/// # Simplifications
///
/// - User information is flattened to just the login name string
/// - Labels are simplified to an array of label name strings
/// - Repository information is flattened to just URL and name strings
/// - Assignee and milestone information is omitted for brevity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactIssueSearchResponse {
    /// Total number of matching issues
    pub total_count: u64,

    /// Indicates if the result was incomplete due to rate limiting
    pub incomplete_results: bool,

    /// List of simplified issue items
    pub items: Vec<CompactIssueItem>,
}

/// Simplified issue item for compact response format
///
/// This structure flattens the complex nested data from the full IssueItem
/// into a more streamlined format suitable for MCP tool responses.
///
/// # Simplifications
///
/// - User data is reduced to just the login name
/// - Labels are simplified to just the label names
/// - Repository data is reduced to URL and name only
/// - Assignee and milestone information is omitted
/// - All essential issue metadata is preserved
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactIssueItem {
    /// Issue ID (unique within the provider)
    pub id: String,

    /// Issue number within the repository
    pub number: u64,

    /// Issue title
    pub title: String,

    /// Issue body/description
    pub body: Option<String>,

    /// Current state of the issue (open, closed)
    pub state: String,

    /// Issue author login name
    pub user: String,

    /// List of label names attached to the issue
    pub labels: Vec<String>,

    /// Number of comments on the issue
    pub comments: u32,

    /// URL for viewing the issue in a browser
    pub html_url: String,

    /// When the issue was created
    pub created_at: String,

    /// When the issue was last updated
    pub updated_at: String,

    /// When the issue was closed (if applicable)
    pub closed_at: Option<String>,

    /// Score (relevance to search query)
    /// May not be available in all search responses
    pub score: Option<f64>,

    /// URL for viewing the repository in a browser
    pub repository_url: String,

    /// Repository name (without owner)
    pub repository_name: String,
}

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

/// Response for the show_file_contents tool (legacy format)
///
/// This type directly uses the FileContents enum from the lumin crate
/// to ensure consistency with the internal implementation. This is the
/// original verbose format with separate line entries.
///
/// **Note**: The `show_file_contents` tool now uses [`CompactFileContentsResponse`]
/// by default for more efficient JSON output. This type is kept for compatibility.
#[allow(dead_code)]
pub type FileContentsResponse = FileContents;

/// Compact response for the show_file_contents tool
///
/// This provides a more concise format where line contents are concatenated
/// into a single string with line numbers, and includes enhanced metadata.
///
/// # Format
///
/// The response structure is:
/// ```json
/// {
///   "type": "text|binary|image",
///   "line_contents": "1:line content\n2:another line",
///   "metadata": {
///     "file_path": "path/to/file.ext",
///     "line_count": 100,
///     "size": 1234
///   }
/// }
/// ```
///
/// # Line Format
///
/// For text files, line contents are formatted as:
/// - Line numbers with no padding or spaces
/// - Format: `"{line_number}:{content}"`
/// - Lines are joined with newline characters
///
/// For binary and image files, the `line_contents` field contains
/// a descriptive message instead of actual file content.
///
/// # Response Types
///
/// - `"text"`: Regular text files with line-by-line content
/// - `"binary"`: Binary files with descriptive message
/// - `"image"`: Image files with descriptive message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactFileContentsResponse {
    /// Response type identifier ("text", "binary", or "image")
    #[serde(rename = "type")]
    pub response_type: String,

    /// Concatenated line contents with line numbers for text files,
    /// or descriptive message for binary/image files
    pub line_contents: String,

    /// Enhanced metadata including file path and size information
    pub metadata: CompactFileMetadata,
}

/// Enhanced metadata for compact file contents response
///
/// Contains essential information about the file being viewed,
/// including path, line count, and size metrics.
///
/// # Size Field
///
/// The `size` field represents:
/// - For text files: character count from the original file
/// - For binary files: byte count (size_bytes from BinaryMetadata)
/// - For image files: byte count (size_bytes from ImageMetadata)
///
/// # Line Count
///
/// The `line_count` field represents:
/// - For text files: actual number of lines in the file
/// - For binary/image files: always 0 (no line-based content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactFileMetadata {
    /// Full path of the file relative to repository root
    pub file_path: String,

    /// Total number of lines in the file (0 for binary/image files)
    pub line_count: usize,

    /// Total size in bytes/characters depending on file type
    pub size: usize,
}

impl CompactFileContentsResponse {
    /// Convert FileContents to CompactFileContentsResponse
    ///
    /// Transforms the verbose lumin::view::FileContents format into a compact
    /// representation suitable for MCP tool responses.
    ///
    /// # Arguments
    ///
    /// * `file_contents` - The original FileContents from lumin crate
    /// * `file_path` - Full file path to include in metadata
    ///
    /// # Returns
    ///
    /// A CompactFileContentsResponse with:
    /// - Concatenated line contents for text files
    /// - Descriptive messages for binary/image files
    /// - Enhanced metadata with file path and size information
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gitcodes_mcp::tools::responses::CompactFileContentsResponse;
    /// use lumin::view::{FileContents, TextContent, TextMetadata, LineContent};
    ///
    /// let metadata = TextMetadata {
    ///     line_count: 3,
    ///     char_count: 50,
    /// };
    /// let content = TextContent {
    ///     line_contents: vec![
    ///         LineContent { line_number: 1, line: "fn main() {".to_string() },
    ///         LineContent { line_number: 2, line: "    println!(\"Hello\");".to_string() },
    ///         LineContent { line_number: 3, line: "}".to_string() },
    ///     ],
    /// };
    /// let file_contents = FileContents::Text { content, metadata };
    /// let compact = CompactFileContentsResponse::from_file_contents(
    ///     file_contents,
    ///     "src/main.rs".to_string()
    /// );
    /// ```
    pub fn from_file_contents(file_contents: FileContents, file_path: String) -> Self {
        match file_contents {
            FileContents::Text { content, metadata } => {
                // Convert line contents to concatenated string with line numbers
                let line_contents = content
                    .line_contents
                    .iter()
                    .map(|line_content| {
                        format!("{}:{}", line_content.line_number, line_content.line)
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                CompactFileContentsResponse {
                    response_type: "text".to_string(),
                    line_contents,
                    metadata: CompactFileMetadata {
                        file_path,
                        line_count: metadata.line_count,
                        size: metadata.char_count,
                    },
                }
            }
            FileContents::Binary { message, metadata } => CompactFileContentsResponse {
                response_type: "binary".to_string(),
                line_contents: message,
                metadata: CompactFileMetadata {
                    file_path,
                    line_count: 0,
                    size: metadata.size_bytes as usize,
                },
            },
            FileContents::Image { message, metadata } => CompactFileContentsResponse {
                response_type: "image".to_string(),
                line_contents: message,
                metadata: CompactFileMetadata {
                    file_path,
                    line_count: 0,
                    size: metadata.size_bytes as usize,
                },
            },
        }
    }
}

impl CompactIssueSearchResponse {
    /// Convert IssueSearchResults to CompactIssueSearchResponse
    ///
    /// Transforms the verbose IssueSearchResults format into a compact
    /// representation suitable for MCP tool responses by flattening nested structures.
    ///
    /// # Arguments
    ///
    /// * `search_results` - The original IssueSearchResults
    ///
    /// # Returns
    ///
    /// A CompactIssueSearchResponse with:
    /// - Simplified issue items with flattened user, label, and repository data
    /// - All original search metadata preserved
    /// - More concise JSON structure for network efficiency
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gitcodes_mcp::tools::responses::CompactIssueSearchResponse;
    /// use gitcodes_mcp::gitcodes::repository_manager::providers::IssueSearchResults;
    ///
    /// let search_results = IssueSearchResults {
    ///     total_count: 1,
    ///     incomplete_results: false,
    ///     items: vec![/* issue items */],
    /// };
    /// let compact = CompactIssueSearchResponse::from_search_results(search_results);
    /// ```
    pub fn from_search_results(search_results: IssueSearchResults) -> Self {
        let items = search_results
            .items
            .into_iter()
            .map(|issue| CompactIssueItem {
                id: issue.id,
                number: issue.number,
                title: issue.title,
                body: issue.body,
                state: issue.state,
                user: issue.user.login,
                labels: issue.labels.into_iter().map(|label| label.name).collect(),
                comments: issue.comments,
                html_url: issue.html_url,
                created_at: issue.created_at,
                updated_at: issue.updated_at,
                closed_at: issue.closed_at,
                score: issue.score,
                repository_url: issue.repository.html_url,
                repository_name: issue.repository.name,
            })
            .collect();

        CompactIssueSearchResponse {
            total_count: search_results.total_count,
            incomplete_results: search_results.incomplete_results,
            items,
        }
    }
}

impl CompactCodeSearchResponse {
    /// Convert CodeSearchResult to CompactCodeSearchResponse
    ///
    /// Transforms the verbose CodeSearchResult format into a compact
    /// representation suitable for MCP tool responses by grouping lines by file.
    ///
    /// # Arguments
    ///
    /// * `search_result` - The original CodeSearchResult
    ///
    /// # Returns
    ///
    /// A CompactCodeSearchResponse with:
    /// - Search results grouped by file path
    /// - Concatenated line contents for each file
    /// - All original search metadata preserved
    ///
    /// # Examples
    ///
    /// ```rust
    /// use gitcodes_mcp::tools::responses::CompactCodeSearchResponse;
    /// use gitcodes_mcp::gitcodes::CodeSearchResult;
    /// use lumin::search::SearchResultLine;
    /// use std::path::PathBuf;
    ///
    /// let search_result = CodeSearchResult {
    ///     total_match_line_number: 1,
    ///     matches: vec![
    ///         SearchResultLine {
    ///             file_path: PathBuf::from("src/main.rs"),
    ///             line_number: 1,
    ///             line_content: "fn main() {".to_string(),
    ///             content_omitted: false,
    ///             is_context: false,
    ///         }
    ///     ],
    ///     pattern: "main".to_string(),
    ///     repository: "test_repo".to_string(),
    ///     case_sensitive: false,
    ///     file_extensions: None,
    ///     include_globs: None,
    ///     exclude_globs: None,
    ///     before_context: None,
    ///     after_context: None,
    /// };
    /// let compact = CompactCodeSearchResponse::from_search_result(search_result);
    /// ```
    pub fn from_search_result(search_result: CodeSearchResult) -> Self {
        use std::collections::HashMap;

        // Group search result lines by file path
        let mut file_groups: HashMap<String, Vec<String>> = HashMap::new();

        for result_line in search_result.matches {
            let file_path = result_line.file_path.display().to_string();
            let line_content = format!("{}:{}", result_line.line_number, result_line.line_content);

            file_groups
                .entry(file_path)
                .or_insert_with(Vec::new)
                .push(line_content);
        }

        // Convert grouped lines to CompactFileMatch structs
        let matches = file_groups
            .into_iter()
            .map(|(file_path, lines)| CompactFileMatch {
                file_path,
                lines: lines.join("\n"),
            })
            .collect();

        CompactCodeSearchResponse {
            total_match_line_number: search_result.total_match_line_number,
            matches,
            pattern: search_result.pattern,
            case_sensitive: search_result.case_sensitive,
            file_extensions: search_result.file_extensions,
            include_globs: search_result.include_globs,
            exclude_globs: search_result.exclude_globs,
            before_context: search_result.before_context,
            after_context: search_result.after_context,
        }
    }
}
