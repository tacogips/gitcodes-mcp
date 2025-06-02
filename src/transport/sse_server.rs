use crate::tools::GitDbTools;
use anyhow::Result;
use rmcp::transport::sse_server::SseServer;
use std::net::SocketAddr;
use std::path::PathBuf;

pub struct SseServerApp {
    bind_addr: SocketAddr,
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
}

impl SseServerApp {
    /// Creates a new SSE server application instance.
    ///
    /// # Arguments
    ///
    /// * `bind_addr` - The socket address to bind the server to
    /// * `github_token` - Optional GitHub personal access token for API authentication
    /// * `repository_cache_dir` - Optional directory for caching repository data
    ///
    /// # Returns
    ///
    /// Returns a new SseServerApp instance.
    pub fn new(
        bind_addr: SocketAddr,
        github_token: Option<String>,
        repository_cache_dir: Option<PathBuf>,
    ) -> Self {
        Self {
            bind_addr,
            github_token,
            repository_cache_dir,
        }
    }

    /// Starts the SSE server and serves GitDbTools over Server-Sent Events.
    ///
    /// This method starts the server and waits for a Ctrl+C signal to shutdown gracefully.
    ///
    /// # Returns
    ///
    /// Returns Ok(()) when the server shuts down gracefully.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The server fails to bind to the specified address
    /// - The server encounters an error during operation
    pub async fn serve(self) -> Result<()> {
        let sse_server = SseServer::serve(self.bind_addr).await?;
        let github_token = self.github_token.clone();
        let repository_cache_dir = self.repository_cache_dir.clone();
        let cancellation_token = sse_server.with_service(move || {
            GitDbTools::new(github_token.clone(), repository_cache_dir.clone())
        });

        // Wait for Ctrl+C signal to gracefully shutdown
        tokio::signal::ctrl_c().await?;

        // Cancel the server
        cancellation_token.cancel();

        Ok(())
    }
}
