use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::git::GitHubClient;
use crate::ids::{RepositoryId, SyncStatusId};
use crate::storage::{CrossReference, GitDatabase, SyncStatus};
use crate::types::{ItemType, ResourceType, SyncStatusType, RepositoryName};

pub struct SyncService {
    db: Arc<GitDatabase>,
    github: Arc<GitHubClient>,
}

impl SyncService {
    /// Creates a new SyncService instance with the specified database and GitHub token.
    ///
    /// # Arguments
    ///
    /// * `db` - Arc reference to the GitDatabase for storing synchronized data
    /// * `github_token` - Optional GitHub personal access token for API authentication
    ///
    /// # Returns
    ///
    /// Returns a Result containing the new SyncService instance or an error if initialization fails.
    ///
    /// # Errors
    ///
    /// Returns an error if the GitHubClient initialization fails.
    pub fn new(db: Arc<GitDatabase>, github_token: Option<String>) -> Result<Self> {
        let github = GitHubClient::new(github_token)?;

        Ok(Self {
            db,
            github: Arc::new(github),
        })
    }

    /// Synchronizes a GitHub repository's issues and pull requests to the local database.
    ///
    /// # Arguments
    ///
    /// * `repo_url` - The GitHub repository URL (supports various formats like https://github.com/owner/repo)
    /// * `full_sync` - If true, performs a full synchronization ignoring last sync timestamps
    ///
    /// # Returns
    ///
    /// Returns a SyncResult containing the number of synced items and any errors encountered.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The repository URL parsing fails
    /// - The GitHub API calls fail
    /// - Database operations fail
    pub async fn sync_repository(&self, repo_url: &str, full_sync: bool) -> Result<SyncResult> {
        let (owner, name) = parse_repo_url(repo_url)?;
        info!("Starting sync for {}/{}", owner, name);

        // Fetch repository info
        let repo = self.github.get_repository(&owner, &name).await?;
        self.db.save_repository(&repo).await?;

        let mut result = SyncResult::default();

        // Get last sync status
        let last_issue_sync = if !full_sync {
            self.db
                .get_sync_status(&repo.id, ResourceType::Issues)
                .await?
        } else {
            None
        };

        let last_pr_sync = if !full_sync {
            self.db
                .get_sync_status(&repo.id, ResourceType::PullRequests)
                .await?
        } else {
            None
        };

        // Sync issues
        let issue_since = last_issue_sync.as_ref().map(|s| s.last_synced_at);
        match self.sync_issues(&owner, &name, repo.id, issue_since).await {
            Ok(count) => {
                result.issues_synced = count;
                self.update_sync_status(
                    repo.id,
                    ResourceType::Issues,
                    SyncStatusType::Success,
                    None,
                    count,
                )
                .await?;
            }
            Err(e) => {
                error!("Failed to sync issues: {}", e);
                result.errors.push(format!("Issues sync failed: {}", e));
                self.update_sync_status(
                    repo.id,
                    ResourceType::Issues,
                    SyncStatusType::Failed,
                    Some(&e.to_string()),
                    0,
                )
                .await?;
            }
        }

        // Sync pull requests
        let pr_since = last_pr_sync.as_ref().map(|s| s.last_synced_at);
        match self
            .sync_pull_requests(&owner, &name, repo.id, pr_since)
            .await
        {
            Ok(count) => {
                result.pull_requests_synced = count;
                self.update_sync_status(
                    repo.id,
                    ResourceType::PullRequests,
                    SyncStatusType::Success,
                    None,
                    count,
                )
                .await?;
            }
            Err(e) => {
                error!("Failed to sync pull requests: {}", e);
                result
                    .errors
                    .push(format!("Pull requests sync failed: {}", e));
                self.update_sync_status(
                    repo.id,
                    ResourceType::PullRequests,
                    SyncStatusType::Failed,
                    Some(&e.to_string()),
                    0,
                )
                .await?;
            }
        }

        info!(
            "Sync completed for {}/{}: {} issues, {} PRs",
            owner, name, result.issues_synced, result.pull_requests_synced
        );

        Ok(result)
    }

    async fn sync_issues(
        &self,
        owner: &str,
        name: &str,
        repository_id: RepositoryId,
        since: Option<DateTime<Utc>>,
    ) -> Result<usize> {
        debug!("Fetching issues for {}/{} since {:?}", owner, name, since);

        let issues = self
            .github
            .get_issues(owner, name, repository_id, since)
            .await?;
        let count = issues.len();

        for issue in &issues {
            // Save issue
            self.db.save_issue(issue).await?;

            // Sync comments
            let comments = self
                .github
                .get_issue_comments(owner, name, issue.number.value() as u64, issue.id)
                .await?;
            for comment in &comments {
                self.db.save_issue_comment(comment).await?;
            }

            // Parse cross-references in issue body and comments
            if let Some(body) = &issue.body {
                self.parse_and_store_references(
                    body,
                    repository_id,
                    ItemType::Issue,
                    issue.id.value(),
                )
                .await?;
            }

            for comment in &comments {
                self.parse_and_store_references(
                    &comment.body,
                    repository_id,
                    ItemType::Issue,
                    issue.id.value(),
                )
                .await?;
            }
        }

        Ok(count)
    }

