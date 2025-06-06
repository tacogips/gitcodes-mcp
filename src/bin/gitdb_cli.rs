use anyhow::Result;
use clap::{Parser, Subcommand};
use gitdb::ids::{IssueId, IssueNumber, PullRequestId, PullRequestNumber};
use gitdb::services::SyncService;
use gitdb::storage::GitDatabase;
use gitdb::types::{ItemType, RepositoryName};
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "gitdb")]
#[command(about = "GitHub Database Tool - Sync and search GitHub repositories locally")]
struct Cli {
    /// GitHub personal access token
    #[arg(long)]
    github_token: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Register a repository for syncing
    Register {
        /// Repository URL (e.g., https://github.com/owner/repo or owner/repo)
        url: String,
    },

    /// List registered repositories
    List,

    /// Remove a repository
    Unregister {
        /// Repository full name (owner/repo)
        repo: String,
    },

    /// Sync repositories
    Sync {
        /// Sync specific repository (owner/repo)
        #[arg(long)]
        repo: Option<String>,

        /// Force full sync (ignore timestamps)
        #[arg(long)]
        full: bool,

        /// Sync all data from beginning (replace existing)
        #[arg(value_name = "all")]
        sync_all: Option<String>,
    },

    /// Search for issues and pull requests
    Search {
        /// Search query
        query: String,

        /// Search in specific repository (owner/repo)
        #[arg(long)]
        repo: Option<String>,

        /// Filter by state (open/closed)
        #[arg(long)]
        state: Option<String>,

        /// Filter by label
        #[arg(long)]
        label: Option<String>,

        /// Maximum number of results
        #[arg(long, default_value = "30")]
        limit: usize,
    },

    /// Find related issues and pull requests
    Related {
        /// Repository and issue/PR number (e.g., owner/repo#123)
        #[arg(value_name = "REFERENCE", conflicts_with_all = ["repo", "issue", "pr"])]
        reference: Option<String>,

        /// Repository (owner/repo)
        #[arg(long, requires = "issue_or_pr")]
        repo: Option<String>,

        /// Issue number
        #[arg(long, group = "issue_or_pr", conflicts_with = "pr")]
        issue: Option<u64>,

        /// Pull request number
        #[arg(long, group = "issue_or_pr", conflicts_with = "issue")]
        pr: Option<u64>,

        /// Maximum number of results
        #[arg(long, default_value = "10")]
        limit: usize,

        /// Only show link relationships (no semantic search)
        #[arg(long, conflicts_with = "semantic_only")]
        links_only: bool,

        /// Only show semantic relationships
        #[arg(long, conflicts_with = "links_only")]
        semantic_only: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("gitdb=info".parse()?))
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();

    // Get GitHub token from CLI or environment
    let github_token = cli
        .github_token
        .or_else(|| std::env::var("GITDB_GITHUB_TOKEN").ok());

    // Initialize database
    let db = Arc::new(GitDatabase::new().await?);

