use anyhow::Result;
use clap::{Parser, Subcommand};
use lumin::view::FileContents;
use std::path::PathBuf;
use tracing_subscriber::{self, EnvFilter};

use gitcodes_mcp::gitcodes::local_repository::prevent_directory_traversal;
use gitcodes_mcp::gitcodes::repository_manager;
use gitcodes_mcp::gitcodes::LocalRepository;
use gitcodes_mcp::tools::{IssueSortOption, OrderOption, SortOption};

#[derive(Parser)]
#[command(
    author,
    version = "0.1.0",
    about = "GitCodes CLI for GitHub and repository operations with comprehensive search capabilities",
    long_about = "A command-line tool for GitHub repository operations including comprehensive issue and pull request search, repository search, and code searching with regex patterns."
)]
#[command(propagate_version = true)]
struct Cli {
    /// GitHub API token for authentication (overrides GITCODES_MCP_GITHUB_TOKEN environment variable)
    #[arg(short = 't', long, global = true)]
    github_token: Option<String>,

    /// Custom directory for storing repository cache data
    /// Defaults to system temp directory if not specified
    #[arg(short = 'c', long = "cache-dir", global = true)]
    repository_cache_dir: Option<PathBuf>,

    /// Show verbose output including INFO-level logs
    #[arg(short = 'v', long, global = true)]
    verbose: bool,

    /// Enable debug logging
    #[arg(short, long, global = true)]
    debug: bool,

    /// Keep local repository clones after operation instead of cleaning them up
    #[arg(
        short = 'k',
        long = "preserve-repos",
        global = true,
        default_value_t = false
    )]
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
    /// Search for GitHub issues
    ///
    /// Examples:
    ///   gitcodes-cli issue-search "memory leak" --repository rust-lang/rust --state open --labels bug
    ///   gitcodes-cli issue-search "performance" --creator username --assignee developer
    ///   gitcodes-cli issue-search "documentation" --labels docs,help-wanted
    IssueSearch {
        /// Search query for issues (full-text search only, use other options for qualifiers)
        #[arg(
            help = "Search query - use keywords and phrases for full-text search. Use other options like --repository, --labels, etc. for specific qualifiers instead of embedding them in the query."
        )]
        query: String,

        /// How to sort results (default is 'best-match')
        #[arg(long, value_enum, default_value = "best-match")]
        sort_by: Option<IssueSortOptionArg>,

        /// Sort order (default is 'descending')
        #[arg(long, value_enum, default_value = "descending")]
        order: Option<OrderOptionArg>,

        /// Results per page (default is 30, max 100)
        #[arg(long, default_value = "30")]
        per_page: Option<u8>,

        /// Result page number (default is 1)
        #[arg(long, default_value = "1")]
        /// Optional page number for pagination (defaults to 1)
        page: Option<u32>,

        /// Repository to search in (format: owner/repo)
        #[arg(
            long,
            help = "Limit search to specific repository (e.g., 'rust-lang/rust')"
        )]
        repository: Option<String>,

        /// Labels to filter by (comma-separated)
        #[arg(long, help = "Filter by labels (e.g., 'bug,urgent' or 'enhancement')")]
        labels: Option<String>,

        /// Issue state to filter by
        #[arg(long, help = "Filter by issue state: 'open', 'closed', or 'all'")]
        state: Option<String>,

        /// Filter by issue creator
        #[arg(long, help = "Filter by user who created the issue")]
        creator: Option<String>,

        /// Filter by mentioned user
        #[arg(long, help = "Filter by user mentioned in the issue")]
        mentioned: Option<String>,

        /// Filter by assignee
        #[arg(
            long,
            help = "Filter by assignee: username, 'none' for unassigned, or '*' for any assignee"
        )]
        assignee: Option<String>,

        /// Filter by milestone
        #[arg(
            long,
            help = "Filter by milestone: number, '*' for any milestone, or 'none' for no milestone"
        )]
        milestone: Option<String>,

        /// Filter by issue type
        #[arg(
            long,
            help = "Filter by issue type: type name, '*' for any type, or 'none' for no type"
        )]
        issue_type: Option<String>,
    },
    /// Search code in a GitHub repository using regex patterns and glob file matching
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

        /// File extensions to search (deprecated, use include_globs instead)
        #[arg(short = 'e', long = "ext", value_delimiter = ',')]
        file_extensions: Option<Vec<String>>,

        /// Glob patterns to include in search (e.g., **/*.rs,src/**/*.md)
        #[arg(
            long = "include",
            value_delimiter = ',',
            help = "Glob patterns to include in search. More flexible than file extensions. Examples: '**/*.rs' (all Rust files), 'src/**/*.md' (markdown files in src directory), '{**/*.rs,**/*.toml}' (Rust and TOML files). Use this instead of --ext for more advanced file filtering."
        )]
        include_globs: Option<Vec<String>>,

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
    /// Get the directory tree structure of a repository
    Tree {
        /// Repository URL or local file path
        #[arg(
            help = "Repository URL or local file path - supports GitHub formats: 'https://github.com/user/repo', 'git@github.com:user/repo.git', 'github:user/repo', or local paths (both absolute and relative paths are supported, but '..' is not allowed for security reasons)"
        )]
        repository_location: String,

        /// Branch, commit, or tag (default is 'main' or 'master')
        #[arg(short, long)]
        ref_name: Option<String>,

        /// Whether file path matching should be case sensitive (default: false)
        #[arg(long, default_value_t = false)]
        case_sensitive: bool,

        /// Whether to respect .gitignore files (default: true)
        #[arg(long, default_value_t = true)]
        respect_gitignore: bool,

        /// Maximum depth of directory traversal (default: unlimited)
        #[arg(short = 'D', long)]
        depth: Option<usize>,

        /// Whether to strip the repository path prefix from results (default: true)
        #[arg(long, default_value_t = true)]
        strip_path_prefix: bool,

        /// Relative path within the repository to start the tree from (optional)
        #[arg(long)]
        search_relative_path: Option<String>,
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

