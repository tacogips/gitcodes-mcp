use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{self, EnvFilter};
use serde_json;

use gitcodes_mcp::gitcodes::repository_manager;
use gitcodes_mcp::gitcodes::LocalRepository;
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
    RepositorySearch {
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

/// Helper function to clean up a repository
/// 
/// This function handles the cleanup of a local repository, including logging.
/// 
/// # Arguments
/// 
/// * `repo` - The LocalRepository to clean up
fn cleanup_repository(repo: LocalRepository) {
    if let Err(err) = repo.cleanup() {
        tracing::warn!("Failed to clean up repository: {}", err);
    } else {
        tracing::debug!("Successfully cleaned up repository");
    }
}

/// Helper function to clean up an optional repository
///
/// This function handles the cleanup of an optional local repository, including logging.
///
/// # Arguments
///
/// * `repo_opt` - The Option<LocalRepository> to clean up if Some
fn cleanup_repository_opt(repo_opt: Option<LocalRepository>) {
    if let Some(repo) = repo_opt {
        cleanup_repository(repo);
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
        Commands::RepositorySearch {
            query,
            sort_by,
            order,
            per_page,
            page,
        } => {
            use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;

            // Default to GitHub as provider
            let git_provider = GitProvider::Github;

            // Convert from clap enum types to the types used by repository_manager
            let sort_option = sort_by.map(|s| s.into());
            let order_option = order.map(|o| o.into());

            // Execute the search using the repository manager
            match manager
                .search_repositories(
                    git_provider,
                    query,
                    sort_option,
                    order_option,
                    per_page,
                    page,
                )
                .await
            {
                Ok(result) => {
                    // Format and print a more user-friendly output from the JSON result
                    match serde_json::from_str::<serde_json::Value>(&result) {
                        Ok(json) => {
                            // Check if the expected structure exists
                            if let Some(items) = json.get("items").and_then(|i| i.as_array()) {
                                for (i, repo) in items.iter().enumerate() {
                                    let name = repo.get("full_name").and_then(|n| n.as_str()).unwrap_or("<unnamed>");
                                    let description = repo.get("description").and_then(|d| d.as_str()).unwrap_or("<no description>");
                                    let url = repo.get("html_url").and_then(|u| u.as_str()).unwrap_or("<no url>");
                                    let stars = repo.get("stargazers_count").and_then(|s| s.as_u64()).unwrap_or(0);
                                    
                                    println!("{}. {} - {} stars", i + 1, name, stars);
                                    println!("   Description: {}", description);
                                    println!("   URL: {}", url);
                                    println!();
                                }
                            } else {
                                // Fallback to printing raw JSON if structure isn't as expected
                                println!("{}", result);
                            }
                        },
                        Err(_) => {
                            // Fallback to printing raw result if it's not valid JSON
                            println!("{}", result);
                        }
                    }
                    Ok(())
                },
                Err(err) => {
                    tracing::error!("Search failed: {}", err);
                    anyhow::bail!("Search failed: {}", err)
                },
            }
        }
        Commands::Grep {
            repository_location,
            ref_name,
            pattern,
            case_sensitive,
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

            // Use the services module to perform the grep operation
            use gitcodes_mcp::services;

            match services::perform_grep_in_repository(
                manager,
                &repository_location,
                pattern,
                ref_name.as_deref(),
                case_sensitive.unwrap_or(false),
                file_extensions.as_ref(),
                exclude_dirs.as_ref(),
            )
            .await
            {
                Ok((result, local_repo)) => {
                    // Just print each match in a simple format: file:line:content
                    if !result.matches.is_empty() {
                        for m in &result.matches {
                            println!(
                                "{}:{}:{}",
                                m.file_path.display(),
                                m.line_number,
                                m.line_content
                            );
                        }
                    } else {
                        // Let user know if no matches were found
                        eprintln!("No matches found.");
                    }
                    
                    // Clean up the repository when finished
                    cleanup_repository(local_repo);

                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to search code: {}", e);
                    anyhow::bail!("Failed to search code: {}", e)
                }
            }
        }
        Commands::ListRefs {
            repository_location,
        } => {
            tracing::info!("Listing references for repository: {}", repository_location);

            // Get the refs directly from the repository manager
            let (refs_json, local_repo_opt) = manager
                .list_repository_refs(&repository_location)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;

            // Format and print the JSON result in a more user-friendly way
            match serde_json::from_str::<serde_json::Value>(&refs_json) {
                Ok(json) => {
                    if let Some(refs) = json.as_array() {
                        println!("{:<40} {:<7} {:<40}", "Reference", "Type", "SHA");
                        println!("{:-<40} {:-<7} {:-<40}", "", "", "");
                        
                        for ref_obj in refs {
                            let ref_name = ref_obj.get("ref").and_then(|r| r.as_str()).unwrap_or("<unknown>");
                            let obj_type = ref_obj.get("object").and_then(|o| o.get("type")).and_then(|t| t.as_str()).unwrap_or("<unknown>");
                            let sha = ref_obj.get("object").and_then(|o| o.get("sha")).and_then(|s| s.as_str()).unwrap_or("<unknown>");
                            
                            // Format the reference name to be more readable
                            let display_ref = ref_name.replace("refs/heads/", "branch: ").replace("refs/tags/", "tag: ");
                            
                            println!("{:<40} {:<7} {:<40}", display_ref, obj_type, sha);
                        }
                    } else {
                        // Fallback to raw JSON if structure isn't as expected
                        println!("{}", refs_json);
                    }
                },
                Err(_) => {
                    // Fallback to raw JSON if parsing fails
                    println!("{}", refs_json);
                }
            }
            
            // Clean up the repository if it exists
            cleanup_repository_opt(local_repo_opt);

            Ok(())
        }
    }
}
