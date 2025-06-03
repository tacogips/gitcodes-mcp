use crate::ids::{IssueId, IssueNumber, PullRequestId, PullRequestNumber};
use crate::services::SyncService;
use crate::storage::GitDatabase;
use crate::types::{ItemType, ResourceType};
use rmcp::handler::server::tool::Parameters;
use rmcp::{Error as McpError, ServerHandler, model::*, tool};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod error;
pub mod responses;

use error::ToolError;

/// Wrapper for GitHub code tools exposed through the MCP protocol
#[derive(Clone)]
pub struct GitDbTools {
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
    db: Arc<Mutex<Option<Arc<GitDatabase>>>>,
}

impl GitDbTools {
    /// Creates a new GitDbTools instance with optional authentication and custom repository cache dir
    pub fn new(github_token: Option<String>, repository_cache_dir: Option<PathBuf>) -> Self {
        Self {
            github_token,
            repository_cache_dir,
            db: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or initialize the database connection
    async fn get_db(&self) -> Result<Arc<GitDatabase>, ToolError> {
        let mut db_opt = self.db.lock().await;
        if let Some(db) = &*db_opt {
            Ok(db.clone())
        } else {
            let db =
                Arc::new(GitDatabase::new().await.map_err(|e| {
                    ToolError::Other(format!("Failed to initialize database: {}", e))
                })?);
            *db_opt = Some(db.clone());
            Ok(db)
        }
    }
}

/// Parameters for registering a repository
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RegisterRepositoryParams {
    #[schemars(
        description = "Repository URL or identifier (required). Supports formats: 'https://github.com/owner/repo', 'git@github.com:owner/repo.git', or 'owner/repo'. The repository will be registered and synced to download issues and pull requests for local search. This parameter is required and must be provided."
    )]
    pub url: String,
}

/// Parameters for syncing repositories
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncParams {
    #[schemars(
        description = "Specific repository to sync (optional). Format: 'owner/repo'. When omitted, syncs all registered repositories. Use this to update a single repository's issues and pull requests."
    )]
    pub repo: Option<String>,

    #[schemars(
        description = "Force full sync ignoring timestamps (optional, default false). When true, fetches all issues and pull requests from the beginning, replacing existing data. When false or omitted, only fetches updates since last sync."
    )]
    pub full: Option<bool>,
}

/// Parameters for searching issues and pull requests
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchParams {
    #[schemars(
        description = "Search query (required). Searches across issue/PR titles, bodies, and comments using full-text search. Supports natural language queries and keywords. This parameter is required and must be provided."
    )]
    pub query: String,

    #[schemars(
        description = "Search in specific repository (optional). Format: 'owner/repo'. When omitted, searches across all registered repositories. Use to limit search scope to a single repository."
    )]
    pub repo: Option<String>,

    #[schemars(
        description = "Filter by state (optional). Valid values: 'open', 'closed'. When omitted, returns items in any state. Use to find only active or resolved items."
    )]
    pub state: Option<String>,

    #[schemars(
        description = "Filter by label (optional). Exact label name to filter by. When specified, only returns issues/PRs with this label. Case-sensitive."
    )]
    pub label: Option<String>,

    #[schemars(
        description = "Maximum number of results (optional, default 30, max 100). Controls how many search results to return. Higher values provide more results but may be slower."
    )]
    pub limit: Option<usize>,
}

/// Parameters for finding related items
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelatedItemsParams {
    #[schemars(
        description = "Repository name (required). Format: 'owner/repo'. Specifies which repository contains the issue or pull request. This parameter is required and must be provided."
    )]
    pub repo: String,

    #[schemars(
        description = "Issue or pull request number (required). The number of the issue or PR to find related items for. Must be a positive integer. This parameter is required and must be provided."
    )]
    pub number: u64,

    #[schemars(
        description = "Item type (optional). Valid values: 'issue' or 'pr'. When omitted, automatically detects whether the number refers to an issue or pull request. Use to explicitly specify the type."
    )]
    pub item_type: Option<String>,

    #[schemars(
        description = "Maximum number of results (optional, default 10). Controls how many related items to return. Includes both linked items and semantically similar items."
    )]
    pub limit: Option<usize>,

    #[schemars(
        description = "Only show link relationships (optional, default false). When true, only returns items that have explicit cross-references (URLs or #123 references). Excludes semantic similarity results."
    )]
    pub links_only: Option<bool>,

    #[schemars(
        description = "Only show semantic relationships (optional, default false). When true, only returns items found through semantic similarity. Excludes explicit cross-references."
    )]
    pub semantic_only: Option<bool>,
}

