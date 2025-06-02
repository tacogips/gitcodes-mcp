# Rust MCP SDK Documentation

## Overview

The Rust MCP SDK provides a Rust implementation of the Model Context Protocol, enabling developers to build MCP servers and clients in Rust. This project uses `rmcp` version 0.1, which is a clean and improved implementation with tokio async runtime support.

## RMCP - The Rust MCP SDK

### Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-sse-server", "transport-io"] }
```

### Key Features

- **Async First**: Built on tokio async runtime for excellent performance
- **Type Safe**: Correct and intact data types following MCP specification
- **Macro Support**: Tool macros for easy tool declaration
- **Multiple Transports**: Supports stdio, SSE, and child process transports
- **Tower Integration**: Compatible with tower middleware ecosystem

### Feature Flags

- `client`: Enable client-side SDK functionality
- `server`: Enable server-side SDK functionality
- `macros`: Enable macro support (default)
- `transport-io`: Server stdio transport
- `transport-sse-server`: Server SSE transport
- `transport-child-process`: Client stdio transport
- `transport-sse`: Client SSE transport

## Quick Start Examples

### Creating a Simple MCP Client

```rust
use rmcp::{ServiceExt, transport::child_process::TokioChildProcess};
use tokio::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start a client in one line
    let client = ().serve(
        TokioChildProcess::new(
            Command::new("npx")
                .arg("-y")
                .arg("@modelcontextprotocol/server-everything")
        )?
    ).await?;
    
    // Request roots
    let roots = client.list_roots().await?;
    println!("Roots: {:?}", roots);
    
    // Wait for shutdown
    let quit_reason = client.waiting().await?;
    Ok(())
}
```

### Creating an MCP Server with Tools

```rust
use rmcp::{ServerHandler, model::ServerInfo, schemars, tool};
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct Calculator;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct SumRequest {
    #[schemars(description = "the left hand side number")]
    pub a: i32,
    #[schemars(description = "the right hand side number")]
    pub b: i32,
}

// Create a toolbox for storing tool attributes
#[tool(tool_box)]
impl Calculator {
    // Async tool function
    #[tool(description = "Calculate the sum of two numbers")]
    async fn sum(&self, #[tool(aggr)] SumRequest { a, b }: SumRequest) -> String {
        (a + b).to_string()
    }

    // Sync tool function
    #[tool(description = "Calculate the difference of two numbers")]
    fn sub(
        &self,
        #[tool(param)]
        #[schemars(description = "the left hand side number")]
        a: i32,
        #[tool(param)]
        #[schemars(description = "the right hand side number")]
        b: i32,
    ) -> String {
        (a - b).to_string()
    }
}

// Implement ServerHandler
#[tool(tool_box)]
impl ServerHandler for Calculator {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            name: "calculator".to_string(),
            version: "0.1.0".to_string(),
            instructions: Some("A simple calculator MCP server".into()),
            ..Default::default()
        }
    }
}
```

### Starting the Server

```rust
use rmcp::ServiceExt;
use tokio::io::{stdin, stdout};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create transport from stdin/stdout
    let transport = (stdin(), stdout());
    
    // Create service
    let service = Calculator;
    
    // Serve the service
    let server = service.serve(transport).await?;
    
    // Wait for shutdown
    let quit_reason = server.waiting().await?;
    Ok(())
}
```

## Transport Options

### 1. Standard I/O Transport
```rust
use tokio::io::{stdin, stdout};
let transport = (stdin(), stdout());
```

### 2. SSE Server Transport (with axum)
```rust
use rmcp::transport::sse_server::SseServer;
use axum::Router;

let sse_server = SseServer::new();
let router = Router::new()
    .route("/sse", sse_server.route());
```

### 3. Child Process Transport (for clients)
```rust
use rmcp::transport::child_process::TokioChildProcess;
use tokio::process::Command;

let transport = TokioChildProcess::new(
    Command::new("path/to/mcp-server")
)?;
```

## Tool Declaration Patterns

### Using Tool Macros

The `#[tool]` macro simplifies tool declaration:

```rust
#[tool(tool_box)]
impl MyService {
    #[tool(description = "Get current time")]
    async fn get_time(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
    
    #[tool(description = "Process data")]
    fn process(
        &self,
        #[tool(param)]
        #[schemars(description = "Input data")]
        data: String,
        #[tool(param)]
        #[schemars(description = "Processing options")]
        options: ProcessOptions,
    ) -> Result<String, String> {
        // Process and return result
        Ok(format!("Processed: {}", data))
    }
}
```

### Tool Parameters

- `#[tool(param)]`: Mark individual parameters
- `#[tool(aggr)]`: Aggregate multiple parameters from a struct
- Use `schemars` attributes for parameter descriptions

## Error Handling

Tools can return:
- Any type implementing `IntoCallToolResult`
- Types implementing `IntoContents` (automatically marked as success)
- `Result<T, E>` where both `T` and `E` implement `IntoContents`

```rust
#[tool(description = "Divide two numbers")]
fn divide(&self, a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 {
        Err("Division by zero".to_string())
    } else {
        Ok(a / b)
    }
}
```

## Managing Multiple Services

Convert services to dynamic types for collection management:

```rust
let service1 = Calculator.into_dyn();
let service2 = FileManager.into_dyn();
let services = vec![service1, service2];
```

## Best Practices

1. **Use Async Functions**: Leverage tokio's async runtime for non-blocking operations
2. **Descriptive Tool Names**: Use clear, action-oriented names for tools
3. **Schema Descriptions**: Always provide detailed descriptions for parameters
4. **Error Messages**: Return meaningful error messages for debugging
5. **Resource Cleanup**: Properly handle shutdown and cleanup

## Comparison with Official SDK

The `rmcp` crate offers several advantages over the official SDK:
- Better async runtime integration with tokio
- More complete and correct data types
- Cleaner API design for general-purpose use
- Powerful macro system for tool declaration
- Better documentation and examples

## Resources

- **rmcp Documentation**: https://docs.rs/rmcp
- **rmcp GitHub**: https://github.com/4t145/rmcp
- **MCP Specification**: https://spec.modelcontextprotocol.io
- **Official MCP Site**: https://modelcontextprotocol.io

## Example Projects

Check the `examples/` directory in the rmcp repository for complete examples:
- Simple calculator server
- File system server
- Database integration
- Multi-service management