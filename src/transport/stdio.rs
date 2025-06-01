use crate::tools::GitDbTools;
use anyhow::Result;
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use std::path::PathBuf;

pub async fn run_stdio_server(
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
) -> Result<()> {
    // Create an instance of our GitHub code tools wrapper with the provided token and cache dir
    let service = GitDbTools::new(github_token, repository_cache_dir);

    // Use the new rust-sdk stdio transport implementation
    let server = service.serve(stdio()).await?;

    server.waiting().await?;
    Ok(())
}
