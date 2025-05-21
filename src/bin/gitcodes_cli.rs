use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::{self, EnvFilter};
use lumin::view::FileContents;

use gitcodes_mcp::gitcodes::repository_manager;
use gitcodes_mcp::gitcodes::LocalRepository;
use gitcodes_mcp::gitcodes::local_repository::prevent_directory_traversal;
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
    
    /// Show verbose output including INFO-level logs
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    /// Keep local repository clones after operation instead of cleaning them up
    #[arg(short = 'k', long = "preserve-repos", global = true, default_value_t = false)]
    preserve_repos: bool,

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
            help = "Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths (both absolute and relative paths are supported, but '..' is not allowed for security reasons)"
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
        
        /// Number of lines to include before each match
        #[arg(short = 'B', long = "before-context")]
        before_context: Option<usize>,
        
        /// Number of lines to include after each match
        #[arg(short = 'A', long = "after-context")]
        after_context: Option<usize>,
    },
    /// Show the contents of a file in a GitHub repository
    ShowFile {
        /// Repository URL or local file path
        #[arg(
            help = "Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths (both absolute and relative paths are supported, but '..' is not allowed for security reasons)"
        )]
        repository_location: String,

        /// Branch, Commit or tag (default is 'main' or 'master')
        #[arg(short, long)]
        ref_name: Option<String>,

        /// Path to the file within the repository
        #[arg(
            help = "Path to the file within the repository, e.g., 'README.md', 'src/main.rs'. Can start with or without a slash."
        )]
        file_path: String,

        /// Maximum file size in bytes to read
        #[arg(long)]
        max_size: Option<usize>,

        /// Start viewing from this line number (1-indexed)
        #[arg(short = 'f', long = "from-line")]
        line_from: Option<usize>,

        /// End viewing at this line number (1-indexed, inclusive)
        #[arg(short = 'l', long = "to-line")]
        line_to: Option<usize>,
        
        /// Show text content without line numbers (default: false = show line numbers)
        #[arg(short = 'p', long = "plain", default_value_t = false)]
        without_line_numbers: bool,
    },
    /// List branches and tags for a GitHub repository
    ListRefs {
        /// Repository URL or local file path
        #[arg(
            help = "Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths (both absolute and relative paths are supported, but '..' is not allowed for security reasons)"
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
/// It silences warnings for explicitly provided local paths.
/// Will not clean up if preserve_repos is true.
/// 
/// # Arguments
/// 
/// * `repo` - The LocalRepository to clean up
/// * `preserve_repos` - If true, repositories will not be cleaned up
fn cleanup_repository(repo: LocalRepository, preserve_repos: bool) {
    if preserve_repos {
        tracing::info!("Preserving repository at user request (--preserve-repos flag was used)");
        return;
    }
    
    if let Err(err) = repo.cleanup() {
        // Only log warnings for temporary repositories (not user-provided local paths)
        if err.contains("doesn't match temporary repository pattern") {
            // This is an explicitly provided local path, suppress the warning
            tracing::debug!("Skipping cleanup for explicitly provided local path: {}", err);
        } else {
            tracing::warn!("Failed to clean up repository: {}", err);
        }
    } else {
        tracing::debug!("Successfully cleaned up repository");
    }
}

/// Helper function to clean up an optional repository
///
/// This function handles the cleanup of an optional local repository, including logging.
/// Will not clean up if preserve_repos is true.
///
/// # Arguments
///
/// * `repo_opt` - The Option<LocalRepository> to clean up if Some
/// * `preserve_repos` - If true, repositories will not be cleaned up
fn cleanup_repository_opt(repo_opt: Option<LocalRepository>, preserve_repos: bool) {
    if let Some(repo) = repo_opt {
        cleanup_repository(repo, preserve_repos);
    }
}

/// Safely converts a repository location string to an absolute path if it's a relative path
/// 
/// This function handles relative paths securely by:
/// 1. Using prevent_directory_traversal to reject directory traversal attempts
/// 2. Converting relative paths to absolute using the current working directory
/// 3. Leaving absolute paths unchanged
/// 
/// # Arguments
/// 
/// * `repository_location` - The repository location string (could be a URL or path)
/// 
/// # Returns
/// 
/// * `Result<String, String>` - The processed repository location or an error message
fn process_repository_location(repository_location: &str) -> Result<String, String> {
    // Skip processing if it doesn't look like a file path (e.g., URLs or GitHub shortcuts)
    if repository_location.starts_with("http://") || 
       repository_location.starts_with("https://") || 
       repository_location.starts_with("git@") || 
       repository_location.starts_with("github:") {
        return Ok(repository_location.to_string());
    }
    
    // Convert the location to a path for validation
    let path = std::path::Path::new(repository_location);
    let path_buf = PathBuf::from(repository_location);
    
    // Security check: Use the standalone prevent_directory_traversal function
    if let Err(e) = prevent_directory_traversal(&path_buf) {
        return Err(format!("Security error: {}", e));
    }
    
    // If the path is already absolute, just return it
    if path.is_absolute() {
        return Ok(repository_location.to_string());
    }
    
    // Convert relative path to absolute using current working directory
    match std::env::current_dir() {
        Ok(current_dir) => {
            match current_dir.join(path).canonicalize() {
                Ok(absolute_path) => {
                    // Convert path to string, handling non-UTF8 paths
                    match absolute_path.to_str() {
                        Some(path_str) => Ok(path_str.to_string()),
                        None => Err(format!("Path '{}' contains invalid Unicode characters", repository_location))
                    }
                },
                Err(e) => Err(format!("Failed to resolve path '{}': {}", repository_location, e))
            }
        },
        Err(e) => Err(format!("Failed to get current working directory: {}", e))
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let level = if cli.debug {
        tracing::Level::DEBUG
    } else if cli.verbose {
        tracing::Level::INFO
    } else {
        tracing::Level::WARN  // Only show warnings and errors by default
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

    tracing::debug!("GitCodes CLI initialized");
    if cli.github_token.is_some() {
        tracing::debug!("Using GitHub token from command line arguments");
    }

    if let Some(dir) = &cli.repository_cache_dir {
        tracing::debug!("Using custom repository cache directory: {}", dir.display());
    }
    
    if cli.preserve_repos {
        tracing::debug!("Repository preservation mode enabled - repositories will not be cleaned up");
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
                    
                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Search failed: {}", err);
                    let suggestion = if error_msg.contains("API rate limit") {
                        "\nSuggestion: You may have exceeded GitHub's API rate limits. Try the following:\n  - Use a GitHub token with '-t' option\n  - Wait a few minutes and try again\n  - Reduce the number of requests"
                    } else if error_msg.contains("authentication") || error_msg.contains("401") {
                        "\nSuggestion: Authentication failed. Try the following:\n  - Check that your GitHub token is valid and has not expired\n  - Ensure the token has appropriate permissions\n  - Regenerate your GitHub token if necessary"
                    } else {
                        "\nSuggestion: Check your network connection and GitHub credentials."
                    };
                    
                    anyhow::bail!("{}{}", error_msg, suggestion)
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
            before_context,
            after_context,
        } => {
            tracing::debug!(
                "Searching for code pattern in repository: {}",
                repository_location
            );
            tracing::debug!("Pattern: {}", pattern);
            if let Some(r) = &ref_name {
                tracing::debug!("Ref: {}", r);
            }
            
            // Process the repository location (convert relative paths to absolute)
            let processed_location = match process_repository_location(&repository_location) {
                Ok(location) => location,
                Err(e) => {
                    tracing::error!("Failed to process repository location: {}", e);
                    return Err(anyhow::anyhow!("Failed to process repository location: {}", e));
                }
            };

            // Use the services module to perform the grep operation
            use gitcodes_mcp::services;

            match services::perform_grep_in_repository(
                manager,
                &processed_location,
                pattern,
                ref_name.as_deref(),
                case_sensitive.unwrap_or(false),
                file_extensions.as_ref(),
                exclude_dirs.as_ref(),
                before_context,
                after_context,
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
                        tracing::warn!("No matches found.");
                    }
                    
                    // Clean up the repository when finished (unless preserve flag is set)
                    cleanup_repository(local_repo, cli.preserve_repos);

                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to search code: {}", e);
                    
                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Failed to search code: {}", e);
                    let suggestion = if error_msg.contains("clone") || error_msg.contains("Failed to parse repository location") {
                        if error_msg.contains("invalid remote git url") || error_msg.contains("must be absolute") {
                            "\nSuggestion: The repository location format appears to be invalid. Try the following:\n  - For GitHub: Use 'git@github.com:user/repo.git' (SSH format works most reliably)\n  - Alternative formats: 'https://github.com/user/repo' or 'github:user/repo'\n  - For local repositories: Use an absolute path to an existing local git repository\n  - Relative paths are not supported for security reasons"
                        } else if error_msg.contains("HTTPS") {
                            "\nSuggestion: There was an issue cloning via HTTPS. Try the following:\n  - Use SSH URL format instead: 'git@github.com:user/repo.git'\n  - Check your network connection or firewall settings\n  - Verify the repository exists and is accessible\n  - For private repositories, provide a GitHub token with '-t'"
                        } else if error_msg.contains("authentication") {
                            "\nSuggestion: Authentication failed when cloning the repository. Try the following:\n  - For private repositories, provide a GitHub token with '-t'\n  - Check if your token has the correct permissions\n  - Try using SSH URL format with properly configured SSH keys: 'git@github.com:user/repo.git'"
                        } else {
                            "\nSuggestion: Failed to clone repository. Try the following:\n  - Use SSH URL format: 'git@github.com:user/repo.git' (most reliable)\n  - Check your network connection\n  - Verify the repository exists and is accessible\n  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)\n  - Check if the ref/branch/tag exists in the repository"
                        }
                    } else if error_msg.contains("pattern") {
                        "\nSuggestion: There was an issue with your search pattern. Try the following:\n  - Simplify your search pattern\n  - Escape special regex characters if you're not using regex\n  - Use the --case-sensitive option if needed"
                    } else {
                        "\nSuggestion: Check your repository location and search parameters."
                    };
                    
                    anyhow::bail!("{}{}", error_msg, suggestion)
                }
            }
        }
        Commands::ShowFile {
            repository_location,
            ref_name,
            file_path,
            max_size,
            line_from,
            line_to,
            without_line_numbers,
        } => {
            tracing::debug!(
                "Showing file contents from repository: {}, file: {}",
                repository_location,
                file_path
            );
            if let Some(r) = &ref_name {
                tracing::debug!("Ref: {}", r);
            }
            
            // Process the repository location (convert relative paths to absolute)
            let processed_location = match process_repository_location(&repository_location) {
                Ok(location) => location,
                Err(e) => {
                    tracing::error!("Failed to process repository location: {}", e);
                    return Err(anyhow::anyhow!("Failed to process repository location: {}", e));
                }
            };

            // Use the services module to access the file contents
            use gitcodes_mcp::services;

            // Clone file_path to retain ownership of the original for later use
            let file_path_clone = file_path.clone();
            
            match services::show_file_contents(
                manager,
                &processed_location,
                file_path_clone,
                ref_name.as_deref(),
                max_size,
                line_from,
                line_to,
                Some(without_line_numbers),
            )
            .await
            {
                Ok((file_contents, local_repo, without_line_numbers)) => {
                    // Format and display the file contents based on type
                    match file_contents {
                        FileContents::Text { content, metadata } => {
                            // Display metadata about the file
                            let line_range_info = match (line_from, line_to) {
                                (Some(from), Some(to)) => format!(" (lines {}-{})", from, to),
                                (Some(from), None) => format!(" (from line {})", from),
                                (None, Some(to)) => format!(" (up to line {})", to),
                                (None, None) => String::new(),
                            };
                            
                            println!("File: {}{}", file_path, line_range_info);
                            println!("Type: Text file");
                            println!("Size: {} characters, {} lines", metadata.char_count, metadata.line_count);
                            println!("---");
                            
                            // Print the actual content based on format preference
                            if without_line_numbers {
                                // Plain text format
                                println!("{}", content.to_string());
                            } else {
                                // Line number format (file:line:content)
                                let content_str = content.to_string();
                                let mut line_number = line_from.unwrap_or(1);
                                
                                for line in content_str.lines() {
                                    println!("{file_path}:{line_number}:{line}");
                                    line_number += 1;
                                }
                            }
                        },
                        FileContents::Binary { message, metadata } => {
                            println!("File: {}", file_path);
                            println!("Type: Binary file");
                            println!("Size: {} bytes", metadata.size_bytes);
                            if let Some(mime) = &metadata.mime_type {
                                println!("MIME type: {}", mime);
                            }
                            println!("---");
                            println!("{}", message);
                        },
                        FileContents::Image { message, metadata } => {
                            println!("File: {}", file_path);
                            println!("Type: Image file");
                            // media_type is directly a String, not an Option
                            println!("Media type: {}", metadata.media_type);
                            println!("Size: {} bytes", metadata.size_bytes);
                            println!("---");
                            println!("{}", message);
                            println!("Note: Use a graphical viewer to see this image.");
                        },
                    }
                    
                    // Clean up the repository when finished (unless preserve flag is set)
                    cleanup_repository(local_repo, cli.preserve_repos);

                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to show file contents: {}", e);
                    
                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Failed to show file contents: {}", e);
                    let suggestion = if error_msg.contains("clone") || error_msg.contains("Failed to parse repository location") {
                        if error_msg.contains("invalid remote git url") || error_msg.contains("must be absolute") {
                            "\nSuggestion: The repository location format appears to be invalid. Try the following:\n  - For GitHub: Use 'git@github.com:user/repo.git' (SSH format works most reliably)\n  - Alternative formats: 'https://github.com/user/repo' or 'github:user/repo'\n  - For local repositories: Use an absolute path to an existing local git repository\n  - Relative paths are not supported for security reasons"
                        } else {
                            "\nSuggestion: Could not access repository. Try the following:\n  - Check your network connection\n  - Verify the repository exists and is accessible\n  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)"
                        }
                    } else if error_msg.contains("not found") || error_msg.contains("File not found") {
                        "\nSuggestion: File not found in repository. Try the following:\n  - Check the file path spelling\n  - Check if the file exists in the specified branch or tag\n  - Use the correct path relative to the repository root"
                    } else if error_msg.contains("too large") {
                        "\nSuggestion: File is too large to display. Try the following:\n  - Use the --max-size option to increase the maximum file size\n  - Use the --from-line and --to-line options to view a subset of the file"
                    } else if error_msg.contains(".." ) || error_msg.contains("Invalid path") {
                        "\nSuggestion: Invalid file path. Ensure that:\n  - The path does not contain '..' (parent directory references)\n  - The path is relative to the repository root\n  - The path does not attempt to access outside the repository"
                    } else {
                        "\nSuggestion: Check your repository location and file path."
                    };
                    
                    anyhow::bail!("{}{}", error_msg, suggestion)
                }
            }
        },
        Commands::ListRefs {
            repository_location,
        } => {
            tracing::debug!("Listing references for repository: {}", repository_location);
            
            // Process the repository location (convert relative paths to absolute)
            let processed_location = match process_repository_location(&repository_location) {
                Ok(location) => location,
                Err(e) => {
                    tracing::error!("Failed to process repository location: {}", e);
                    return Err(anyhow::anyhow!("Failed to process repository location: {}", e));
                }
            };

            // Get the refs directly from the repository manager
            let refs_result = manager
                .list_repository_refs(&processed_location)
                .await;
                
            let (refs_json, local_repo_opt) = match refs_result {
                Ok(result) => result,
                Err(e) => {
                    tracing::error!("Failed to list repository references: {}", e);
                    
                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Failed to list repository references: {}", e);
                    let suggestion = if error_msg.contains("clone") || error_msg.contains("Failed to parse repository location") {
                        if error_msg.contains("invalid remote git url") || error_msg.contains("must be absolute") {
                            "\nSuggestion: The repository location format appears to be invalid. Try the following:\n  - For GitHub: Use 'git@github.com:user/repo.git' (SSH format works most reliably)\n  - Alternative formats: 'https://github.com/user/repo' or 'github:user/repo'\n  - For local repositories: Use an absolute path to an existing local git repository\n  - Relative paths are not supported for security reasons"
                        } else {
                            "\nSuggestion: Failed to access repository. Try the following:\n  - Check your network connection\n  - Verify the repository exists and is accessible\n  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)"
                        }
                    } else if error_msg.contains("API") || error_msg.contains("rate limit") {
                        "\nSuggestion: You may have exceeded GitHub's API rate limits. Try the following:\n  - Use a GitHub token with '-t' option\n  - Wait a few minutes and try again"
                    } else if error_msg.contains("not found") || error_msg.contains("404") {
                        "\nSuggestion: Repository not found. Try the following:\n  - Check the spelling of the repository name\n  - Ensure the repository exists and is public (or you have access to it)\n  - Use the correct repository format (user/repo)"
                    } else {
                        "\nSuggestion: Check the repository location and your access permissions."
                    };
                    
                    return Err(anyhow::anyhow!("{}{}", error_msg, suggestion));
                }
            };

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
            
            // Clean up the repository if it exists (unless preserve flag is set)
            cleanup_repository_opt(local_repo_opt, cli.preserve_repos);

            Ok(())
        }
    }
}
