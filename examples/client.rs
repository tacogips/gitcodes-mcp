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
// The MCP protocol follows the JSON-RPC 2.0 specification with additional methods
// specific to the Model Context Protocol. For more details on the protocol, see:
// https://modelcontextprotocol.io/introduction
//
// This client implements two transports:
// - STDIO: Direct process-to-process communication via stdin/stdout
// - HTTP/SSE: HTTP for requests, Server-Sent Events for responses

/// Simple example client for interacting with the gitcodes-mcp server via stdin/stdout
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

    // The MCP protocol requires clients to first send an initialize request
    // This establishes the protocol version and capabilities
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

    // Read and process the initialize response
    let mut init_response = String::new();
    stdout.read_line(&mut init_response).await?;
    println!("Initialize response: {}", init_response);

    // Parse the response to check for errors
    if let Ok(json_response) = serde_json::from_str::<Value>(&init_response) {
        if json_response.get("error").is_some() {
            eprintln!("Error in initialize response: {}", json_response["error"]);
            // In a real client, you might want to handle this error gracefully
        }
    }

    // After successful initialization, send the initialized notification
    // This tells the server that the client is ready to proceed
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

    // Get list of available tools
    // This is optional but useful to discover what tools are available
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
    println!("Tool list response received.");

    // Parse and pretty-print the tools response
    if let Ok(json_response) = serde_json::from_str::<Value>(&tools_response) {
        if let Some(error) = json_response.get("error") {
            eprintln!("Error listing tools: {}", error);
        } else if let Some(result) = json_response.get("result") {
            println!("Available tools: {}", serde_json::to_string_pretty(result)?);
        }
    }

    // Example 1: Send a request to lookup tokio crate documentation
    let tokio_request = json!({
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

    println!("\nSending request to look up tokio crate...");
    stdin
        .write_all(tokio_request.to_string().as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read the response
    let mut tokio_response = String::new();
    stdout.read_line(&mut tokio_response).await?;

    // Parse and handle the response
    if let Ok(parsed) = serde_json::from_str::<Value>(&tokio_response) {
        if let Some(error) = parsed.get("error") {
            eprintln!("Error looking up tokio crate: {}", error);
        } else {
            println!("Received tokio crate documentation");
            // In a real client, you would typically display this documentation to the user
            // For brevity, we're not printing the full response as it can be quite large
        }
    }

    // Example 2: Search for GitHub repositories
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

    println!("\nSending request to search GitHub repositories...");
    stdin
        .write_all(github_request.to_string().as_bytes())
        .await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read the response
    let mut github_response = String::new();
    stdout.read_line(&mut github_response).await?;

    // Parse and pretty-print the GitHub search results
    if let Ok(github_parsed) = serde_json::from_str::<Value>(&github_response) {
        if let Some(error) = github_parsed.get("error") {
            eprintln!("Error searching GitHub repositories: {}", error);
        } else if let Some(result) = github_parsed.get("result") {
            println!("GitHub repositories found:");
            if let Some(data) = result.get("data") {
                if let Some(repos) = data.get("repositories") {
                    if let Some(repos_array) = repos.as_array() {
                        for (i, repo) in repos_array.iter().enumerate() {
                            println!(
                                "{}. {} - {}",
                                i + 1,
                                repo["full_name"].as_str().unwrap_or("Unknown"),
                                repo["description"].as_str().unwrap_or("No description")
                            );
                        }
                    }
                }
            }
        }
    }

    // Example 3: Grep code in a repository
    let grep_request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "grep_repository",
            "arguments": {
                "repository": "tokio-rs/tokio",
                "ref_name": "master",
                "pattern": "AsyncRead",
                "case_sensitive": true,
                "use_regex": false,
                "file_extensions": ["rs"],
                "exclude_dirs": ["target", ".github"],
                "before_context": 1,
                "after_context": 1
            }
        },
        "id": 4
    });

    println!("\nSending request to grep code in tokio repository...");
    stdin.write_all(grep_request.to_string().as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await?;

    // Read the response
    let mut grep_response = String::new();
    stdout.read_line(&mut grep_response).await?;

    // Parse and display a summary of the grep results
    if let Ok(grep_parsed) = serde_json::from_str::<Value>(&grep_response) {
        if let Some(error) = grep_parsed.get("error") {
            eprintln!("Error grepping repository: {}", error);
        } else if let Some(result) = grep_parsed.get("result") {
            if let Some(data) = result.get("data") {
                if let Some(stats) = data.get("stats") {
                    println!("\nGrep Results Summary:");
                    println!("Files searched: {}", stats["files_searched"]);
                    println!("Total matches: {}", stats["total_matches"]);
                    println!("Files with matches: {}", stats["files_with_matches"]);
                    println!("Execution time: {} ms", stats["execution_time_ms"]);
                }

                // Show the first few matches as examples
                if let Some(matches) = data.get("matches").and_then(|m| m.as_array()) {
                    println!("\nExample matches:");
                    for (i, file_match) in matches.iter().take(2).enumerate() {
                        println!("{}. File: {}", i + 1, file_match["path"]);
                        if let Some(line_matches) = file_match["line_matches"].as_array() {
                            for (j, line_match) in line_matches.iter().take(2).enumerate() {
                                println!(
                                    "   {}. Line {}: {}",
                                    j + 1,
                                    line_match["line_number"],
                                    line_match["line"].as_str().unwrap_or("<binary data>")
                                );
                            }
                            if line_matches.len() > 2 {
                                println!(
                                    "   ... and {} more matches in this file",
                                    line_matches.len() - 2
                                );
                            }
                        }
                    }
                    if matches.len() > 2 {
                        println!("... and {} more files with matches", matches.len() - 2);
                    }
                }
            }
        }
    }

    // Clean up by terminating the child process
    child.kill().await?;

    Ok(())
}