    match cli.command {
        Commands::Register { url } => {
            let sync_service = SyncService::new(db.clone(), github_token.clone())?;

            // First sync the repository to get its info
            println!("Registering repository: {}", url);
            let result = sync_service.sync_repository(&url, false).await?;

            if !result.errors.is_empty() {
                eprintln!("Sync completed with errors:");
                for error in &result.errors {
                    eprintln!("  - {}", error);
                }
            }

            println!("Repository registered and synced successfully!");
            println!("Issues synced: {}", result.issues_synced);
            println!("Pull requests synced: {}", result.pull_requests_synced);
        }

        Commands::List => {
            let repos = db.list_repositories().await?;

            if repos.is_empty() {
                println!("No repositories registered");
            } else {
                println!("Registered repositories:");
                for repo in repos {
                    println!(
                        "  - {} (⭐ {}, 🍴 {})",
                        repo.full_name, repo.stars, repo.forks
                    );
                    if let Some(desc) = &repo.description {
                        println!("    {}", desc);
                    }
                }
            }
        }

        Commands::Unregister { repo: _ } => {
            // For now, we don't have a direct unregister method
            // This would need to be implemented in the database layer
            eprintln!("Unregister command not yet implemented");
        }

        Commands::Sync {
            repo,
            full,
            sync_all,
        } => {
            let sync_service = SyncService::new(db.clone(), github_token)?;

            let full_sync = full || sync_all.is_some();

            if let Some(repo_name) = repo {
                println!("Syncing repository: {}", repo_name);
                let result = sync_service.sync_repository(&repo_name, full_sync).await?;

                if !result.errors.is_empty() {
                    eprintln!("Sync completed with errors:");
                    for error in &result.errors {
                        eprintln!("  - {}", error);
                    }
                }

                println!("Sync completed!");
                println!("Issues synced: {}", result.issues_synced);
                println!("Pull requests synced: {}", result.pull_requests_synced);
            } else {
                // Sync all repositories
                let repos = db.list_repositories().await?;

                if repos.is_empty() {
                    println!("No repositories to sync");
                    return Ok(());
                }

                println!("Syncing {} repositories...", repos.len());

                for repo in repos {
                    println!("\nSyncing {}", repo.full_name);
                    let result = sync_service
                        .sync_repository(&repo.full_name, full_sync)
                        .await?;

                    if !result.errors.is_empty() {
                        eprintln!("  Errors:");
                        for error in &result.errors {
                            eprintln!("    - {}", error);
                        }
                    }

                    println!(
                        "  Issues: {}, PRs: {}",
                        result.issues_synced, result.pull_requests_synced
                    );
                }

                println!("\nAll repositories synced!");
            }
        }

        Commands::Search {
            query,
            repo,
            state,
            label: _,
            limit,
        } => {
            // First, determine repository ID if filtering by repo
            let repo_id = if let Some(repo_name) = repo {
                let repo_name = match RepositoryName::new(&repo_name) {
                    Ok(name) => name,
                    Err(e) => {
                        eprintln!("Invalid repository name: {}", e);
                        return Ok(());
                    }
                };
                match db.get_repository_by_full_name(&repo_name).await? {
                    Some(repository) => Some(repository.id),
                    None => {
                        eprintln!("Repository {} not found", repo_name);
                        return Ok(());
                    }
                }
            } else {
                None
            };

            // Search functionality has been removed from GitDatabase
            eprintln!("Error: Search functionality is not currently available.");
            eprintln!("The search backend has been removed from the database module.");
            return Ok(());
        }

        Commands::Related {
            reference,
            repo,
            issue,
            pr,
            limit,
            links_only,
            semantic_only,
        } => {
            // Parse the reference or use repo/issue/pr
            let (repo_name, item_type, item_number) = if let Some(ref_str) = reference {
                // Parse owner/repo#123 format
                let parts: Vec<&str> = ref_str.split('#').collect();
                if parts.len() != 2 {
                    eprintln!("Invalid reference format. Use owner/repo#123");
                    return Ok(());
                }

                let number = parts[1].parse::<u64>().map_err(|_| {
                    eprintln!("Invalid issue/PR number");
                    anyhow::anyhow!("Invalid number")
                })?;

                (parts[0].to_string(), None, number)
            } else if let Some(repo_name) = repo {
                if let Some(issue_num) = issue {
                    (repo_name, Some(ItemType::Issue), issue_num)
                } else if let Some(pr_num) = pr {
                    (repo_name, Some(ItemType::PullRequest), pr_num)
                } else {
                    eprintln!("Must specify either --issue or --pr");
                    return Ok(());
                }
            } else {
                eprintln!(
                    "Must specify either a reference (owner/repo#123) or --repo with --issue/--pr"
                );
                return Ok(());
            };

            // Get repository
            let repo_name_typed = match RepositoryName::new(&repo_name) {
                Ok(name) => name,
                Err(e) => {
                    eprintln!("Invalid repository name: {}", e);
                    return Ok(());
                }
            };
            let repository = match db.get_repository_by_full_name(&repo_name_typed).await? {
                Some(repo) => repo,
                None => {
                    eprintln!("Repository {} not found", repo_name);
                    return Ok(());
                }
            };

            // Find the specific issue or PR and determine its actual type
            let (source_title, source_body, actual_item_type) =
                if item_type.is_none() || item_type == Some(ItemType::Issue) {
                    // Try to find as issue first
                    let issues = db.list_issues_by_repository(&repository.id).await?;
                    if let Some(issue) = issues
                        .iter()
                        .find(|i| i.number == IssueNumber::new(item_number as i64))
                    {
                        (
                            issue.title.clone(),
                            issue.body.clone().unwrap_or_default(),
                            ItemType::Issue,
                        )
                    } else if item_type.is_none() {
                        // Try as PR
                        let prs = db
                            .list_pull_requests_by_repository(&repository.id)
                            .await?;
                        if let Some(pr) = prs
                            .iter()
                            .find(|p| p.number == PullRequestNumber::new(item_number as i64))
                        {
                            (
                                pr.title.clone(),
                                pr.body.clone().unwrap_or_default(),
                                ItemType::PullRequest,
                            )
                        } else {
                            eprintln!("Issue or PR #{} not found in {}", item_number, repo_name);
                            return Ok(());
                        }
                    } else {
                        eprintln!("Issue #{} not found in {}", item_number, repo_name);
                        return Ok(());
                    }
                } else {
                    // Find as PR
                    let prs = db
                        .list_pull_requests_by_repository(&repository.id)
                        .await?;
                    if let Some(pr) = prs
                        .iter()
                        .find(|p| p.number == PullRequestNumber::new(item_number as i64))
                    {
                        (
                            pr.title.clone(),
                            pr.body.clone().unwrap_or_default(),
                            ItemType::PullRequest,
                        )
                    } else {
                        eprintln!("Pull request #{} not found in {}", item_number, repo_name);
                        return Ok(());
                    }
                };

            println!(
                "Finding items related to: {} #{} - {}",
                repo_name, item_number, source_title
            );
            println!();

            let mut all_results = Vec::new();

            // 1. Find items referenced by this issue/PR (outgoing)
            if !semantic_only {
                let all_refs = db.list_cross_references_from(&repository.id).await?;
                let outgoing_refs: Vec<_> = all_refs.into_iter()
                    .filter(|xref| xref.source_type == actual_item_type && xref.source_id == item_number as i64)
                    .collect();

                if !outgoing_refs.is_empty() {
                    println!("=== Outgoing References (this item references) ===");
                    for xref in &outgoing_refs {
                        println!("  → {} ({})", xref.link_text, xref.target_type);
                        all_results
                            .push(format!("[OUT] {} ({})", xref.link_text, xref.target_type));
                    }
                    println!();
                }
            }

            // 2. Find items that reference this issue/PR (incoming)
            if !semantic_only {
                let all_refs = db.list_cross_references_to(&repository.id).await?;
                let incoming_refs: Vec<_> = all_refs.into_iter()
                    .filter(|xref| xref.target_type == actual_item_type && xref.target_number == item_number as i64)
                    .collect();

                if !incoming_refs.is_empty() {
                    println!("=== Incoming References (referenced by) ===");
                    for xref in &incoming_refs {
                        // Find the source item to display
                        let source_desc = if xref.source_type == ItemType::Issue {
                            let issues = db
                                .list_issues_by_repository(&xref.source_repository_id)
                                .await?;
                            issues
                                .iter()
                                .find(|i| i.id == IssueId::new(xref.source_id))
                                .map(|i| format!("Issue #{}: {}", i.number, i.title))
                                .unwrap_or_else(|| format!("Issue #{}", xref.source_id))
                        } else {
                            let prs = db
                                .list_pull_requests_by_repository(&xref.source_repository_id)
                                .await?;
                            prs.iter()
                                .find(|p| p.id == PullRequestId::new(xref.source_id))
                                .map(|p| format!("PR #{}: {}", p.number, p.title))
                                .unwrap_or_else(|| format!("PR #{}", xref.source_id))
                        };

                        println!("  ← {}", source_desc);
                        all_results.push(format!("[IN] {}", source_desc));
                    }
                    println!();
                }
            }

            // 3. Find semantically similar items
            if !links_only {
                // Search functionality has been removed from GitDatabase
                // Skip semantic similarity search
            }

            if all_results.is_empty() {
                println!("No related items found");
            } else {
                println!("\nTotal related items found: {}", all_results.len());
            }
        }
    }

    Ok(())
}
