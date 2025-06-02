use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use octocrab::models::{issues::Issue as OctoIssue, pulls::PullRequest as OctoPullRequest};
use octocrab::{Octocrab, Page};
use std::sync::Arc;

use crate::storage::models::{Issue, IssueComment, PullRequest, PullRequestComment, Repository};

pub struct GitHubClient {
    client: Arc<Octocrab>,
}

impl GitHubClient {
    pub fn new(token: Option<String>) -> Result<Self> {
        let mut builder = Octocrab::builder();
        
        if let Some(token) = token {
            builder = builder.personal_token(token);
        }
        
        let client = builder.build()
            .context("Failed to build GitHub client")?;
        
        Ok(Self {
            client: Arc::new(client),
        })
    }
    
    pub async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository> {
        let repo = self.client
            .repos(owner, name)
            .get()
            .await
            .context("Failed to fetch repository")?;
        
        Ok(Repository {
            id: repo.id.0 as i64,
            owner: repo.owner.as_ref().map(|o| o.login.clone()).unwrap_or_default(),
            name: repo.name.clone(),
            full_name: repo.full_name.as_ref().cloned().unwrap_or_default(),
            description: repo.description.clone(),
            stars: repo.stargazers_count.unwrap_or(0) as i64,
            forks: repo.forks_count.unwrap_or(0) as i64,
            language: repo.language.as_ref().and_then(|v| v.as_str()).map(|s| s.to_string()),
            created_at: repo.created_at.unwrap_or_else(Utc::now),
            updated_at: repo.updated_at.unwrap_or_else(Utc::now),
            indexed_at: Utc::now(),
        })
    }
    
    pub async fn get_issues(&self, owner: &str, name: &str, repository_id: i64, since: Option<DateTime<Utc>>) -> Result<Vec<Issue>> {
        let mut page = 1u32;
        let mut all_issues = Vec::new();
        
        loop {
            let issues_page: Page<OctoIssue> = if let Some(since_date) = since {
                self.client
                    .issues(owner, name)
                    .list()
                    .state(octocrab::params::State::All)
                    .per_page(100)
                    .page(page)
                    .since(since_date)
                    .send()
                    .await
                    .context("Failed to fetch issues")?
            } else {
                self.client
                    .issues(owner, name)
                    .list()
                    .state(octocrab::params::State::All)
                    .per_page(100)
                    .page(page)
                    .send()
                    .await
                    .context("Failed to fetch issues")?
            };
            
            let issues: Vec<Issue> = issues_page
                .items
                .into_iter()
                .filter(|issue| issue.pull_request.is_none()) // Filter out pull requests
                .map(|issue| self.convert_issue(issue, repository_id))
                .collect();
            
            all_issues.extend(issues);
            
            if issues_page.next.is_none() {
                break;
            }
            
            page += 1;
        }
        
        Ok(all_issues)
    }
    
    pub async fn get_pull_requests(&self, owner: &str, name: &str, repository_id: i64, since: Option<DateTime<Utc>>) -> Result<Vec<PullRequest>> {
        let mut page = 1u32;
        let mut all_prs = Vec::new();
        
        loop {
            let pulls_page = self.client
                .pulls(owner, name)
                .list()
                .per_page(100)
                .page(page)
                .send()
                .await
                .context("Failed to fetch pull requests")?;
            
            let prs: Vec<PullRequest> = pulls_page
                .items
                .into_iter()
                .filter(|pr| {
                    if let Some(since_date) = since {
                        pr.updated_at.map(|d| DateTime::<Utc>::from(d) > since_date).unwrap_or(false)
                    } else {
                        true
                    }
                })
                .map(|pr| self.convert_pull_request(pr, repository_id))
                .collect::<Result<Vec<_>>>()?;
            
            all_prs.extend(prs);
            
            if pulls_page.next.is_none() {
                break;
            }
            
            page += 1;
        }
        
        Ok(all_prs)
    }
    