    async fn sync_pull_requests(
        &self,
        owner: &str,
        name: &str,
        repository_id: RepositoryId,
        since: Option<DateTime<Utc>>,
    ) -> Result<usize> {
        debug!(
            "Fetching pull requests for {}/{} since {:?}",
            owner, name, since
        );

        let prs = self
            .github
            .get_pull_requests(owner, name, repository_id, since)
            .await?;
        let count = prs.len();

        for pr in &prs {
            // Save pull request
            self.db.save_pull_request(pr).await?;

            // Sync comments
            let comments = self
                .github
                .get_pull_request_comments(owner, name, pr.number.value() as u64, pr.id)
                .await?;
            for comment in &comments {
                self.db.save_pull_request_comment(comment).await?;
            }

            // Parse cross-references in PR body and comments
            if let Some(body) = &pr.body {
                self.parse_and_store_references(
                    body,
                    repository_id,
                    ItemType::PullRequest,
                    pr.id.value(),
                )
                .await?;
            }

            for comment in &comments {
                self.parse_and_store_references(
                    &comment.body,
                    repository_id,
                    ItemType::PullRequest,
                    pr.id.value(),
                )
                .await?;
            }
        }

        Ok(count)
    }

    async fn parse_and_store_references(
        &self,
        text: &str,
        source_repo_id: RepositoryId,
        source_type: ItemType,
        source_id: i64,
    ) -> Result<()> {
        // Regex patterns for GitHub references
        let url_pattern = Regex::new(r"https://github\.com/([^/]+)/([^/]+)/(issues|pull)/(\d+)")
            .context("Failed to compile URL regex")?;
        let short_pattern = Regex::new(r"([^/\s]+)/([^/#\s]+)#(\d+)")
            .context("Failed to compile short reference regex")?;

        let mut found_refs = HashSet::new();

        // Find URL references
        for cap in url_pattern.captures_iter(text) {
            let owner = &cap[1];
            let repo = &cap[2];
            let ref_type = if &cap[3] == "issues" {
                ItemType::Issue
            } else {
                ItemType::PullRequest
            };
            let number: i64 = cap[4].parse()?;

            found_refs.insert((owner.to_string(), repo.to_string(), ref_type, number));
        }

        // Find short references
        for cap in short_pattern.captures_iter(text) {
            let owner = &cap[1];
            let repo = &cap[2];
            let number: i64 = cap[3].parse()?;

            // We don't know if it's an issue or PR from short form, so we'll check both
            found_refs.insert((owner.to_string(), repo.to_string(), ItemType::Issue, number));
            found_refs.insert((
                owner.to_string(),
                repo.to_string(),
                ItemType::PullRequest,
                number,
            ));
        }

        // Store cross-references for registered repositories
        for (owner, repo, ref_type, number) in found_refs {
            let full_name = format!("{}/{}", owner, repo);

            // Check if target repository is registered
            let repo_name = match RepositoryName::new(&full_name) {
                Ok(name) => name,
                Err(_) => continue, // Skip invalid repository names
            };
            if let Some(target_repo) = self.db.get_repository_by_full_name(&repo_name).await? {
                let cross_ref = CrossReference {
                    source_type,
                    source_id,
                    source_repository_id: source_repo_id,
                    target_type: ref_type,
                    target_repository_id: target_repo.id,
                    target_number: number,
                    link_text: format!("{}#{}", full_name, number),
                    created_at: Utc::now(),
                };

                self.db.save_cross_reference(&cross_ref).await?;
            }
        }

        Ok(())
    }

    async fn update_sync_status(
        &self,
        repository_id: RepositoryId,
        resource_type: ResourceType,
        status: SyncStatusType,
        error_message: Option<&str>,
        items_synced: usize,
    ) -> Result<()> {
        let sync_status = SyncStatus {
            id: SyncStatusId::new(0), // Will be assigned by database
            repository_id,
            resource_type,
            last_synced_at: Utc::now(),
            status,
            error_message: error_message.map(|s| s.to_string()),
            items_synced: items_synced as i64,
        };

        self.db.save_sync_status(&sync_status).await?;

        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SyncResult {
    pub issues_synced: usize,
    pub pull_requests_synced: usize,
    pub errors: Vec<String>,
}

/// Parses a GitHub repository URL to extract the owner and repository name.
///
/// # Arguments
///
/// * `url` - The repository URL to parse
///
/// # Returns
///
/// Returns a tuple of (owner, repository_name) on success.
///
/// # Supported Formats
///
/// - `https://github.com/owner/repo`
/// - `https://github.com/owner/repo.git`
/// - `git@github.com:owner/repo.git`
/// - `owner/repo` (simple format)
///
/// # Errors
///
/// Returns an error if the URL doesn't match any supported format.
pub fn parse_repo_url(url: &str) -> Result<(String, String)> {
    // Support various GitHub URL formats
    let patterns = [
        Regex::new(r"^https://github\.com/([^/]+)/([^/]+?)(?:\.git)?/?$")?,
        Regex::new(r"^git@github\.com:([^/]+)/([^/]+?)(?:\.git)?$")?,
        Regex::new(r"^([^/]+)/([^/]+)$")?, // Simple owner/repo format
    ];

    for pattern in &patterns {
        if let Some(caps) = pattern.captures(url) {
            return Ok((caps[1].to_string(), caps[2].to_string()));
        }
    }

    anyhow::bail!("Invalid repository URL format: {}", url)
}
