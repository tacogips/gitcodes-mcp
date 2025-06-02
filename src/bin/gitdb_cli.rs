use anyhow::Result;
use clap::{Parser, Subcommand};
use gitdb::services::SyncService;
use gitdb::storage::GitDatabase;
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
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("gitdb=info".parse()?)
        )
        .with_writer(std::io::stderr)
        .init();
    
    let cli = Cli::parse();
    
    // Get GitHub token from CLI or environment
    let github_token = cli.github_token.or_else(|| std::env::var("GITDB_GITHUB_TOKEN").ok());
    
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
                    println!("  - {} (⭐ {}, 🍴 {})", repo.full_name, repo.stars, repo.forks);
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
        
        Commands::Sync { repo, full, sync_all } => {
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
                    let result = sync_service.sync_repository(&repo.full_name, full_sync).await?;
                    
                    if !result.errors.is_empty() {
                        eprintln!("  Errors:");
                        for error in &result.errors {
                            eprintln!("    - {}", error);
                        }
                    }
                    
                    println!("  Issues: {}, PRs: {}", result.issues_synced, result.pull_requests_synced);
                }
                
                println!("\nAll repositories synced!");
            }
        }
        
        Commands::Search { query, repo, state, label: _, limit } => {
            // First, determine repository ID if filtering by repo
            let repo_id = if let Some(repo_name) = repo {
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
            
            // Perform search
            let results = db.search(&query, repo_id, limit).await?;
            
            if results.is_empty() {
                println!("No results found");
            } else {
                println!("Found {} results:", results.len());
                
                for result in results {
                    // Apply additional filters if needed
                    if let Some(_filter_state) = &state {
                        // We'd need to load the full issue/PR to filter by state
                        // For now, skip this filter
                    }
                    
                    println!("\n[{}] {}", result.result_type, result.title);
                    if let Some(body) = &result.body {
                        // Show first 200 chars of body
                        let preview = if body.len() > 200 {
                            format!("{}...", &body[..200])
                        } else {
                            body.clone()
                        };
                        println!("{}", preview);
                    }
                }
            }
        }
    }
    
    Ok(())
}