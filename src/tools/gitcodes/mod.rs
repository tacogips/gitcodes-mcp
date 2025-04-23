use std::sync::Arc;

use html2md::parse_html;

use reqwest::Client;
use tokio::sync::Mutex;

use rmcp::{model::*, schemars, tool, ServerHandler};

// Cache for documentation lookups to avoid repeated requests
#[derive(Clone)]
pub struct DocCache {
    cache: Arc<Mutex<std::collections::HashMap<String, String>>>,
}

impl Default for DocCache {
    fn default() -> Self {
        Self::new()
    }
}

impl DocCache {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let cache = self.cache.lock().await;
        cache.get(key).cloned()
    }

    pub async fn set(&self, key: String, value: String) {
        let mut cache = self.cache.lock().await;
        cache.insert(key, value);
    }
}

#[derive(Clone)]
pub struct CargoDocRouter {
    pub client: Client,
    pub cache: DocCache,
}

impl Default for CargoDocRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[tool(tool_box)]
impl CargoDocRouter {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            cache: DocCache::new(),
        }
    }

    #[tool(description = "This is example")]
    async fn this_is_example_tool(
        &self,
        #[tool(param)]
        #[schemars(description = "The name of the crate to look up")]
        crate_name: String,

        #[tool(param)]
        #[schemars(description = "The version of the crate (optional, defaults to latest)")]
        version: Option<String>,
    ) -> String {
        // Check cache first
        let cache_key = if let Some(ver) = &version {
            format!("{}:{}", crate_name, ver)
        } else {
            crate_name.clone()
        };

        if let Some(doc) = self.cache.get(&cache_key).await {
            return doc;
        }

        // Construct the docs.rs URL for the crate
        let url = if let Some(ver) = version {
            format!("https://docs.rs/crate/{}/{}/", crate_name, ver)
        } else {
            format!("https://docs.rs/crate/{}/", crate_name)
        };

        // Fetch the documentation page
        let response = match self
            .client
            .get(&url)
            .header(
                "User-Agent",
                "gitcodes/0.1.0 (https://github.com/d6e/gitcodes-mcp)",
            )
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => return format!("Failed to fetch documentation: {}", e),
        };

        if !response.status().is_success() {
            return format!(
                "Failed to fetch documentation. Status: {}",
                response.status()
            );
        }

        let html_body = match response.text().await {
            Ok(body) => body,
            Err(e) => return format!("Failed to read response body: {}", e),
        };

        // Convert HTML to markdown
        let markdown_body = parse_html(&html_body);

        // Cache the markdown result
        self.cache.set(cache_key, markdown_body.clone()).await;

        markdown_body
    }
}

#[tool(tool_box)]
impl ServerHandler for CargoDocRouter {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Rust Documentation MCP Server for accessing Rust crate documentation.".to_string(),
            ),
        }
    }
}