/// Simple example client for interacting with the gitcodes-mcp server via HTTP/SSE
async fn http_sse_client() -> Result<()> {
    println!("Connecting to HTTP/SSE server...");

    // Create HTTP client with a reasonable timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    // Start our own server with a unique port for testing
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

    // Check if server is running with a health check
    let server_check = client
        .get("http://127.0.0.1:8081/health")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;

    match server_check {
        Ok(_) => println!("Server is running."),
        Err(e) => {
            println!("Could not connect to server health check: {}", e);
            println!("This example may not work properly if the server isn't running.");
            println!("Continuing with the example anyway...");
        }
    }

    // IMPORTANT: In an actual implementation, clients should use a proper
    // Server-Sent Events (SSE) client library to establish and maintain the
    // event stream connection. This example demonstrates the API pattern but
    // doesn't implement the full SSE protocol.
    println!("\n--- HTTP/SSE CLIENT IMPORTANT NOTES ---");
    println!("This is a simplified example that demonstrates the basic API structure.");
    println!("A proper SSE client implementation would:");
    println!("1. Establish an EventSource connection to the /sse endpoint");
    println!("2. Listen for server-sent events (SSE) on that connection");
    println!("3. Send JSON-RPC requests over a separate HTTP channel");
    println!("4. Process responses that come through the SSE event stream");
    println!("----------------------------------------\n");

    // Generate a unique session ID for this client session
    let rand_num: u32 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    let session_id = format!("test_session_{}", rand_num);

    println!("Using session ID: {}", session_id);

    // Step 1: Send initialize request first
    // This establishes the protocol version and capabilities
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

            // Try to clean up by stopping the server process
            tokio::spawn(async {
                let _ = tokio::process::Command::new("pkill")
                    .args(["-f", "gitcodes-mcp http"])
                    .status()
                    .await;
            });

            return Ok(());
        }
    };

    // Process the initialize response
    if init_response.status().is_success() {
        println!("Initialize request sent successfully");
        let init_result = init_response.text().await?;
        println!("Initialize response: {}", init_result);

        // In a real client, you would validate the response and check for errors
        if let Ok(parsed) = serde_json::from_str::<Value>(&init_result) {
            if parsed.get("error").is_some() {
                eprintln!("Error in initialize response: {}", parsed["error"]);
            }
        }
    } else {
        // Handle error response
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

        // Clean up by stopping the server process
        tokio::spawn(async {
            let _ = tokio::process::Command::new("pkill")
                .args(["-f", "gitcodes-mcp http"])
                .status()
                .await;
        });

        return Ok(());
    }

    // Step 2: Send initialized notification
    // This tells the server that the client is ready to proceed
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

    // Process the notification response (usually empty for notifications)
    if notify_response.status().is_success() {
        println!("Initialized notification sent successfully");
    } else {
        println!(
            "Error sending initialized notification: {}",
            notify_response.status()
        );
    }

    // Step 3: Get list of available tools
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

    // Process the tools list response
    if tools_response.status().is_success() {
        println!("Tools list request sent successfully");
        let tools_body = tools_response.text().await?;
        if !tools_body.is_empty() {
            // In a real SSE client, you wouldn't get a direct response here
            // Instead, you'd receive the response as an event on the SSE channel
            println!("Tools list response: {}", tools_body);

            if let Ok(json_value) = serde_json::from_str::<Value>(&tools_body) {
                println!(
                    "Available tools: {}",
                    serde_json::to_string_pretty(&json_value)?
                );
            }
        } else {
            println!("Empty response body (expected for SSE pattern)");
            println!("In a full SSE implementation, the response would come via the event stream");
        }
    } else {
        println!("Error getting tools list: {}", tools_response.status());
    }

    // Step 4: Send a request to search for GitHub repositories
    let request_url = format!("{}?sessionId={}", sse_url, session_id);
    let search_request = json!({
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

    println!("\nSending request to search for GitHub repositories...");
    let response = client
        .post(&request_url)
        .json(&search_request)
        .send()
        .await?;

    // Process the search response
    if response.status().is_success() {
        println!("Request sent successfully");

        // Try to read the response body
        let response_body = response.text().await?;
        if !response_body.is_empty() {
            // In a real SSE client, this would come through the event stream
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

    // Provide guidance on implementing a full SSE client
    println!("\n--- IMPLEMENTING A PROPER SSE CLIENT ---");
    println!("For a complete HTTP/SSE client implementation, you would need to:");
    println!("1. Use a library that supports SSE (EventSource) connections");
    println!("2. Establish a persistent SSE connection to /sse?sessionId=<id>");
    println!("3. Listen for events on that connection and parse them as JSON-RPC responses");
    println!("4. Send requests via HTTP POST to the same endpoint");
    println!("5. Match request IDs with response IDs to correlate requests and responses");
    println!("----------------------------------------");

    // Clean up by stopping the server process
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
    println!("Gitcodes MCP Client Example");
    println!("-------------------------------");
    println!("This example demonstrates how to interact with the gitcodes-mcp server");
    println!("using both STDIO and HTTP/SSE transports.\n");

    // Run STDIO client test
    println!("1. Testing STDIN/STDOUT client:");
    if let Err(e) = stdio_client().await {
        println!("Error in STDIN/STDOUT client: {}", e);
    }

    println!("\n2. Testing HTTP/SSE client:");
    if let Err(e) = http_sse_client().await {
        println!("Error in HTTP/SSE client: {}", e);
    }

    Ok(())
}