/// Response for listing repositories
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListRepositoriesResponse {
    pub repositories: Vec<RepositoryInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryInfo {
    pub full_name: String,
    pub description: Option<String>,
    pub stars: i64,
    pub forks: i64,
    pub language: Option<String>,
    pub issues_count: Option<i64>,
    pub pull_requests_count: Option<i64>,
    pub last_synced: Option<String>,
}

/// Response for sync operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SyncResponse {
    pub repositories_synced: usize,
    pub total_issues_synced: i64,
    pub total_pull_requests_synced: i64,
    pub errors: Vec<String>,
}

/// Response for search operation
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SearchResult {
    pub repository: String,
    pub item_type: String,
    pub number: i64,
    pub title: String,
    pub body: Option<String>,
    pub state: String,
    pub author: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub url: String,
}

/// Response for related items
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RelatedItemsResponse {
    pub source_item: ItemInfo,
    pub outgoing_references: Vec<CrossReferenceInfo>,
    pub incoming_references: Vec<CrossReferenceInfo>,
    pub semantically_similar: Vec<ItemInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ItemInfo {
    pub repository: String,
    pub item_type: String,
    pub number: i64,
    pub title: String,
    pub state: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CrossReferenceInfo {
    pub repository: String,
    pub item_type: String,
    pub number: i64,
    pub title: String,
    pub link_text: String,
    pub direction: String,
}

#[tool(tool_box)]
impl GitDbTools {
    #[tool(
        description = "Register a GitHub repository for syncing and searching. This tool registers a repository and performs an initial sync to download all issues and pull requests. The data is stored locally for fast searching. Example: `{\"name\": \"register_repository\", \"arguments\": {\"url\": \"https://github.com/rust-lang/rust\"}}`. Alternative format: `{\"name\": \"register_repository\", \"arguments\": {\"url\": \"tokio-rs/tokio\"}}`"
    )]
    async fn register_repository(
        &self,
        Parameters(params): Parameters<RegisterRepositoryParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = match self.get_db().await {
            Ok(db) => db,
            Err(e) => return error_result(e.to_string()),
        };

        let sync_service = match SyncService::new(db.clone(), self.github_token.clone()) {
            Ok(service) => service,
            Err(e) => return error_result(format!("Failed to create sync service: {}", e)),
        };

        match sync_service.sync_repository(&params.url, false).await {
            Ok(result) => {
                let response = serde_json::json!({
                    "status": "success",
                    "repository": params.url,
                    "issues_synced": result.issues_synced,
                    "pull_requests_synced": result.pull_requests_synced,
                    "errors": result.errors
                });
                success_result(serde_json::to_string(&response).unwrap())
            }
            Err(e) => error_result(format!("Failed to register repository: {}", e)),
        }
    }

    #[tool(
        description = "List all registered repositories with their sync status. Shows repository details including stars, forks, and last sync time. No parameters required. Example: `{\"name\": \"list_repositories\", \"arguments\": {}}`"
    )]
    async fn list_repositories(&self) -> Result<CallToolResult, McpError> {
        let db = match self.get_db().await {
            Ok(db) => db,
            Err(e) => return error_result(e.to_string()),
        };

        match db.list_repositories().await {
            Ok(repos) => {
                let mut repo_infos = Vec::new();
                for repo in repos {
                    // Get counts for issues and PRs
                    let issues_count = db
                        .get_issues_by_repository(repo.id, None)
                        .await
                        .map(|issues| issues.len() as i64)
                        .ok();
                    let prs_count = db
                        .get_pull_requests_by_repository(repo.id, None)
                        .await
                        .map(|prs| prs.len() as i64)
                        .ok();

                    // Get last sync time
                    let last_synced = db
                        .get_last_sync_status(repo.id, ResourceType::Issues)
                        .await
                        .ok()
                        .and_then(|status| status)
                        .map(|s| s.last_synced_at.to_rfc3339());

                    repo_infos.push(RepositoryInfo {
                        full_name: repo.full_name,
                        description: repo.description,
                        stars: repo.stars,
                        forks: repo.forks,
                        language: repo.language,
                        issues_count,
                        pull_requests_count: prs_count,
                        last_synced,
                    });
                }

                let response = ListRepositoriesResponse {
                    repositories: repo_infos,
                };
                success_result(serde_json::to_string(&response).unwrap())
            }
            Err(e) => error_result(format!("Failed to list repositories: {}", e)),
        }
    }

    #[tool(
        description = "Sync repository data from GitHub. Updates issues, pull requests, and comments for registered repositories. By default performs incremental sync (only new updates). Example for all repos: `{\"name\": \"sync_repositories\", \"arguments\": {}}`. For specific repo: `{\"name\": \"sync_repositories\", \"arguments\": {\"repo\": \"rust-lang/rust\"}}`. For full sync: `{\"name\": \"sync_repositories\", \"arguments\": {\"repo\": \"tokio-rs/tokio\", \"full\": true}}`"
    )]
    async fn sync_repositories(
        &self,
        Parameters(params): Parameters<SyncParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = match self.get_db().await {
            Ok(db) => db,
            Err(e) => return error_result(e.to_string()),
        };

        let sync_service = match SyncService::new(db.clone(), self.github_token.clone()) {
            Ok(service) => service,
            Err(e) => return error_result(format!("Failed to create sync service: {}", e)),
        };

        let full_sync = params.full.unwrap_or(false);

        let mut response = SyncResponse {
            repositories_synced: 0,
            total_issues_synced: 0,
            total_pull_requests_synced: 0,
            errors: Vec::new(),
        };

        if let Some(repo_name) = params.repo {
            // Sync specific repository
            match sync_service.sync_repository(&repo_name, full_sync).await {
                Ok(result) => {
                    response.repositories_synced = 1;
                    response.total_issues_synced = result.issues_synced as i64;
                    response.total_pull_requests_synced = result.pull_requests_synced as i64;
                    response.errors = result.errors;
                }
                Err(e) => {
                    response
                        .errors
                        .push(format!("Failed to sync {}: {}", repo_name, e));
                }
            }
        } else {
            // Sync all repositories
            match db.list_repositories().await {
                Ok(repos) => {
                    for repo in repos {
                        match sync_service
                            .sync_repository(&repo.full_name, full_sync)
                            .await
                        {
                            Ok(result) => {
                                response.repositories_synced += 1;
                                response.total_issues_synced += result.issues_synced as i64;
                                response.total_pull_requests_synced +=
                                    result.pull_requests_synced as i64;
                                if !result.errors.is_empty() {
                                    response.errors.extend(result.errors);
                                }
                            }
                            Err(e) => {
                                response
                                    .errors
                                    .push(format!("Failed to sync {}: {}", repo.full_name, e));
                            }
                        }
                    }
                }
                Err(e) => {
                    return error_result(format!("Failed to list repositories: {}", e));
                }
            }
        }

        success_result(serde_json::to_string(&response).unwrap())
    }

    #[tool(
        description = "Search for issues and pull requests in synced repositories. Performs full-text search across titles, bodies, and comments. Returns both issues and PRs matching the query. Example: `{\"name\": \"search_items\", \"arguments\": {\"query\": \"memory leak\"}}`. With filters: `{\"name\": \"search_items\", \"arguments\": {\"query\": \"authentication\", \"repo\": \"tokio-rs/tokio\", \"state\": \"open\", \"limit\": 50}}`"
    )]
    async fn search_items(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = match self.get_db().await {
            Ok(db) => db,
            Err(e) => return error_result(e.to_string()),
        };

        // Get repository ID if filtering by repo
        let repo_id = if let Some(repo_name) = &params.repo {
            match db.get_repository_by_full_name(repo_name).await {
                Ok(Some(repo)) => Some(repo.id),
                Ok(None) => return error_result(format!("Repository {} not found", repo_name)),
                Err(e) => return error_result(format!("Failed to get repository: {}", e)),
            }
        } else {
            None
        };

        let limit = params.limit.unwrap_or(30).min(100);

        match db.search(&params.query, repo_id, limit).await {
            Ok(results) => {
                let mut search_results = Vec::new();

                for result in results {
                    // Get repository name
                    let repo_name = if let Some(repo_name) = &params.repo {
                        repo_name.clone()
                    } else {
                        // Look up repository name
                        match db.get_repository_by_id(result.repository_id).await {
                            Ok(Some(repo)) => repo.full_name,
                            _ => continue,
                        }
                    };

                    // Filter by state if specified
                    if let Some(filter_state) = &params.state {
                        let item_state = if result.result_type == "issue" {
                            // Get issue to check state
                            match db
                                .get_issues_by_repository(result.repository_id, None)
                                .await
                            {
                                Ok(issues) => issues
                                    .iter()
                                    .find(|i| i.number.value() == result.id)
                                    .map(|i| i.state.to_string()),
                                Err(_) => None,
                            }
                        } else {
                            // Get PR to check state
                            match db
                                .get_pull_requests_by_repository(result.repository_id, None)
                                .await
                            {
                                Ok(prs) => prs
                                    .iter()
                                    .find(|p| p.number.value() == result.id)
                                    .map(|p| p.state.to_string()),
                                Err(_) => None,
                            }
                        };

                        if let Some(state) = item_state {
                            if state.to_lowercase() != filter_state.to_lowercase() {
                                continue;
                            }
                        }
                    }

                    // Get full item details
                    let (author, labels, created_at, updated_at, state) = if result.result_type
                        == "issue"
                    {
                        match db
                            .get_issues_by_repository(result.repository_id, None)
                            .await
                        {
                            Ok(issues) => {
                                if let Some(issue) =
                                    issues.iter().find(|i| i.number.value() == result.id)
                                {
                                    (
                                        issue.author.clone(),
                                        issue.labels.clone(),
                                        issue.created_at.to_rfc3339(),
                                        issue.updated_at.to_rfc3339(),
                                        issue.state.to_string(),
                                    )
                                } else {
                                    continue;
                                }
                            }
                            Err(_) => continue,
                        }
                    } else {
                        match db
                            .get_pull_requests_by_repository(result.repository_id, None)
                            .await
                        {
                            Ok(prs) => {
                                if let Some(pr) = prs.iter().find(|p| p.number.value() == result.id)
                                {
                                    (
                                        pr.author.clone(),
                                        pr.labels.clone(),
                                        pr.created_at.to_rfc3339(),
                                        pr.updated_at.to_rfc3339(),
                                        pr.state.to_string(),
                                    )
                                } else {
                                    continue;
                                }
                            }
                            Err(_) => continue,
                        }
                    };

                    // Filter by label if specified
                    if let Some(filter_label) = &params.label {
                        if !labels.contains(filter_label) {
                            continue;
                        }
                    }

                    let url = format!(
                        "https://github.com/{}/{}/{}",
                        repo_name,
                        if result.result_type == "issue" {
                            "issues"
                        } else {
                            "pull"
                        },
                        result.id
                    );

                    search_results.push(SearchResult {
                        repository: repo_name,
                        item_type: result.result_type,
                        number: result.id,
                        title: result.title,
                        body: result.body,
                        state,
                        author,
                        labels,
                        created_at,
                        updated_at,
                        url,
                    });
                }

                let response = SearchResponse {
                    total_count: search_results.len(),
                    results: search_results,
                };
                success_result(serde_json::to_string(&response).unwrap())
            }
            Err(e) => error_result(format!("Search failed: {}", e)),
        }
    }

    #[tool(
        description = "Find issues and pull requests related to a specific item. Returns items that reference or are referenced by the specified issue/PR, as well as semantically similar items. Example: `{\"name\": \"find_related_items\", \"arguments\": {\"repo\": \"rust-lang/rust\", \"number\": 12345}}`. With options: `{\"name\": \"find_related_items\", \"arguments\": {\"repo\": \"tokio-rs/tokio\", \"number\": 4567, \"item_type\": \"pr\", \"links_only\": true}}`"
    )]
    async fn find_related_items(
        &self,
        Parameters(params): Parameters<RelatedItemsParams>,
    ) -> Result<CallToolResult, McpError> {
        let db = match self.get_db().await {
            Ok(db) => db,
            Err(e) => return error_result(e.to_string()),
        };

        // Get repository
        let repository = match db.get_repository_by_full_name(&params.repo).await {
            Ok(Some(repo)) => repo,
            Ok(None) => return error_result(format!("Repository {} not found", params.repo)),
            Err(e) => return error_result(format!("Failed to get repository: {}", e)),
        };

        let limit = params.limit.unwrap_or(10);
        let links_only = params.links_only.unwrap_or(false);
        let semantic_only = params.semantic_only.unwrap_or(false);

        // Determine item type
        let (actual_item_type, source_title, source_state) = if let Some(item_type_str) =
            &params.item_type
        {
            let item_type = match item_type_str.as_str() {
                "issue" => ItemType::Issue,
                "pr" | "pull_request" => ItemType::PullRequest,
                _ => return error_result("Invalid item_type. Use 'issue' or 'pr'"),
            };

            // Get item details
            match item_type {
                ItemType::Issue => match db.get_issues_by_repository(repository.id, None).await {
                    Ok(issues) => {
                        if let Some(issue) = issues
                            .iter()
                            .find(|i| i.number == IssueNumber::new(params.number as i64))
                        {
                            (
                                ItemType::Issue,
                                issue.title.clone(),
                                issue.state.to_string(),
                            )
                        } else {
                            return error_result(format!(
                                "Issue #{} not found in {}",
                                params.number, params.repo
                            ));
                        }
                    }
                    Err(e) => return error_result(format!("Failed to get issues: {}", e)),
                },
                ItemType::PullRequest => {
                    match db
                        .get_pull_requests_by_repository(repository.id, None)
                        .await
                    {
                        Ok(prs) => {
                            if let Some(pr) = prs
                                .iter()
                                .find(|p| p.number == PullRequestNumber::new(params.number as i64))
                            {
                                (
                                    ItemType::PullRequest,
                                    pr.title.clone(),
                                    pr.state.to_string(),
                                )
                            } else {
                                return error_result(format!(
                                    "Pull request #{} not found in {}",
                                    params.number, params.repo
                                ));
                            }
                        }
                        Err(e) => {
                            return error_result(format!("Failed to get pull requests: {}", e));
                        }
                    }
                }
            }
        } else {
            // Auto-detect type
            let issues = match db.get_issues_by_repository(repository.id, None).await {
                Ok(issues) => issues,
                Err(e) => return error_result(format!("Failed to get issues: {}", e)),
            };

            if let Some(issue) = issues
                .iter()
                .find(|i| i.number == IssueNumber::new(params.number as i64))
            {
                (
                    ItemType::Issue,
                    issue.title.clone(),
                    issue.state.to_string(),
                )
            } else {
                // Try as PR
                let prs = match db
                    .get_pull_requests_by_repository(repository.id, None)
                    .await
                {
                    Ok(prs) => prs,
                    Err(e) => return error_result(format!("Failed to get pull requests: {}", e)),
                };

                if let Some(pr) = prs
                    .iter()
                    .find(|p| p.number == PullRequestNumber::new(params.number as i64))
                {
                    (
                        ItemType::PullRequest,
                        pr.title.clone(),
                        pr.state.to_string(),
                    )
                } else {
                    return error_result(format!(
                        "Item #{} not found in {}",
                        params.number, params.repo
                    ));
                }
            }
        };

        let source_url = format!(
            "https://github.com/{}/{}/{}",
            params.repo,
            if actual_item_type == ItemType::Issue {
                "issues"
            } else {
                "pull"
            },
            params.number
        );

        let source_item = ItemInfo {
            repository: params.repo.clone(),
            item_type: actual_item_type.to_string(),
            number: params.number as i64,
            title: source_title.clone(),
            state: source_state,
            url: source_url,
        };

        let mut outgoing_references = Vec::new();
        let mut incoming_references = Vec::new();
        let mut semantically_similar = Vec::new();

        // Get cross-references unless semantic_only
        if !semantic_only {
            // Outgoing references
            match db.get_cross_references_by_source(
                repository.id,
                actual_item_type,
                params.number as i64,
            ) {
                Ok(refs) => {
                    for xref in refs {
                        // Get target repository name
                        if let Ok(Some(target_repo)) =
                            db.get_repository_by_id(xref.target_repository_id).await
                        {
                            let _target_url = format!(
                                "https://github.com/{}/{}/{}",
                                target_repo.full_name,
                                if xref.target_type == ItemType::Issue {
                                    "issues"
                                } else {
                                    "pull"
                                },
                                xref.target_number
                            );

                            // Get target title
                            let target_title = if xref.target_type == ItemType::Issue {
                                db.get_issues_by_repository(xref.target_repository_id, None)
                                    .await
                                    .ok()
                                    .and_then(|issues| {
                                        issues
                                            .iter()
                                            .find(|i| i.number.value() == xref.target_number)
                                            .map(|i| i.title.clone())
                                    })
                                    .unwrap_or_else(|| format!("Issue #{}", xref.target_number))
                            } else {
                                db.get_pull_requests_by_repository(xref.target_repository_id, None)
                                    .await
                                    .ok()
                                    .and_then(|prs| {
                                        prs.iter()
                                            .find(|p| p.number.value() == xref.target_number)
                                            .map(|p| p.title.clone())
                                    })
                                    .unwrap_or_else(|| format!("PR #{}", xref.target_number))
                            };

                            outgoing_references.push(CrossReferenceInfo {
                                repository: target_repo.full_name,
                                item_type: xref.target_type.to_string(),
                                number: xref.target_number,
                                title: target_title,
                                link_text: xref.link_text,
                                direction: "outgoing".to_string(),
                            });
                        }
                    }
                }
                Err(_) => {}
            }

            // Incoming references
            match db.get_cross_references_by_target(
                repository.id,
                actual_item_type,
                params.number as i64,
            ) {
                Ok(refs) => {
                    for xref in refs {
                        // Get source repository name
                        if let Ok(Some(source_repo)) =
                            db.get_repository_by_id(xref.source_repository_id).await
                        {
                            // Get source title
                            let source_title = if xref.source_type == ItemType::Issue {
                                db.get_issues_by_repository(xref.source_repository_id, None)
                                    .await
                                    .ok()
                                    .and_then(|issues| {
                                        issues
                                            .iter()
                                            .find(|i| i.id == IssueId::new(xref.source_id))
                                            .map(|i| i.title.clone())
                                    })
                                    .unwrap_or_else(|| format!("Issue"))
                            } else {
                                db.get_pull_requests_by_repository(xref.source_repository_id, None)
                                    .await
                                    .ok()
                                    .and_then(|prs| {
                                        prs.iter()
                                            .find(|p| p.id == PullRequestId::new(xref.source_id))
                                            .map(|p| p.title.clone())
                                    })
                                    .unwrap_or_else(|| format!("PR"))
                            };

                            // Get source number
                            let source_number = if xref.source_type == ItemType::Issue {
                                db.get_issues_by_repository(xref.source_repository_id, None)
                                    .await
                                    .ok()
                                    .and_then(|issues| {
                                        issues
                                            .iter()
                                            .find(|i| i.id == IssueId::new(xref.source_id))
                                            .map(|i| i.number.value())
                                    })
                                    .unwrap_or(0)
                            } else {
                                db.get_pull_requests_by_repository(xref.source_repository_id, None)
                                    .await
                                    .ok()
                                    .and_then(|prs| {
                                        prs.iter()
                                            .find(|p| p.id == PullRequestId::new(xref.source_id))
                                            .map(|p| p.number.value())
                                    })
                                    .unwrap_or(0)
                            };

                            incoming_references.push(CrossReferenceInfo {
                                repository: source_repo.full_name,
                                item_type: xref.source_type.to_string(),
                                number: source_number,
                                title: source_title,
                                link_text: xref.link_text,
                                direction: "incoming".to_string(),
                            });
                        }
                    }
                }
                Err(_) => {}
            }
        }

        // Get semantically similar items unless links_only
        if !links_only {
            match db
                .search(&source_title, Some(repository.id), limit * 2)
                .await
            {
                Ok(results) => {
                    for result in results {
                        // Skip the source item itself
                        if result.result_type == actual_item_type.to_string()
                            && result.id == params.number as i64
                        {
                            continue;
                        }

                        // Get full details
                        let (title, state) = if result.result_type == "issue" {
                            match db
                                .get_issues_by_repository(result.repository_id, None)
                                .await
                            {
                                Ok(issues) => {
                                    if let Some(issue) =
                                        issues.iter().find(|i| i.number.value() == result.id)
                                    {
                                        (issue.title.clone(), issue.state.to_string())
                                    } else {
                                        continue;
                                    }
                                }
                                Err(_) => continue,
                            }
                        } else {
                            match db
                                .get_pull_requests_by_repository(result.repository_id, None)
                                .await
                            {
                                Ok(prs) => {
                                    if let Some(pr) =
                                        prs.iter().find(|p| p.number.value() == result.id)
                                    {
                                        (pr.title.clone(), pr.state.to_string())
                                    } else {
                                        continue;
                                    }
                                }
                                Err(_) => continue,
                            }
                        };

                        let url = format!(
                            "https://github.com/{}/{}/{}",
                            params.repo,
                            if result.result_type == "issue" {
                                "issues"
                            } else {
                                "pull"
                            },
                            result.id
                        );

                        semantically_similar.push(ItemInfo {
                            repository: params.repo.clone(),
                            item_type: result.result_type,
                            number: result.id,
                            title,
                            state,
                            url,
                        });

                        if semantically_similar.len() >= limit {
                            break;
                        }
                    }
                }
                Err(_) => {}
            }
        }

        let response = RelatedItemsResponse {
            source_item,
            outgoing_references,
            incoming_references,
            semantically_similar,
        };

        success_result(serde_json::to_string(&response).unwrap())
    }
}

