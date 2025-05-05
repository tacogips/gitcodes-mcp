use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

// Example client for gitcodes-MCP
//
// This example demonstrates how to interact with the gitcodes MCP server using both:
// 1. STDIO transport (Process-to-process communication via standard input/output)
// 2. HTTP/SSE transport (HTTP Server-Sent Events)
//
// The client demonstrates the proper MCP protocol sequence:
// - Send initialize request (with capabilities)
// - Process initialization response
// - Send initialized notification
// - List available tools
// - Make tool calls
//
// Requirements:
// - The STDIO client spawns its own server
// - The HTTP/SSE client requires a full EventSource implementation for production use

// Simple example client for interacting with the doc server via stdin/stdout
async fn stdio_client() -> Result<()> {
    // Start the stdio-server in a separate process
    let mut child = tokio::process::Command::new("cargo")
        .args(["run", "--bin", "gitcodes-mcp", "stdio"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let stdin = child.stdin.take().expect("Failed to open stdin");
    let stdout = child.stdout.take().expect("Failed to open stdout");
    let mut stdin = io::BufWriter::new(stdin);
    let mut stdout = BufReader::new(stdout);

    // Send initialize request first
    let initialize_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {},
                "sampling": {}
            },
            "clientInfo": {
                "name": "ExampleClient",
                "version": "1.0.0"
            }
        },
        "id": 0
    });

    println!("Sending initialize request...");
    stdin
        .write_all(initialize_request.to_string().as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read initialize response
    let mut init_response = String::new();
    stdout.read_line(&mut init_response).await?;
    println!("Initialize response: {:?}", init_response);

    // Send initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    println!("Sending initialized notification...");
    stdin
        .write_all(initialized_notification.to_string().as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Get list of available tools first
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    println!("Sending request to list available tools...");
    stdin
        .write_all(list_tools_request.to_string().as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read tools list response
    let mut tools_response = String::new();
    stdout.read_line(&mut tools_response).await?;
    println!("Tools list response: {:?}", tools_response);

    // Send a request to lookup tokio crate using tools/call method
    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "lookup_crate",
            "arguments": {
                "crate_name": "tokio"
            }
        },
        "id": 2
    });

    // Also prepare a GitHub repository search request
    let github_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "search_repositories",
            "arguments": {
                "params": {
                    "query": "rust web framework",
                    "sort_by": "stars",
                    "per_page": 3
                }
            }
        },
        "id": 3
    });

    println!("Sending request to look up tokio crate...");
    stdin.write_all(request.to_string().as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read the response
    let mut response = String::new();
    stdout.read_line(&mut response).await?;

    println!("response {:?}", response);
    let parsed: Value = serde_json::from_str(&response)?;
    println!(
        "Received response: {}",
        serde_json::to_string_pretty(&parsed)?
    );

    // Now send the GitHub repository search request
    println!("Sending request to search GitHub repositories...");
    stdin
        .write_all(github_request.to_string().as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read the response
    let mut github_response = String::new();
    stdout.read_line(&mut github_response).await?;

    println!("GitHub search response: {:?}", github_response);
    let github_parsed: Value = serde_json::from_str(&github_response)?;
    println!(
        "Received GitHub response: {}",
        serde_json::to_string_pretty(&github_parsed)?
    );

    // Terminate the child process
    child.kill().await?;

    Ok(())
}

// Simple example client for interacting with the doc server via HTTP/SSE
async fn http_sse_client() -> Result<()> {
    println!("Connecting to HTTP/SSE server...");

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Start our own server with a unique port
    println!("Starting our own server on port 8081...");
    let _server_handle = tokio::spawn(async {
        match tokio::process::Command::new("cargo")
            .args([
                "run",
                "--bin",
                "gitcodes-mcp",
                "http",
                "--address",
                "127.0.0.1:8081",
            ])
            .spawn()
        {
            Ok(mut child) => {
                // Keep the child process running until we're done
                match child.wait().await {
                    Ok(status) => println!("Server process exited with: {}", status),
                    Err(e) => println!("Error waiting for server: {}", e),
                }
            }
            Err(e) => println!("Failed to start server: {}", e),
        }
    });

    // Use port 8081 for our testing
    let sse_url = "http://127.0.0.1:8081/sse";

    // Give the server some time to start
    println!("Waiting for server to start...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Try to check if server is running
    let server_check = client
        .get("http://127.0.0.1:8081/health")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;

    match server_check {
        Ok(_) => println!("Server appears to be running."),
        Err(_) => {
            println!("Could not connect to server. Health check failed.");
            println!(
                "Note: This example expects a health endpoint at /health, which might not exist."
            );
            println!("Continuing anyway, but expect possible connection errors.");
        }
    }

    println!("\n--- HTTP/SSE CLIENT IMPORTANT NOTES ---");
    println!("This is a simplified example that demonstrates the basic API structure.");
    println!("A proper SSE client implementation would:");
    println!("1. Establish an EventSource connection to the /sse endpoint");
    println!("2. Listen for server-sent events (SSE) on that connection");
    println!("3. Send JSON-RPC requests over a separate HTTP channel");
    println!("4. Process responses that come through the SSE event stream");
    println!("----------------------------------------\n");

    // Generate a random session ID for testing
    let rand_num: u32 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let session_id = format!("test_session_{}", rand_num);

    println!("Using session ID: {}", session_id);

    // First send initialize request
    let init_url = format!("{}?sessionId={}", sse_url, session_id);
    let init_request = json!({
        "jsonrpc": "2.0",
        "method": "initialize",
        "params": {
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {},
                "sampling": {}
            },
            "clientInfo": {
                "name": "ExampleClient",
                "version": "1.0.0"
            }
        },
        "id": 0
    });

    println!("Sending initialize request to HTTP server...");
    let init_response = match client.post(&init_url).json(&init_request).send().await {
        Ok(resp) => resp,
        Err(e) => {
            println!("Failed to send initialize request: {}", e);
            println!("\nIMPORTANT: HTTP/SSE transport requires special handling.");
            println!("The server expects EventSource connections, not regular HTTP requests.");
            println!("If you see connection errors, it's likely because we're not using a proper SSE client.");

            // Try to abort the server process to clean up
            tokio::spawn(async {
                let _ = tokio::process::Command::new("pkill")
                    .args(["-f", "gitcodes-mcp http"])
                    .status()
                    .await;
            });

            return Ok(());
        }
    };

    if init_response.status().is_success() {
        println!("Initialize request sent successfully");
        let init_result = init_response.text().await?;
        println!("Initialize response: {}", init_result);
    } else {
        println!(
            "Error sending initialize request: {}",
            init_response.status()
        );
        println!("\nDIAGNOSTICS:");
        println!("- Status: {}", init_response.status());
        println!(
            "- Error Body: {}",
            init_response.text().await.unwrap_or_default()
        );
        println!("\nNOTE: The HTTP/SSE transport expects:");
        println!("1. An EventSource connection to /sse?sessionId=<id>");
        println!("2. JSON-RPC requests sent as POST requests to same endpoint");
        println!("3. Responses delivered via the SSE event stream, not HTTP responses");

        // Try to abort the server process to clean up
        tokio::spawn(async {
            let _ = tokio::process::Command::new("pkill")
                .args(["-f", "gitcodes-mcp http"])
                .status()
                .await;
        });

        return Ok(());
    }

    // Send initialized notification
    let init_notify_request = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    println!("Sending initialized notification...");
    let notify_response = client
        .post(&init_url)
        .json(&init_notify_request)
        .send()
        .await?;
    if notify_response.status().is_success() {
        println!("Initialized notification sent successfully");
    } else {
        println!(
            "Error sending initialized notification: {}",
            notify_response.status()
        );
    }

    // Get list of available tools first
    let list_tools_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/list",
        "id": 1
    });

    println!("Sending request to list available tools...");
    let tools_response = client
        .post(&init_url)
        .json(&list_tools_request)
        .send()
        .await?;

    if tools_response.status().is_success() {
        println!("Tools list request sent successfully");
        let tools_body = tools_response.text().await?;
        if !tools_body.is_empty() {
            println!("Tools list response: {}", tools_body);

            if let Ok(json_value) = serde_json::from_str::<Value>(&tools_body) {
                println!(
                    "Available tools: {}",
                    serde_json::to_string_pretty(&json_value)?
                );
            }
        }
    }

    // Send a request to search for GitHub repositories
    let request_url = format!("{}?sessionId={}", sse_url, session_id);
    let request_body = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "search_repositories",
            "arguments": {
                "params": {
                    "query": "async runtime",
                    "sort_by": "stars",
                    "per_page": 5
                }
            }
        },
        "id": 2
    });

    println!("Sending request to search for GitHub repositories...");
    let response = client.post(&request_url).json(&request_body).send().await?;

    if response.status().is_success() {
        println!("Request sent successfully");
        // Try to read the response body
        let response_body = response.text().await?;
        if !response_body.is_empty() {
            println!("Response body: {}", response_body);

            // Try to parse as JSON if possible
            if let Ok(json_value) = serde_json::from_str::<Value>(&response_body) {
                println!(
                    "Parsed JSON response: {}",
                    serde_json::to_string_pretty(&json_value)?
                );
            }
        } else {
            println!("Empty response body (expected for SSE pattern)");
            println!(
                "In a full implementation, you would receive the actual response via SSE events"
            );
        }
    } else {
        println!("Error sending request: {}", response.status());
        let error_body = response.text().await?;
        println!("Error response: {}", error_body);
    }

    // Print implementation guidance
    println!("\n--- IMPLEMENTING A PROPER SSE CLIENT ---");
    println!("For a complete HTTP/SSE client implementation, you would need to:");
    println!("1. Use a library that supports SSE (EventSource) connections");
    println!("2. Establish a persistent SSE connection to /sse?sessionId=<id>");
    println!("3. Listen for events on that connection and parse them as JSON-RPC responses");
    println!("4. Send requests via HTTP POST to the same endpoint");
    println!("5. Match request IDs with response IDs to correlate requests and responses");
    println!("----------------------------------------");

    // Try to gracefully terminate the server
    println!("\nCleaning up server process...");
    tokio::spawn(async {
        let _ = tokio::process::Command::new("pkill")
            .args(["-f", "gitcodes-mcp http"])
            .status()
            .await;
    });

    // Wait a moment for server to shut down
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("Rust Documentation Server Client Example");
    println!("---------------------------------------");

    // Run STDIO client test
    println!("\n1. Testing STDIN/STDOUT client:");
    if let Err(e) = stdio_client().await {
        println!("Error in STDIN/STDOUT client: {}", e);
    }

    println!("\n2. Testing HTTP/SSE client:");
    if let Err(e) = http_sse_client().await {
        println!("Error in HTTP/SSE client: {}", e);
    }

    Ok(())
}
