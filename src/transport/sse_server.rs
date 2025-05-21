use crate::tools::GitHubCodeTools;
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

    pub async fn serve(self) -> Result<()> {
        let sse_server = SseServer::serve(self.bind_addr).await?;
        let github_token = self.github_token.clone();
        let repository_cache_dir = self.repository_cache_dir.clone();
        let cancellation_token = sse_server.with_service(move || {
            GitHubCodeTools::new(github_token.clone(), repository_cache_dir.clone())
        });

        // Wait for Ctrl+C signal to gracefully shutdown
        tokio::signal::ctrl_c().await?;

        // Cancel the server
        cancellation_token.cancel();

        Ok(())
    }
}