impl ServerHandler for GitDbTools {
    /// Provides information about this MCP server
    fn get_info(&self) -> ServerInfo {
        let auth_status = match &self.github_token {
            Some(_) => "Authenticated with GitHub token",
            None => "Not authenticated (rate limits apply)",
        };

        let instructions = format!(
            r#"GitDB MCP Server - {}

## Overview
GitDB is a tool for syncing and searching GitHub repository data locally. It downloads issues, pull requests, and comments from GitHub repositories and stores them in a local database for fast searching.

## Available Tools

### 1. register_repository
Register a GitHub repository for syncing. This downloads all issues and PRs for local search.

Example:
```json
{{"name": "register_repository", "arguments": {{"url": "rust-lang/rust"}}}}
```

### 2. list_repositories
List all registered repositories with their sync status.

Example:
```json
{{"name": "list_repositories", "arguments": {{}}}}
```

### 3. sync_repositories
Update repository data from GitHub. By default performs incremental sync.

Examples:
```json
// Sync all repositories
{{"name": "sync_repositories", "arguments": {{}}}}

// Sync specific repository
{{"name": "sync_repositories", "arguments": {{"repo": "rust-lang/rust"}}}}

// Force full sync
{{"name": "sync_repositories", "arguments": {{"repo": "tokio-rs/tokio", "full": true}}}}
```

### 4. search_items
Search for issues and pull requests across synced repositories.

Examples:
```json
// Basic search
{{"name": "search_items", "arguments": {{"query": "memory leak"}}}}

// Search with filters
{{"name": "search_items", "arguments": {{
    "query": "authentication",
    "repo": "tokio-rs/tokio",
    "state": "open",
    "limit": 50
}}}}
```

### 5. find_related_items
Find items related to a specific issue or pull request through cross-references and semantic similarity.

Examples:
```json
// Find all related items
{{"name": "find_related_items", "arguments": {{"repo": "rust-lang/rust", "number": 12345}}}}

// Only show cross-references
{{"name": "find_related_items", "arguments": {{
    "repo": "tokio-rs/tokio",
    "number": 4567,
    "links_only": true
}}}}
```

## Common Workflows

1. **Initial Setup**:
   - Register repositories you want to search
   - Initial sync happens automatically during registration

2. **Searching**:
   - Use search_items to find issues/PRs by keywords
   - Use find_related_items to explore connections between items

3. **Keeping Data Fresh**:
   - Run sync_repositories periodically to get latest updates
   - Use full sync if you suspect data inconsistencies
"#,
            auth_status
        );

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(instructions),
        }
    }
}

/// Helper method to create a CallToolResult for successful responses.
///
/// # Arguments
///
/// * `json` - The JSON string to include in the successful response
///
/// # Returns
///
/// Returns a Result containing a successful CallToolResult with the JSON content.
fn success_result(json: String) -> Result<CallToolResult, McpError> {
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

/// Helper method to create a CallToolResult for error responses.
///
/// # Arguments
///
/// * `message` - The error message to include in the response
///
/// # Returns
///
/// Returns a Result containing an error CallToolResult with the error message.
fn error_result(message: impl Into<String>) -> Result<CallToolResult, McpError> {
    let error_message = message.into();
    Ok(CallToolResult::error(vec![Content::text(error_message)]))
}
