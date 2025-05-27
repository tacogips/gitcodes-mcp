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
    /// use lumin::view::FileContents;
    /// 
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
                    .map(|line_content| format!("{}:{}", line_content.line_number, line_content.line))
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
            FileContents::Binary { message, metadata } => {
                CompactFileContentsResponse {
                    response_type: "binary".to_string(),
                    line_contents: message,
                    metadata: CompactFileMetadata {
                        file_path,
                        line_count: 0,
                        size: metadata.size_bytes as usize,
                    },
                }
            }
            FileContents::Image { message, metadata } => {
                CompactFileContentsResponse {
                    response_type: "image".to_string(),
                    line_contents: message,
                    metadata: CompactFileMetadata {
                        file_path,
                        line_count: 0,
                        size: metadata.size_bytes as usize,
                    },
                }
            }
        }
    }
}
