use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;
use tracing::{debug, error, info};

use crate::git::GitHubClient;
use crate::storage::{CrossReference, GitDatabase, SyncStatus};

pub struct SyncService {
    db: Arc<GitDatabase>,
    github: Arc<GitHubClient>,
}

impl SyncService {
    pub fn new(db: Arc<GitDatabase>, github_token: Option<String>) -> Result<Self> {
        let github = GitHubClient::new(github_token)?;
        
        Ok(Self {
            db,
            github: Arc::new(github),
        })
    }
    
    pub async fn sync_repository(&self, repo_url: &str, full_sync: bool) -> Result<SyncResult> {
        let (owner, name) = parse_repo_url(repo_url)?;
        info!("Starting sync for {}/{}", owner, name);
        
        // Fetch repository info
        let repo = self.github.get_repository(&owner, &name).await?;
        self.db.upsert_repository(&repo).await?;
        
        let mut result = SyncResult::default();
        
        // Get last sync status
        let last_issue_sync = if !full_sync {
            self.db.get_last_sync_status(repo.id, "issues").await?
        } else {
            None
        };
        
        let last_pr_sync = if !full_sync {
            self.db.get_last_sync_status(repo.id, "pull_requests").await?
        } else {
            None
        };
        
        // Sync issues
        let issue_since = last_issue_sync.as_ref().map(|s| s.last_synced_at);
        match self.sync_issues(&owner, &name, repo.id, issue_since).await {
            Ok(count) => {
                result.issues_synced = count;
                self.update_sync_status(repo.id, "issues", "success", None, count).await?;
            }
            Err(e) => {
                error!("Failed to sync issues: {}", e);
                result.errors.push(format!("Issues sync failed: {}", e));
                self.update_sync_status(repo.id, "issues", "failed", Some(&e.to_string()), 0).await?;
            }
        }
        
        // Sync pull requests
        let pr_since = last_pr_sync.as_ref().map(|s| s.last_synced_at);
        match self.sync_pull_requests(&owner, &name, repo.id, pr_since).await {
            Ok(count) => {
                result.pull_requests_synced = count;
                self.update_sync_status(repo.id, "pull_requests", "success", None, count).await?;
            }
            Err(e) => {
                error!("Failed to sync pull requests: {}", e);
                result.errors.push(format!("Pull requests sync failed: {}", e));
                self.update_sync_status(repo.id, "pull_requests", "failed", Some(&e.to_string()), 0).await?;
            }
        }
        
        info!("Sync completed for {}/{}: {} issues, {} PRs", 
            owner, name, result.issues_synced, result.pull_requests_synced);
        
        Ok(result)
    }
    
    async fn sync_issues(&self, owner: &str, name: &str, repository_id: i64, since: Option<DateTime<Utc>>) -> Result<usize> {
        debug!("Fetching issues for {}/{} since {:?}", owner, name, since);
        
        let issues = self.github.get_issues(owner, name, repository_id, since).await?;
        let count = issues.len();
        
        for issue in &issues {
            // Save issue
            self.db.upsert_issue(issue).await?;
            
            // Sync comments
            let comments = self.github.get_issue_comments(owner, name, issue.number as u64, issue.id).await?;
            for comment in &comments {
                self.db.upsert_issue_comment(comment).await?;
            }
            
            // Parse cross-references in issue body and comments
            if let Some(body) = &issue.body {
                self.parse_and_store_references(body, repository_id, "issue", issue.id).await?;
            }
            
            for comment in &comments {
                self.parse_and_store_references(&comment.body, repository_id, "issue", issue.id).await?;
            }
        }
        
        Ok(count)
    }
    
    async fn sync_pull_requests(&self, owner: &str, name: &str, repository_id: i64, since: Option<DateTime<Utc>>) -> Result<usize> {
        debug!("Fetching pull requests for {}/{} since {:?}", owner, name, since);
        
        let prs = self.github.get_pull_requests(owner, name, repository_id, since).await?;
        let count = prs.len();
        
        for pr in &prs {
            // Save pull request
            self.db.upsert_pull_request(pr).await?;
            
            // Sync comments
            let comments = self.github.get_pull_request_comments(owner, name, pr.number as u64, pr.id).await?;
            for comment in &comments {
                self.db.upsert_pull_request_comment(comment).await?;
            }
            
            // Parse cross-references in PR body and comments
            if let Some(body) = &pr.body {
                self.parse_and_store_references(body, repository_id, "pull_request", pr.id).await?;
            }
            
            for comment in &comments {
                self.parse_and_store_references(&comment.body, repository_id, "pull_request", pr.id).await?;
            }
        }
        
        Ok(count)
    }
    
    async fn parse_and_store_references(&self, text: &str, source_repo_id: i64, source_type: &str, source_id: i64) -> Result<()> {
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
            let ref_type = if &cap[3] == "issues" { "issue" } else { "pull_request" };
            let number: i64 = cap[4].parse()?;
            
            found_refs.insert((owner.to_string(), repo.to_string(), ref_type.to_string(), number));
        }
        
        // Find short references
        for cap in short_pattern.captures_iter(text) {
            let owner = &cap[1];
            let repo = &cap[2];
            let number: i64 = cap[3].parse()?;
            
            // We don't know if it's an issue or PR from short form, so we'll check both
            found_refs.insert((owner.to_string(), repo.to_string(), "issue".to_string(), number));
            found_refs.insert((owner.to_string(), repo.to_string(), "pull_request".to_string(), number));
        }
        
        // Store cross-references for registered repositories
        for (owner, repo, ref_type, number) in found_refs {
            let full_name = format!("{}/{}", owner, repo);
            
            // Check if target repository is registered
            if let Some(target_repo) = self.db.get_repository_by_full_name(&full_name).await? {
                let cross_ref = CrossReference {
                    source_type: source_type.to_string(),
                    source_id,
                    source_repository_id: source_repo_id,
                    target_type: ref_type,
                    target_repository_id: target_repo.id,
                    target_number: number,
                    link_text: format!("{}#{}", full_name, number),
                    created_at: Utc::now(),
                };
                
                self.db.add_cross_reference(&cross_ref)?;
            }
        }
        
        Ok(())
    }
    
    async fn update_sync_status(&self, repository_id: i64, resource_type: &str, status: &str, error_message: Option<&str>, items_synced: usize) -> Result<()> {
        let sync_status = SyncStatus {
            id: 0, // Will be assigned by database
            repository_id,
            resource_type: resource_type.to_string(),
            last_synced_at: Utc::now(),
            status: status.to_string(),
            error_message: error_message.map(|s| s.to_string()),
            items_synced: items_synced as i64,
        };
        
        self.db.update_sync_status(&sync_status).await?;
        
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct SyncResult {
    pub issues_synced: usize,
    pub pull_requests_synced: usize,
    pub errors: Vec<String>,
}

fn parse_repo_url(url: &str) -> Result<(String, String)> {
    // Support various GitHub URL formats
    let patterns = [
        Regex::new(r"^https://github\.com/([^/]+)/([^/]+)(?:\.git)?/?$")?,
        Regex::new(r"^git@github\.com:([^/]+)/([^/]+)(?:\.git)?$")?,
        Regex::new(r"^([^/]+)/([^/]+)$")?, // Simple owner/repo format
    ];
    
    for pattern in &patterns {
        if let Some(caps) = pattern.captures(url) {
            return Ok((caps[1].to_string(), caps[2].to_string()));
        }
    }
    
    anyhow::bail!("Invalid repository URL format: {}", url)
}