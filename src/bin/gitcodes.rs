use anyhow::Result;
use clap::{Parser, Subcommand};
use std::net::SocketAddr;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Parser)]
#[command(author, version = "0.1.0", about, long_about = None)]
#[command(propagate_version = true)]
#[command(disable_version_flag = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the server in stdin/stdout mode
    Stdio {
        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,

        /// GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
        #[arg(short = 't', long)]
        github_token: Option<String>,
        
        /// Custom directory for storing repository cache data
        /// Defaults to system temp directory if not specified
        #[arg(short = 'c', long = "cache-dir")]
        repository_cache_dir: Option<std::path::PathBuf>,
    },
    /// Run the server with HTTP/SSE interface
    Http {
        /// Address to bind the HTTP server to
        #[arg(short, long, default_value = "0.0.0.0:8080")]
        address: String,

        /// Enable debug logging
        #[arg(short, long)]
        debug: bool,

        /// GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
        #[arg(short = 't', long)]
        github_token: Option<String>,
        
        /// Custom directory for storing repository cache data
        /// Defaults to system temp directory if not specified
        #[arg(short = 'c', long = "cache-dir")]
        repository_cache_dir: Option<std::path::PathBuf>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Stdio {
            debug,
            github_token,
            repository_cache_dir,
        } => run_stdio_server(debug, github_token, repository_cache_dir).await,
        Commands::Http {
            address,
            debug,
            github_token,
            repository_cache_dir,
        } => run_http_server(address, debug, github_token, repository_cache_dir).await,
    }
}

async fn run_stdio_server(
    debug: bool, 
    github_token: Option<String>,
    repository_cache_dir: Option<std::path::PathBuf>
) -> Result<()> {
    // Initialize the global repository manager at startup
    // This ensures a single process_id is used throughout the application lifetime
    let _ = gitcodes_mcp::gitcodes::repository_manager::instance::init_repository_manager(github_token.clone(), repository_cache_dir.clone());
    // Initialize the tracing subscriber with stderr logging
    let level = if debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(level.into()))
        .with_writer(std::io::stderr) // Explicitly use stderr for logging
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false) // Disable ANSI color codes
        .init();

    tracing::info!("Starting MCP documentation server in STDIN/STDOUT mode");
    if github_token.is_some() {
        tracing::info!("Using GitHub token from command line arguments");
    }
    
    if let Some(dir) = &repository_cache_dir {
        tracing::info!("Using custom repository cache directory: {}", dir.display());
    }

    // Run the server using the new rust-sdk implementation
    gitcodes_mcp::transport::stdio::run_stdio_server(github_token, repository_cache_dir)
        .await
        .map_err(|e| anyhow::anyhow!("Error running STDIO server: {}", e))
}

async fn run_http_server(
    address: String, 
    debug: bool, 
    github_token: Option<String>,
    repository_cache_dir: Option<std::path::PathBuf>
) -> Result<()> {
    // Initialize the global repository manager at startup
    // This ensures a single process_id is used throughout the application lifetime
    let _ = gitcodes_mcp::gitcodes::repository_manager::instance::init_repository_manager(github_token.clone(), repository_cache_dir.clone());
    // Setup tracing
    let level = if debug { "debug" } else { "info" };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{},{}", level, env!("CARGO_CRATE_NAME")).into()),
        )
        .with(tracing_subscriber::fmt::layer().with_ansi(false)) // Disable ANSI color codes
        .init();

    // Parse socket address
    let addr: SocketAddr = address.parse()?;

    tracing::debug!("Rust Documentation Server listening on {}", addr);
    tracing::info!(
        "Access the Rust Documentation Server at http://{}/sse",
        addr
    );

    if github_token.is_some() {
        tracing::info!("Using GitHub token from command line arguments");
    }
    
    if let Some(dir) = &repository_cache_dir {
        tracing::info!("Using custom repository cache directory: {}", dir.display());
    }

    // Create app and run server using the new rust-sdk implementation
    let app = gitcodes_mcp::transport::sse_server::SseServerApp::new(addr, github_token, repository_cache_dir);
    app.serve().await?;

    Ok(())
}