    pub async fn get_issue_comments(&self, owner: &str, name: &str, issue_number: u64, issue_id: i64) -> Result<Vec<IssueComment>> {
        let mut page = 1u32;
        let mut all_comments = Vec::new();
        
        loop {
            let comments_page = self.client
                .issues(owner, name)
                .list_comments(issue_number)
                .per_page(100)
                .page(page)
                .send()
                .await
                .context("Failed to fetch issue comments")?;
            
            let comments: Vec<IssueComment> = comments_page
                .items
                .into_iter()
                .map(|comment| IssueComment {
                    id: 0, // Will be assigned by database
                    issue_id,
                    comment_id: comment.id.0 as i64,
                    author: comment.user.login.clone(),
                    body: comment.body.unwrap_or_default(),
                    created_at: comment.created_at,
                    updated_at: comment.updated_at.unwrap_or_else(Utc::now),
                })
                .collect();
            
            all_comments.extend(comments);
            
            if comments_page.next.is_none() {
                break;
            }
            
            page += 1;
        }
        
        Ok(all_comments)
    }
    
    pub async fn get_pull_request_comments(&self, owner: &str, name: &str, pr_number: u64, pr_id: i64) -> Result<Vec<PullRequestComment>> {
        let mut page = 1u32;
        let mut all_comments = Vec::new();
        
        loop {
            // Get issue comments (pull requests also have issue comments)
            let comments_page = self.client
                .issues(owner, name)
                .list_comments(pr_number)
                .per_page(100)
                .page(page)
                .send()
                .await
                .context("Failed to fetch pull request comments")?;
            
            let comments: Vec<PullRequestComment> = comments_page
                .items
                .into_iter()
                .map(|comment| PullRequestComment {
                    id: 0, // Will be assigned by database
                    pull_request_id: pr_id,
                    comment_id: comment.id.0 as i64,
                    author: comment.user.login.clone(),
                    body: comment.body.unwrap_or_default(),
                    created_at: comment.created_at,
                    updated_at: comment.updated_at.unwrap_or_else(Utc::now),
                })
                .collect();
            
            all_comments.extend(comments);
            
            if comments_page.next.is_none() {
                break;
            }
            
            page += 1;
        }
        
        Ok(all_comments)
    }
    
    fn convert_issue(&self, issue: OctoIssue, repository_id: i64) -> Issue {
        Issue {
            id: issue.id.0 as i64,
            repository_id,
            number: issue.number as i64,
            title: issue.title,
            body: issue.body,
            state: match issue.state {
                octocrab::models::IssueState::Open => "open",
                octocrab::models::IssueState::Closed => "closed",
                _ => "unknown",
            }.to_string(),
            author: issue.user.login.clone(),
            assignees: issue.assignees.into_iter().map(|a| a.login.clone()).collect(),
            labels: issue.labels.into_iter().map(|l| l.name).collect(),
            created_at: issue.created_at,
            updated_at: issue.updated_at,
            closed_at: issue.closed_at,
            comments_count: issue.comments as i64,
        }
    }
    
    fn convert_pull_request(&self, pr: OctoPullRequest, repository_id: i64) -> Result<PullRequest> {
        // For detailed PR info, we need to fetch it separately
        Ok(PullRequest {
            id: pr.id.0 as i64,
            repository_id,
            number: pr.number as i64,
            title: pr.title.clone().unwrap_or_default(),
            body: pr.body.clone(),
            state: if pr.merged_at.is_some() { 
                "merged".to_string()
            } else { 
                match pr.state {
                    Some(octocrab::models::IssueState::Open) => "open",
                    Some(octocrab::models::IssueState::Closed) => "closed",
                    _ => "open",
                }.to_string()
            },
            author: pr.user.as_ref().map(|u| u.login.clone()).unwrap_or_default(),
            assignees: pr.assignees.as_ref().map(|a| a.iter().map(|u| u.login.clone()).collect()).unwrap_or_default(),
            labels: pr.labels.as_ref().map(|l| l.iter().map(|label| label.name.clone()).collect()).unwrap_or_default(),
            head_ref: pr.head.ref_field.clone(),
            base_ref: pr.base.ref_field.clone(),
            created_at: pr.created_at.unwrap_or_else(Utc::now),
            updated_at: pr.updated_at.unwrap_or_else(Utc::now),
            merged_at: pr.merged_at,
            closed_at: pr.closed_at,
            comments_count: pr.comments.unwrap_or(0) as i64,
            commits_count: pr.commits.unwrap_or(0) as i64,
            additions: pr.additions.unwrap_or(0) as i64,
            deletions: pr.deletions.unwrap_or(0) as i64,
            changed_files: pr.changed_files.unwrap_or(0) as i64,
        })
    }
}