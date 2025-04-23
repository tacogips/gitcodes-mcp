use crate::tools::cargo_docs::CargoDocRouter;
use anyhow::Result;
use rmcp::transport::stdio;
use rmcp::ServiceExt;

pub async fn run_stdio_server() -> Result<()> {
    // Create an instance of our documentation router
    let service = CargoDocRouter::new();

    // Use the new rust-sdk stdio transport implementation
    let server = service.serve(stdio()).await?;

    server.waiting().await?;
    Ok(())
}
