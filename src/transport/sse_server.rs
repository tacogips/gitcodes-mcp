use crate::tools::CargoDocRouter;
use anyhow::Result;
use rmcp::transport::sse_server::SseServer;
use std::net::SocketAddr;

pub struct SseServerApp {
    bind_addr: SocketAddr,
}

impl SseServerApp {
    pub fn new(bind_addr: SocketAddr) -> Self {
        Self { bind_addr }
    }

    pub async fn serve(self) -> Result<()> {
        let sse_server = SseServer::serve(self.bind_addr).await?;
        let cancellation_token = sse_server.with_service(CargoDocRouter::new);

        // Wait for Ctrl+C signal to gracefully shutdown
        tokio::signal::ctrl_c().await?;

        // Cancel the server
        cancellation_token.cancel();

        Ok(())
    }
}
