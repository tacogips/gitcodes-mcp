use rmcp::{Error as McpError, ServerHandler, model::*, tool};
use std::path::PathBuf;

pub mod error;
pub mod responses;

/// Wrapper for GitHub code tools exposed through the MCP protocol
#[derive(Clone)]
pub struct GitDbTools {
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
}

impl GitDbTools {
    /// Creates a new GitDbTools instance with optional authentication and custom repository cache dir
    pub fn new(github_token: Option<String>, repository_cache_dir: Option<PathBuf>) -> Self {
        Self {
            github_token,
            repository_cache_dir,
        }
    }
}

#[tool(tool_box)]
impl GitDbTools {
    /// Placeholder tool to ensure compilation
    #[tool(description = "Health check endpoint to verify the MCP server is running")]
    fn health_check(&self) -> String {
        "GitDB MCP Server is running".to_string()
    }
}

#[tool(tool_box)]
impl ServerHandler for GitDbTools {
    /// Provides information about this MCP server
    fn get_info(&self) -> ServerInfo {
        let auth_status = match &self.github_token {
            Some(_) => "Authenticated with GitHub token",
            None => "Not authenticated (rate limits apply)",
        };

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(format!("GitDB MCP Server - {}", auth_status)),
        }
    }
}

/// Helper method to create a CallToolResult for successful responses
fn success_result(json: String) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Helper method to create a CallToolResult for error responses
fn error_result(message: impl Into<String>) -> Result<CallToolResult, McpError> {
    let error_message = message.into();
    Ok(CallToolResult::error(vec![Content::text(error_message)]))
}