/// Sorting options for issue search
#[derive(clap::ValueEnum, Clone, Debug)]
enum IssueSortOptionArg {
    Created,
    Updated,
    Comments,
    #[value(name = "best-match")]
    BestMatch,
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

impl From<IssueSortOptionArg> for IssueSortOption {
    fn from(value: IssueSortOptionArg) -> Self {
        match value {
            IssueSortOptionArg::Created => IssueSortOption::Created,
            IssueSortOptionArg::Updated => IssueSortOption::Updated,
            IssueSortOptionArg::Comments => IssueSortOption::Comments,
            IssueSortOptionArg::BestMatch => IssueSortOption::BestMatch,
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
            tracing::debug!(
                "Skipping cleanup for explicitly provided local path: {}",
                err
            );
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
/// * `repo_opt` - The `Option<LocalRepository>` to clean up if Some
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
    if repository_location.starts_with("http://")
        || repository_location.starts_with("https://")
        || repository_location.starts_with("git@")
        || repository_location.starts_with("github:")
    {
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
                        None => Err(format!(
                            "Path '{}' contains invalid Unicode characters",
                            repository_location
                        )),
                    }
                }
                Err(e) => Err(format!(
                    "Failed to resolve path '{}': {}",
                    repository_location, e
                )),
            }
        }
        Err(e) => Err(format!("Failed to get current working directory: {}", e)),
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
        tracing::Level::ERROR // Only show errors by default (no INFO or WARN)
    };

    // If verbose is not enabled, completely silence the INFO level logs from our repository_manager
    let env_filter = if cli.verbose || cli.debug {
        // Show logs based on the level determined above
        EnvFilter::from_default_env().add_directive(level.into())
    } else {
        // Silent mode - hide INFO logs from repository_manager
        EnvFilter::from_default_env()
            .add_directive(level.into())
            .add_directive(
                "gitcodes_mcp::gitcodes::repository_manager=error"
                    .parse()
                    .unwrap(),
            )
    };

    // Configure a minimal custom logging format - just level and message
    tracing_subscriber::fmt()
        .with_env_filter(env_filter) // Use our custom filter
        .with_writer(std::io::stderr) // Use stderr for logging
        .with_thread_ids(false) // Hide thread IDs
        .with_target(false) // Hide module targets
        .with_file(false) // Hide file names
        .with_line_number(false) // Hide line numbers
        .with_timer(tracing_subscriber::fmt::time::uptime()) // Show uptime instead of timestamp
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
        tracing::debug!(
            "Repository preservation mode enabled - repositories will not be cleaned up"
        );
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
                    // Pretty print each repository item with additional fields
                    for (i, repo) in result.items.iter().enumerate() {
                        let description = repo.description.as_deref().unwrap_or("<no description>");

                        // Basic format as shown in the documentation
                        println!(
                            "{}. {}/{} - {} stars",
                            i + 1,
                            repo.owner.id.as_ref().unwrap_or(&"N/A".to_string()),
                            repo.name,
                            repo.stargazers_count.unwrap_or(0)
                        );
                        println!("   Description: {}", description);
                        println!(
                            "   URL: {}",
                            repo.html_url.as_ref().map(|u| u.as_str()).unwrap_or("N/A")
                        );

                        // Additional fields requested
                        println!("   ID: {}", repo.id);

                        if let Some(lang) = &repo.language {
                            println!("   Language: {}", lang);
                        } else {
                            println!("   Language: <none>");
                        }

                        println!("   Archived: {}", repo.archived.unwrap_or(false));
                        println!(
                            "   Created: {}",
                            repo.created_at.as_ref().unwrap_or(&"N/A".to_string())
                        );
                        if let Some(score) = repo.score {
                            println!("   Score: {:.2}", score);
                        }
                        println!(
                            "   Last Push: {}",
                            repo.pushed_at.as_ref().unwrap_or(&"N/A".to_string())
                        );

                        // Add empty line after each repository
                        println!();
                    }

                    // If no results found
                    if result.items.is_empty() {
                        println!("No repositories matched your search criteria.");
                    }

                    Ok(())
                }
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
                }
            }
        }
        Commands::IssueSearch {
            query,
            sort_by,
            order,
            per_page,
            page,
            repository,
            labels,
            state,
            creator,
            mentioned,
            assignee,
            milestone,
            issue_type,
        } => {
            use gitcodes_mcp::gitcodes::repository_manager::providers::GitProvider;

            // Default to GitHub as provider
            let git_provider = GitProvider::Github;

            // Convert from clap enum types to the types used by repository_manager
            let sort_option = sort_by.map(|s| s.into());
            let order_option = order.map(|o| o.into());

            // Create issue search parameters
            let search_params = gitcodes_mcp::gitcodes::repository_manager::IssueSearchParams {
                query,
                sort_by: sort_option,
                order: order_option,
                per_page,
                page,
                repository,
                labels,
                state,
                creator,
                mentioned,
                assignee,
                milestone,
                issue_type,
            };

            // Execute the search using the repository manager
            match manager.search_issues(git_provider, search_params).await {
                Ok(result) => {
                    // Pretty print each issue item
                    for (i, issue) in result.items.iter().enumerate() {
                        let body_preview = issue
                            .body
                            .as_deref()
                            .unwrap_or("<no description>")
                            .chars()
                            .take(100)
                            .collect::<String>();
                        let body_display = if issue.body.as_deref().unwrap_or("").len() > 100 {
                            format!("{}...", body_preview)
                        } else {
                            body_preview
                        };

                        // Basic format
                        println!(
                            "{}. #{} - {} [{}]",
                            i + 1,
                            issue.number,
                            issue.title,
                            issue.state
                        );
                        println!("   Repository: {}", issue.repository.name);
                        println!("   Author: {}", issue.user.login);
                        println!("   Description: {}", body_display);
                        println!("   URL: {}", issue.html_url);

                        // Additional fields
                        println!("   Comments: {}", issue.comments);
                        println!("   Created: {}", issue.created_at);
                        println!("   Updated: {}", issue.updated_at);

                        if let Some(closed_at) = &issue.closed_at {
                            println!("   Closed: {}", closed_at);
                        }

                        // Show labels if any
                        if !issue.labels.is_empty() {
                            let label_names: Vec<String> = issue
                                .labels
                                .iter()
                                .map(|label| label.name.clone())
                                .collect();
                            println!("   Labels: {}", label_names.join(", "));
                        }

                        // Show assignees if any
                        if !issue.assignees.is_empty() {
                            let assignee_names: Vec<String> = issue
                                .assignees
                                .iter()
                                .map(|assignee| assignee.login.clone())
                                .collect();
                            println!("   Assignees: {}", assignee_names.join(", "));
                        }

                        // Show milestone if any
                        // if let Some(milestone) = &issue.milestone {
                        //     println!("   Milestone: {} [{}]", milestone.title, milestone.state);
                        // }

                        if let Some(score) = issue.score {
                            println!("   Score: {:.2}", score);
                        }

                        // Add empty line after each issue
                        println!();
                    }

                    // Summary information
                    println!(
                        "Found {} issues (total: {})",
                        result.items.len(),
                        result.total_count
                    );
                    if result.incomplete_results {
                        println!(
                            "Note: Results may be incomplete due to timeout or other factors."
                        );
                    }

                    // If no results found
                    if result.items.is_empty() {
                        println!("No issues matched your search criteria.");
                    }

                    Ok(())
                }
                Err(err) => {
                    tracing::error!("Issue search failed: {}", err);

                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Issue search failed: {}", err);
                    let suggestion = if error_msg.contains("API rate limit") {
                        "\nSuggestion: You may have exceeded GitHub's API rate limits. Try the following:\n  - Use a GitHub token with '-t' option\n  - Wait a few minutes and try again\n  - Reduce the number of requests"
                    } else if error_msg.contains("authentication") || error_msg.contains("401") {
                        "\nSuggestion: Authentication failed. Try the following:\n  - Check that your GitHub token is valid and has not expired\n  - Ensure the token has appropriate permissions\n  - Regenerate your GitHub token if necessary"
                    } else {
                        "\nSuggestion: Check your network connection, GitHub credentials, and search query syntax."
                    };

                    anyhow::bail!("{}{}", error_msg, suggestion)
                }
            }
        }
        Commands::Grep {
            repository_location,
            ref_name,
            pattern,
            case_sensitive,
            file_extensions,
            include_globs,
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
                    return Err(anyhow::anyhow!(
                        "Failed to process repository location: {}",
                        e
                    ));
                }
            };

            // Use the services module to perform the grep operation
            use gitcodes_mcp::services;

            let grep_params = services::GrepParams {
                repository_location_str: processed_location.clone(),
                pattern,
                ref_name: ref_name.clone(),
                case_sensitive: case_sensitive.unwrap_or(false),
                file_extensions: file_extensions.clone(),
                include_globs: include_globs.clone(),
                exclude_dirs: exclude_dirs.clone(),
                before_context,
                after_context,
                skip: None,                        // No skip (pagination)
                take: None,                        // No take (pagination)
                match_content_omit_num: Some(150), // Default to 150 characters
            };

            match services::perform_grep_in_repository(manager, grep_params).await {
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
                    // Log the error but don't exit immediately
                    tracing::error!("Failed to search code: {}", e);

                    // Check if this is a terminal error or if our fallback mechanism might have worked
                    if e.contains("All URL formats failed")
                        || !e.contains("Failed to clone repository")
                    {
                        // This is a terminal error after all fallbacks, provide user-friendly message
                        let error_msg = format!("Failed to search code: {}", e);
                        let suggestion = if error_msg.contains("clone")
                            || error_msg.contains("Failed to parse repository location")
                        {
                            if error_msg.contains("invalid remote git url")
                                || error_msg.contains("must be absolute")
                            {
                                "\nSuggestion: The repository location format appears to be invalid. Try the following:\n  - For GitHub: Use 'git@github.com:user/repo.git' (SSH format works most reliably)\n  - Alternative formats: 'https://github.com/user/repo' (without .git suffix) or 'github:user/repo'\n  - For local repositories: Use an absolute path to an existing local git repository\n  - Relative paths are not supported for security reasons"
                            } else if error_msg.contains("HTTPS")
                                || error_msg.contains("I/O error")
                                || error_msg.contains("io error")
                                || error_msg.contains("An IO error occurred")
                            {
                                "\nSuggestion: There was an issue cloning via HTTPS. Try the following:\n  - For GitHub URLs, try the format: 'https://github.com/user/repo' (without .git suffix)\n  - As an alternative, use SSH URL format: 'git@github.com:user/repo.git'\n  - Check your network connection or firewall settings\n  - Check if you have a proxy configured and it's properly set in your environment variables\n  - Run 'git config --global --get http.proxy' to verify your Git proxy settings\n  - Verify the repository exists and is accessible\n  - For private repositories, provide a GitHub token with '-t'"
                            } else if error_msg.contains("authentication")
                                || error_msg.contains("credential")
                            {
                                "\nSuggestion: Authentication failed when cloning the repository. Try the following:\n  - For private repositories, provide a GitHub token with '-t'\n  - Check if your token has the correct permissions\n  - Try using SSH URL format with properly configured SSH keys: 'git@github.com:user/repo.git'\n  - Verify your SSH keys are set up correctly (run 'ssh -T git@github.com')"
                            } else if error_msg.contains("checkout")
                                || error_msg.contains("reference")
                                || error_msg.contains("ref")
                            {
                                "\nSuggestion: Failed to checkout the specified branch or reference. Try the following:\n  - Verify the branch/tag name exists in the repository\n  - Try without specifying a branch (it will use the default branch)\n  - Check the repository's available branches on GitHub"
                            } else {
                                "\nSuggestion: Failed to clone repository. Try the following:\n  - Use SSH URL format: 'git@github.com:user/repo.git' (most reliable)\n  - Check your network connection\n  - Check for any proxy requirements in your network\n  - Try running: ./git_diagnostic <repository_url> to diagnose issues\n  - Verify the repository exists and is accessible\n  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)\n  - Check if the ref/branch/tag exists in the repository"
                            }
                        } else if error_msg.contains("pattern") {
                            "\nSuggestion: There was an issue with your search pattern. Try the following:\n  - Simplify your search pattern\n  - Escape special regex characters if you're not using regex\n  - Use the --case-sensitive option if needed"
                        } else {
                            "\nSuggestion: Check your repository location and search parameters."
                        };

                        anyhow::bail!("{}{}", error_msg, suggestion)
                    } else {
                        // The repository manager might be handling this with fallbacks
                        tracing::info!("Repository manager is handling the error with fallbacks");
                        // Continue without error to let repository manager fallbacks work
                        Ok(())
                    }
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
                    return Err(anyhow::anyhow!(
                        "Failed to process repository location: {}",
                        e
                    ));
                }
            };

            // Use the services module to access the file contents
            use gitcodes_mcp::services;

            // Clone file_path to retain ownership of the original for later use
            let file_path_clone = file_path.clone();

            let show_params = services::ShowFileParams {
                repository_location_str: processed_location.clone(),
                file_path: file_path_clone,
                ref_name: ref_name.clone(),
                max_size,
                line_from,
                line_to,
                without_line_numbers: Some(without_line_numbers),
            };

            match services::show_file_contents(manager, show_params).await {
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
                            println!(
                                "Size: {} characters, {} lines",
                                metadata.char_count, metadata.line_count
                            );

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
                        }
                        FileContents::Binary { message, metadata } => {
                            println!("File: {}", file_path);
                            println!("Type: Binary file");
                            println!("Size: {} bytes", metadata.size_bytes);
                            if let Some(mime) = &metadata.mime_type {
                                println!("MIME type: {}", mime);
                            }
                            println!("---");
                            println!("{}", message);
                        }
                        FileContents::Image { message, metadata } => {
                            println!("File: {}", file_path);
                            println!("Type: Image file");
                            // media_type is directly a String, not an Option
                            println!("Media type: {}", metadata.media_type);
                            println!("Size: {} bytes", metadata.size_bytes);
                            println!("---");
                            println!("{}", message);
                            println!("Note: Use a graphical viewer to see this image.");
                        }
                    }

                    // Clean up the repository when finished (unless preserve flag is set)
                    cleanup_repository(local_repo, cli.preserve_repos);

                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to show file contents: {}", e);

                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Failed to show file contents: {}", e);
                    let suggestion = if error_msg.contains("clone")
                        || error_msg.contains("Failed to parse repository location")
                    {
                        if error_msg.contains("invalid remote git url")
                            || error_msg.contains("must be absolute")
                        {
                            "\nSuggestion: The repository location format appears to be invalid. Try the following:\n  - For GitHub: Use 'git@github.com:user/repo.git' (SSH format works most reliably)\n  - Alternative formats: 'https://github.com/user/repo' or 'github:user/repo'\n  - For local repositories: Use an absolute path to an existing local git repository\n  - Relative paths are not supported for security reasons"
                        } else {
                            "\nSuggestion: Could not access repository. Try the following:\n  - Check your network connection\n  - Verify the repository exists and is accessible\n  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)"
                        }
                    } else if error_msg.contains("not found")
                        || error_msg.contains("File not found")
                    {
                        "\nSuggestion: File not found in repository. Try the following:\n  - Check the file path spelling\n  - Check if the file exists in the specified branch or tag\n  - Use the correct path relative to the repository root"
                    } else if error_msg.contains("too large") {
                        "\nSuggestion: File is too large to display. Try the following:\n  - Use the --max-size option to increase the maximum file size\n  - Use the --from-line and --to-line options to view a subset of the file"
                    } else if error_msg.contains("..") || error_msg.contains("Invalid path") {
                        "\nSuggestion: Invalid file path. Ensure that:\n  - The path does not contain '..' (parent directory references)\n  - The path is relative to the repository root\n  - The path does not attempt to access outside the repository"
                    } else {
                        "\nSuggestion: Check your repository location and file path."
                    };

                    anyhow::bail!("{}{}", error_msg, suggestion)
                }
            }
        }
        Commands::ListRefs {
            repository_location,
        } => {
            tracing::debug!("Listing references for repository: {}", repository_location);

            // Process the repository location (convert relative paths to absolute)
            let processed_location = match process_repository_location(&repository_location) {
                Ok(location) => location,
                Err(e) => {
                    tracing::error!("Failed to process repository location: {}", e);
                    return Err(anyhow::anyhow!(
                        "Failed to process repository location: {}",
                        e
                    ));
                }
            };

            // Get the refs directly from the repository manager

            match manager.list_repository_refs(&processed_location).await {
                Err(e) => {
                    tracing::error!("Failed to list repository references: {}", e);

                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Failed to list repository references: {}", e);
                    let suggestion = if error_msg.contains("clone")
                        || error_msg.contains("Failed to parse repository location")
                    {
                        if error_msg.contains("invalid remote git url")
                            || error_msg.contains("must be absolute")
                        {
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
                Ok((refs, local_repo_opt)) => {
                    // Print the header for the table
                    println!("{:<40} {:<10} {:<40}", "Reference", "Type", "Commit ID");
                    println!("{:-<40} {:-<10} {:-<40}", "", "", "");

                    // Display branches
                    if !refs.branches.is_empty() {
                        for branch in &refs.branches {
                            println!(
                                "{:<40} {:<10} {:<40}",
                                format!("branch: {}", branch.name),
                                "branch",
                                branch.commit_id
                            );
                        }
                    }

                    // Display tags
                    if !refs.tags.is_empty() {
                        for tag in &refs.tags {
                            println!(
                                "{:<40} {:<10} {:<40}",
                                format!("tag: {}", tag.name),
                                "tag",
                                tag.commit_id
                            );
                        }
                    }

                    // Display count of references found
                    let total_refs = refs.branches.len() + refs.tags.len();
                    println!(
                        "
Total: {} references found ({} branches, {} tags)",
                        total_refs,
                        refs.branches.len(),
                        refs.tags.len()
                    );

                    // Clean up the repository if it exists (unless preserve flag is set)
                    cleanup_repository_opt(local_repo_opt, cli.preserve_repos);
                }
            }

            Ok(())
        }
        Commands::Tree {
            repository_location,
            ref_name,
            case_sensitive,
            respect_gitignore,
            depth,
            strip_path_prefix,
            search_relative_path,
        } => {
            tracing::debug!("Getting tree for repository: {}", repository_location);

            // Process the repository location (convert relative paths to absolute)
            let processed_location = match process_repository_location(&repository_location) {
                Ok(location) => location,
                Err(e) => {
                    tracing::error!("Failed to process repository location: {}", e);
                    return Err(anyhow::anyhow!(
                        "Failed to process repository location: {}",
                        e
                    ));
                }
            };

            // Call the tree service function
            let tree_params = gitcodes_mcp::services::TreeServiceParams {
                repository_location_str: processed_location.clone(),
                ref_name: ref_name.clone(),
                case_sensitive: Some(case_sensitive),
                respect_gitignore: Some(respect_gitignore),
                depth,
                strip_path_prefix: Some(strip_path_prefix),
                search_relative_path: search_relative_path.map(std::path::PathBuf::from),
            };

            match gitcodes_mcp::services::get_repository_tree(manager, tree_params).await {
                Ok((tree, local_repo)) => {
                    // Display the tree structure
                    if tree.is_empty() {
                        println!("No directories found in repository");
                    } else {
                        println!("Directory tree for repository: {}", repository_location);
                        if let Some(ref_name) = &ref_name {
                            println!("Reference: {}", ref_name);
                        }
                        println!("---");

                        // Print each directory tree
                        for dir_tree in &tree {
                            if dir_tree.dir.is_empty() {
                                println!("Repository root:");
                            } else {
                                println!("{}:", dir_tree.dir);
                            }

                            // Sort entries by type (directories first) and then by name
                            let mut sorted_entries = dir_tree.entries.clone();
                            sorted_entries.sort_by(|a, b| {
                                use gitcodes_mcp::gitcodes::local_repository::TreeEntry;
                                match (a, b) {
                                    (
                                        TreeEntry::Directory(a_name),
                                        TreeEntry::Directory(b_name),
                                    ) => a_name.cmp(b_name),
                                    (TreeEntry::File(a_name), TreeEntry::File(b_name)) => {
                                        a_name.cmp(b_name)
                                    }
                                    (TreeEntry::Directory(_), TreeEntry::File(_)) => {
                                        std::cmp::Ordering::Less
                                    }
                                    (TreeEntry::File(_), TreeEntry::Directory(_)) => {
                                        std::cmp::Ordering::Greater
                                    }
                                }
                            });

                            // Display entries with tree-like formatting
                            for (i, entry) in sorted_entries.iter().enumerate() {
                                let is_last = i == sorted_entries.len() - 1;
                                let prefix = if is_last { "└── " } else { "├── " };

                                match entry {
                                    gitcodes_mcp::gitcodes::local_repository::TreeEntry::Directory(name) => {
                                        println!("{}📁 {}/", prefix, name);
                                    }
                                    gitcodes_mcp::gitcodes::local_repository::TreeEntry::File(name) => {
                                        println!("{}📄 {}", prefix, name);
                                    }
                                }
                            }
                            println!(); // Empty line between directories
                        }

                        // Print summary
                        let total_files: usize = tree
                            .iter()
                            .map(|dir| {
                                dir.entries
                                    .iter()
                                    .filter(|entry| {
                                        matches!(
                                            entry,
                                            gitcodes_mcp::gitcodes::local_repository::TreeEntry::File(_)
                                        )
                                    })
                                    .count()
                            })
                            .sum();

                        let total_dirs: usize = tree
                            .iter()
                            .map(|dir| {
                                dir.entries
                                    .iter()
                                    .filter(|entry| {
                                        matches!(
                                            entry,
                                            gitcodes_mcp::gitcodes::local_repository::TreeEntry::Directory(_)
                                        )
                                    })
                                    .count()
                            })
                            .sum();

                        println!("Total: {} files, {} directories", total_files, total_dirs);
                    }

                    // Clean up the repository when finished (unless preserve flag is set)
                    cleanup_repository(local_repo, cli.preserve_repos);

                    Ok(())
                }
                Err(e) => {
                    tracing::error!("Failed to get repository tree: {}", e);

                    // Provide more user-friendly error message with suggestions
                    let error_msg = format!("Failed to get repository tree: {}", e);
                    let suggestion = if error_msg.contains("clone")
                        || error_msg.contains("Failed to parse repository location")
                    {
                        if error_msg.contains("invalid remote git url")
                            || error_msg.contains("must be absolute")
                        {
                            "\nSuggestion: The repository location format appears to be invalid. Try the following:\n  - For GitHub: Use 'git@github.com:user/repo.git' (SSH format works most reliably)\n  - Alternative formats: 'https://github.com/user/repo' or 'github:user/repo'\n  - For local repositories: Use an absolute path to an existing local git repository\n  - Relative paths are not supported for security reasons"
                        } else {
                            "\nSuggestion: Could not access repository. Try the following:\n  - Check your network connection\n  - Verify the repository exists and is accessible\n  - Ensure you have proper permissions (provide a GitHub token with '-t' if it's a private repository)"
                        }
                    } else if error_msg.contains("not found") {
                        "\nSuggestion: Repository not found. Try the following:\n  - Check the repository location spelling\n  - Check if the repository exists and is accessible\n  - Use the correct repository format"
                    } else {
                        "\nSuggestion: Check your repository location and access permissions."
                    };

                    anyhow::bail!("{}{}", error_msg, suggestion)
                }
            }
        }
    }
}
