//! Error types for the GitHub code tools MCP server
//!
//! This module defines error types that provide more structured
//! information about failures that might occur during tool execution.

use std::fmt;

/// Error types that can occur in the GitHub code tools
#[derive(Debug)]
pub enum ToolError {
    /// Invalid provider (only GitHub is currently supported)
    InvalidProvider(String),
    
    /// Error parsing repository location
    InvalidRepositoryLocation(String),
    
    /// Error cloning or accessing repository
    RepositoryError(String),
    
    /// Error in code search operation
    CodeSearchError(String),
    
    /// Error accessing file contents
    FileAccessError(String),
    
    /// Error parsing API response
    ParseError(String),
    
    /// Error serializing response
    SerializationError(String),
    
    /// Generic error for other failure cases
    Other(String),
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ToolError::InvalidProvider(provider) => write!(f, "Invalid provider: '{}'. Currently only 'github' is supported.", provider),
            ToolError::InvalidRepositoryLocation(details) => write!(f, "Invalid repository location: {}", details),
            ToolError::RepositoryError(details) => write!(f, "Repository error: {}", details),
            ToolError::CodeSearchError(details) => write!(f, "Code search error: {}", details),
            ToolError::FileAccessError(details) => write!(f, "File access error: {}", details),
            ToolError::ParseError(details) => write!(f, "Parse error: {}", details),
            ToolError::SerializationError(details) => write!(f, "Serialization error: {}", details),
            ToolError::Other(details) => write!(f, "Error: {}", details),
        }
    }
}

impl std::error::Error for ToolError {}

/// Convert from ToolError to a plain String for the MCP tool function result
impl From<ToolError> for String {
    fn from(error: ToolError) -> Self {
        error.to_string()
    }
}