use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::str::FromStr;
use tokio;
use tracing_subscriber::{self, EnvFilter};

use gitcodes_mcp::gitcodes::{repository_manager, RepositoryLocation};
use gitcodes_mcp::tools::{OrderOption, SortOption};

#[derive(Parser)]
#[command(author, version = "0.1.0", about = "GitCodes CLI for GitHub and repository operations", long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// GitHub API token for authentication (overrides GITCODE_MCP_GITHUB_TOKEN environment variable)
    #[arg(short = 't', long, global = true)]
    github_token: Option<String>,

    /// Custom directory for storing repository cache data
    /// Defaults to system temp directory if not specified
    #[arg(short = 'c', long = "cache-dir", global = true)]
    repository_cache_dir: Option<PathBuf>,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search for GitHub repositories
    Search {
        /// Search query for repositories
        #[arg(
            help = "Search query - keywords to search for repositories. Can include advanced search qualifiers like 'language:rust' or 'stars:>1000'"
        )]
        query: String,

        /// How to sort results (default is 'relevance')
        #[arg(long, value_enum, default_value = "relevance")]
        sort_by: Option<SortOptionArg>,

        /// Sort order (default is 'descending')
        #[arg(long, value_enum, default_value = "descending")]
        order: Option<OrderOptionArg>,

        /// Results per page (default is 30, max 100)
        #[arg(long, default_value = "30")]
        per_page: Option<u8>,

        /// Result page number (default is 1)
        #[arg(long, default_value = "1")]
        page: Option<u32>,
    },
    /// Search code in a GitHub repository
    Grep {
        /// Repository URL or local file path
        #[arg(
            help = "Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths"
        )]
        repository_location: String,

        /// Branch, Commit or tag (default is 'main' or 'master')
        #[arg(short, long)]
        ref_name: Option<String>,

        /// Search pattern - the text pattern to search for in the code
        #[arg(
            help = "Search pattern - the text pattern to search for in the code. Supports regular expressions by default"
        )]
        pattern: String,

        /// Whether to be case-sensitive
        #[arg(long, default_value = "false")]
        case_sensitive: Option<bool>,

        /// Whether to use regex
        #[arg(long, default_value = "true")]
        use_regex: Option<bool>,

        /// File extensions to search
        #[arg(short = 'e', long = "ext", value_delimiter = ',')]
        file_extensions: Option<Vec<String>>,

        /// Directories to exclude from search
        #[arg(long = "exclude", value_delimiter = ',')]
        exclude_dirs: Option<Vec<String>>,
    },
    /// List branches and tags for a GitHub repository
    ListRefs {
        /// Repository URL or local file path
        #[arg(
            help = "Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths"
        )]
        repository_location: String,
    },
}

/// Sorting options for repository search
#[derive(clap::ValueEnum, Clone, Debug)]
enum SortOptionArg {
    Relevance,
    Stars,
    Forks,
    Updated,
}

/// Order options for repository search
#[derive(clap::ValueEnum, Clone, Debug)]
enum OrderOptionArg {
    Ascending,
    Descending,
}

impl From<SortOptionArg> for SortOption {
    fn from(value: SortOptionArg) -> Self {
        match value {
            SortOptionArg::Relevance => SortOption::Relevance,
            SortOptionArg::Stars => SortOption::Stars,
            SortOptionArg::Forks => SortOption::Forks,
            SortOptionArg::Updated => SortOption::Updated,
        }
    }
}

impl From<OrderOptionArg> for OrderOption {
    fn from(value: OrderOptionArg) -> Self {
        match value {
            OrderOptionArg::Ascending => OrderOption::Ascending,
            OrderOptionArg::Descending => OrderOption::Descending,
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.debug {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(level.into()))
        .with_writer(std::io::stderr) // Use stderr for logging
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    // Initialize the global repository manager at startup
    // This ensures a single process_id is used throughout the application lifetime
    let manager = repository_manager::instance::init_repository_manager(
        cli.github_token.clone(),
        cli.repository_cache_dir.clone(),
    );

    tracing::info!("GitCodes CLI initialized");
    if cli.github_token.is_some() {
        tracing::info!("Using GitHub token from command line arguments");
    }

    if let Some(dir) = &cli.repository_cache_dir {
        tracing::info!("Using custom repository cache directory: {}", dir.display());
    }

    // Process the command
    match cli.command {
        Commands::Search {
            query,
            sort_by,
            order,
            per_page,
            page,
        } => {
            tracing::info!("Searching for repositories with query: {}", query);
            // Call the search_repositories implementation
            // NOTE: Placeholder response since the actual implementation is temporarily disabled
            println!("Search functionality is temporarily disabled during refactoring.");
            println!("Query: {}", query);
            if let Some(sort) = sort_by {
                println!("Sort by: {:?}", sort);
            }
            if let Some(order) = order {
                println!("Order: {:?}", order);
            }
            Ok(())
        }
        Commands::Grep {
            repository_location,
            ref_name,
            pattern,
            case_sensitive,
            use_regex,
            file_extensions,
            exclude_dirs,
        } => {
            tracing::info!(
                "Searching for code pattern in repository: {}",
                repository_location
            );
            tracing::info!("Pattern: {}", pattern);
            if let Some(r) = &ref_name {
                tracing::info!("Ref: {}", r);
            }

            // Parse the repository location
            match RepositoryLocation::from_str(&repository_location) {
                Ok(repo_location) => {
                    // Prepare the repository (clone or reuse local)
                    match manager.prepare_repository(repo_location, ref_name).await {
                        Ok(local_repo) => {
                            tracing::info!(
                                "Repository prepared at: {}",
                                local_repo.get_repository_dir().display()
                            );
                            // NOTE: Placeholder response since the actual implementation is temporarily disabled
                            println!("Search code functionality is temporarily disabled during refactoring.");
                            println!("Repository: {}", repository_location);
                            println!("Pattern: {}", pattern);
                            Ok(())
                        }
                        Err(e) => {
                            tracing::error!("Failed to prepare repository: {}", e);
                            anyhow::bail!("Failed to prepare repository: {}", e)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Invalid repository location: {}", e);
                    anyhow::bail!("Invalid repository location: {}", e)
                }
            }
        }
        Commands::ListRefs {
            repository_location,
        } => {
            tracing::info!("Listing references for repository: {}", repository_location);

            // Parse the repository location
            match RepositoryLocation::from_str(&repository_location) {
                Ok(repo_location) => {
                    // Prepare the repository (clone or reuse local)
                    match manager.prepare_repository(repo_location, None).await {
                        Ok(local_repo) => {
                            tracing::info!(
                                "Repository prepared at: {}",
                                local_repo.get_repository_dir().display()
                            );
                            // NOTE: Placeholder response since the actual implementation is temporarily disabled
                            println!("Repository refs listing functionality is temporarily disabled during refactoring.");
                            println!("Repository: {}", repository_location);
                            Ok(())
                        }
                        Err(e) => {
                            tracing::error!("Failed to prepare repository: {}", e);
                            anyhow::bail!("Failed to prepare repository: {}", e)
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Invalid repository location: {}", e);
                    anyhow::bail!("Invalid repository location: {}", e)
                }
            }
        }
    }
}
