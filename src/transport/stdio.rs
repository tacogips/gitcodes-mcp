use crate::tools::gitcodes::GitHubCodeTools;
use anyhow::Result;
use rmcp::transport::stdio;
use rmcp::ServiceExt;

pub async fn run_stdio_server(github_token: Option<String>) -> Result<()> {
    // Create an instance of our GitHub code tools wrapper with the provided token
    let service = GitHubCodeTools::new(github_token);

    // Use the new rust-sdk stdio transport implementation
    let server = service.serve(stdio()).await?;

    server.waiting().await?;
    Ok(())
}
