use anyhow::{Context, Result};
use chrono::Utc;
use native_db::*;
use once_cell::sync::Lazy;

use crate::ids::{IssueId, ProjectId, PullRequestId, RepositoryId, UserId};
use crate::storage::models::*;
use crate::storage::paths::StoragePaths;
use crate::storage::search_store::{self, SearchStore, SearchQuery, SearchResult};
use crate::types::{ResourceType, RepositoryName};

static MODELS: Lazy<Models> = Lazy::new(|| {
    let mut models = Models::new();
    models.define::<Repository>().unwrap();
    models.define::<Issue>().unwrap();
    models.define::<PullRequest>().unwrap();
    models.define::<IssueComment>().unwrap();
    models.define::<PullRequestComment>().unwrap();
    models.define::<SyncStatus>().unwrap();
    models.define::<CrossReference>().unwrap();
    models.define::<User>().unwrap();
    models.define::<IssueParticipant>().unwrap();
    models.define::<PullRequestParticipant>().unwrap();
    models.define::<Project>().unwrap();
    models.define::<ProjectItem>().unwrap();
    models
});

pub struct GitDatabase {
    db: Database<'static>,
    paths: StoragePaths,
    search_store: SearchStore,
}

impl GitDatabase {
    /// Creates a new GitDatabase instance with storage.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the initialized GitDatabase.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Storage paths initialization fails
    /// - Native_db database opening fails
    pub async fn new() -> Result<Self> {
        let paths = StoragePaths::new()?;

        // Open native_db database
        let db = Builder::new()
            .create(&*MODELS, &paths.database_path())
            .context("Failed to open native_db database")?;

        // Initialize search store
        let search_store = SearchStore::new(paths.search_store_path())
            .await
            .context("Failed to initialize search store")?;

        Ok(Self {
            db,
            paths,
            search_store,
        })
    }

    /// Get the storage paths
    pub fn paths(&self) -> &StoragePaths {
        &self.paths
    }

    // Repository operations
    pub async fn save_repository(&self, repo: &Repository) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(repo.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn get_repository(&self, id: &RepositoryId) -> Result<Option<Repository>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id.clone())?)
    }

    pub async fn get_repository_by_full_name(&self, full_name: &RepositoryName) -> Result<Option<Repository>> {
        let r = self.db.r_transaction()?;
        Ok(r.get()
            .secondary(RepositoryKey::full_name, full_name.as_str())?)
    }

    pub async fn list_repositories(&self) -> Result<Vec<Repository>> {
        let r = self.db.r_transaction()?;
        let repos: Vec<Repository> = r.scan().primary::<Repository>()?.all()?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(repos)
    }

    pub async fn delete_repository(&self, id: &RepositoryId) -> Result<()> {
        let r = self.db.r_transaction()?;
        if let Some(repo) = r.get().primary::<Repository>(id.clone())? {
            let rw = self.db.rw_transaction()?;
            rw.remove(repo)?;
            
            // Delete all related data
            // Delete issues
            let issues: Vec<Issue> = r.scan()
                .secondary(IssueKey::repository_id)?
                .start_with(*id)?
                .collect::<Result<Vec<_>, _>>()?;
            for issue in issues {
                rw.remove(issue)?;
            }
            
            // Delete pull requests
            let prs: Vec<PullRequest> = r.scan()
                .secondary(PullRequestKey::repository_id)?
                .start_with(*id)?
                .collect::<Result<Vec<_>, _>>()?;
            for pr in prs {
                rw.remove(pr)?;
            }
            
            // Delete sync status
            let statuses: Vec<SyncStatus> = r.scan()
                .secondary(SyncStatusKey::repository_id)?
                .start_with(*id)?
                .collect::<Result<Vec<_>, _>>()?;
            for status in statuses {
                rw.remove(status)?;
            }
            
            rw.commit()?;
        }
        Ok(())
    }

    // Issue operations
    pub async fn save_issue(&self, issue: &Issue) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(issue.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn get_issue(&self, id: &IssueId) -> Result<Option<Issue>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id.clone())?)
    }

    pub async fn list_issues_by_repository(&self, repo_id: &RepositoryId) -> Result<Vec<Issue>> {
        let r = self.db.r_transaction()?;
        let issues: Vec<Issue> = r.scan()
            .secondary(IssueKey::repository_id)?
            .start_with(*repo_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(issues)
    }

    // Pull Request operations
    pub async fn save_pull_request(&self, pr: &PullRequest) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(pr.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn get_pull_request(&self, id: &PullRequestId) -> Result<Option<PullRequest>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id.clone())?)
    }

    pub async fn list_pull_requests_by_repository(
        &self,
        repo_id: &RepositoryId,
    ) -> Result<Vec<PullRequest>> {
        let r = self.db.r_transaction()?;
        let prs: Vec<PullRequest> = r.scan()
            .secondary(PullRequestKey::repository_id)?
            .start_with(*repo_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(prs)
    }

    // Comment operations
    pub async fn save_issue_comment(&self, comment: &IssueComment) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(comment.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn list_issue_comments(&self, issue_id: &IssueId) -> Result<Vec<IssueComment>> {
        let r = self.db.r_transaction()?;
        let comments: Vec<IssueComment> = r.scan()
            .secondary(IssueCommentKey::issue_id)?
            .start_with(*issue_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(comments)
    }

    pub async fn save_pull_request_comment(&self, comment: &PullRequestComment) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(comment.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn list_pull_request_comments(
        &self,
        pr_id: &PullRequestId,
    ) -> Result<Vec<PullRequestComment>> {
        let r = self.db.r_transaction()?;
        let comments: Vec<PullRequestComment> = r.scan()
            .secondary(PullRequestCommentKey::pull_request_id)?
            .start_with(*pr_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(comments)
    }

    // Sync status operations
    pub async fn save_sync_status(&self, status: &SyncStatus) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(status.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn get_sync_status(
        &self,
        repo_id: &RepositoryId,
        resource_type: ResourceType,
    ) -> Result<Option<SyncStatus>> {
        let r = self.db.r_transaction()?;
        let statuses: Vec<SyncStatus> = r.scan()
            .secondary(SyncStatusKey::repository_id)?
            .start_with(*repo_id)?
            .filter_map(|s: Result<SyncStatus, _>| match s {
                Ok(status) if status.resource_type == resource_type => Some(Ok(status)),
                Ok(_) => None,
                Err(e) => Some(Err(e)),
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(statuses.into_iter().next())
    }

    // Cross-reference operations
    pub async fn save_cross_reference(&self, xref: &CrossReference) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(xref.clone())?;
        rw.commit()?;
        Ok(())
    }

    pub async fn list_cross_references_from(
        &self,
        source_repo_id: &RepositoryId,
    ) -> Result<Vec<CrossReference>> {
        let r = self.db.r_transaction()?;
        let xrefs: Vec<CrossReference> = r.scan()
            .secondary(CrossReferenceKey::source_repository_id)?
            .start_with(*source_repo_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(xrefs)
    }

    pub async fn list_cross_references_to(
        &self,
        target_repo_id: &RepositoryId,
    ) -> Result<Vec<CrossReference>> {
        let r = self.db.r_transaction()?;
        let xrefs: Vec<CrossReference> = r.scan()
            .secondary(CrossReferenceKey::target_repository_id)?
            .start_with(*target_repo_id)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(xrefs)
    }

    // User management methods
    pub async fn get_or_create_user(
        &self,
        github_id: i64,
        login: &str,
        avatar_url: Option<String>,
        html_url: Option<String>,
        user_type: &str,
        site_admin: bool,
    ) -> Result<User> {
        let user_id = UserId::new(github_id);
        
        let rw = self.db.rw_transaction()?;
        
        match rw.get().primary::<User>(user_id)? {
            Some(existing_user) => {
                // User exists, update if data changed
                let mut updated_user = existing_user.clone();
                if updated_user.login != login {
                    updated_user.login = login.to_string();
                }
                // Update other fields
                updated_user.avatar_url = avatar_url;
                updated_user.html_url = html_url;
                updated_user.user_type = user_type.to_string();
                updated_user.site_admin = site_admin;
                updated_user.last_updated_at = Utc::now();
                
                rw.update(existing_user, updated_user.clone())?;
                rw.commit()?;
                Ok(updated_user)
            }
            None => {
                // Create new user
                let now = Utc::now();
                let new_user = User {
                    id: user_id,
                    login: login.to_string(),
                    avatar_url,
                    html_url,
                    user_type: user_type.to_string(),
                    site_admin,
                    first_seen_at: now,
                    last_updated_at: now,
                };
                rw.insert(new_user.clone())?;
                rw.commit()?;
                Ok(new_user)
            }
        }
    }
    
    pub async fn get_user_by_login(&self, login: &str) -> Result<Option<User>> {
        let r = self.db.r_transaction()?;
        Ok(r.get()
            .secondary::<User>(UserKey::login, login)?)
    }
    
    pub async fn get_users_by_ids(&self, user_ids: &[UserId]) -> Result<Vec<User>> {
        let r = self.db.r_transaction()?;
        let mut users = Vec::new();
        
        for user_id in user_ids {
            if let Some(user) = r.get().primary::<User>(*user_id)? {
                users.push(user);
            }
        }
        
        Ok(users)
    }
    
    // Participant management methods
    pub async fn add_issue_participant(
        &self,
        issue_id: IssueId,
        user_id: UserId,
        participation_type: ParticipationType,
    ) -> Result<()> {
        let id = format!("{}:{}", issue_id, user_id);
        let participant = IssueParticipant {
            id,
            issue_id,
            user_id,
            participation_type,
            created_at: Utc::now(),
        };
        
        let rw = self.db.rw_transaction()?;
        rw.insert(participant)?;
        rw.commit()?;
        Ok(())
    }
    
    pub async fn add_pull_request_participant(
        &self,
        pull_request_id: PullRequestId,
        user_id: UserId,
        participation_type: ParticipationType,
    ) -> Result<()> {
        let id = format!("{}:{}", pull_request_id, user_id);
        let participant = PullRequestParticipant {
            id,
            pull_request_id,
            user_id,
            participation_type,
            created_at: Utc::now(),
        };
        
        let rw = self.db.rw_transaction()?;
        rw.insert(participant)?;
        rw.commit()?;
        Ok(())
    }
    
    pub async fn get_issue_participants(&self, issue_id: IssueId) -> Result<Vec<User>> {
        let r = self.db.r_transaction()?;
        
        // Get all participant records for this issue
        let participants: Vec<IssueParticipant> = r.scan()
            .secondary(IssueParticipantKey::issue_id)?
            .start_with(issue_id)?
            .collect::<Result<Vec<_>, _>>()?;
        
        // Collect unique user IDs
        let user_ids: Vec<UserId> = participants.into_iter()
            .map(|p| p.user_id)
            .collect();
        
        // Fetch all users
        self.get_users_by_ids(&user_ids).await
    }
    
    pub async fn get_pull_request_participants(&self, pr_id: PullRequestId) -> Result<Vec<User>> {
        let r = self.db.r_transaction()?;
        
        // Get all participant records for this PR
        let participants: Vec<PullRequestParticipant> = r.scan()
            .secondary(PullRequestParticipantKey::pull_request_id)?
            .start_with(pr_id)?
            .collect::<Result<Vec<_>, _>>()?;
        
        // Collect unique user IDs
        let user_ids: Vec<UserId> = participants.into_iter()
            .map(|p| p.user_id)
            .collect();
        
        // Fetch all users
        self.get_users_by_ids(&user_ids).await
    }
    
    // Project management methods
    
    /// Saves a project to the database
    pub async fn save_project(&self, project: Project) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(project)?;
        rw.commit()?;
        Ok(())
    }
    
    /// Gets a project by ID
    pub async fn get_project(&self, project_id: &ProjectId) -> Result<Option<Project>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary::<Project>(project_id.clone())?)
    }
    
    /// Gets all projects
    pub async fn get_all_projects(&self) -> Result<Vec<Project>> {
        let r = self.db.r_transaction()?;
        let projects: Vec<Project> = r.scan().primary::<Project>()?.all()?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(projects)
    }
    
    /// Gets all projects with their item counts
    pub async fn get_all_projects_with_stats(&self) -> Result<Vec<(Project, usize, usize)>> {
        let projects = self.get_all_projects().await?;
        let mut results = Vec::new();
        
        for project in projects {
            let items = self.get_project_items(&project.id).await?;
            let issue_count = items.iter()
                .filter(|item| matches!(item.item_type, crate::types::ItemType::Issue))
                .count();
            let pr_count = items.iter()
                .filter(|item| matches!(item.item_type, crate::types::ItemType::PullRequest))
                .count();
            
            results.push((project, issue_count, pr_count));
        }
        
        Ok(results)
    }
    
    /// Adds an issue or PR to a project
    pub async fn add_item_to_project(&self, project_item: ProjectItem) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        rw.insert(project_item)?;
        rw.commit()?;
        Ok(())
    }
    
    /// Gets all items in a project
    pub async fn get_project_items(&self, project_id: &ProjectId) -> Result<Vec<ProjectItem>> {
        let r = self.db.r_transaction()?;
        let items: Vec<ProjectItem> = r.scan()
            .secondary(ProjectItemKey::project_id)?
            .start_with(project_id.clone())?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(items)
    }
    
    /// Updates an issue to include a project reference
    pub async fn add_project_to_issue(&self, issue_id: IssueId, project_id: ProjectId) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        
        if let Some(issue) = rw.get().primary::<Issue>(issue_id)? {
            if !issue.project_ids.contains(&project_id) {
                let mut updated_issue = issue.clone();
                updated_issue.project_ids.push(project_id);
                rw.update(issue, updated_issue)?;
            }
        }
        
        rw.commit()?;
        Ok(())
    }
    
    /// Updates a PR to include a project reference
    pub async fn add_project_to_pull_request(&self, pr_id: PullRequestId, project_id: ProjectId) -> Result<()> {
        let rw = self.db.rw_transaction()?;
        
        if let Some(pr) = rw.get().primary::<PullRequest>(pr_id)? {
            if !pr.project_ids.contains(&project_id) {
                let mut updated_pr = pr.clone();
                updated_pr.project_ids.push(project_id);
                rw.update(pr, updated_pr)?;
            }
        }
        
        rw.commit()?;
        Ok(())
    }
    
    /// Gets all issues belonging to a project
    pub async fn get_issues_by_project(&self, project_id: &ProjectId) -> Result<Vec<Issue>> {
        let r = self.db.r_transaction()?;
        
        // First get all project items for this project
        let project_items = self.get_project_items(project_id).await?;
        
        // Filter for issue items and collect their IDs
        let issue_ids: Vec<IssueId> = project_items
            .into_iter()
            .filter_map(|item| {
                if matches!(item.item_type, crate::types::ItemType::Issue) {
                    Some(IssueId::new(item.item_id))
                } else {
                    None
                }
            })
            .collect();
        
        // Fetch all issues by their IDs
        let mut issues = Vec::new();
        for issue_id in issue_ids {
            if let Some(issue) = r.get().primary::<Issue>(issue_id)? {
                issues.push(issue);
            }
        }
        
        Ok(issues)
    }
    
    /// Gets all pull requests belonging to a project
    pub async fn get_pull_requests_by_project(&self, project_id: &ProjectId) -> Result<Vec<PullRequest>> {
        let r = self.db.r_transaction()?;
        
        // First get all project items for this project
        let project_items = self.get_project_items(project_id).await?;
        
        // Filter for PR items and collect their IDs
        let pr_ids: Vec<PullRequestId> = project_items
            .into_iter()
            .filter_map(|item| {
                if matches!(item.item_type, crate::types::ItemType::PullRequest) {
                    Some(PullRequestId::new(item.item_id))
                } else {
                    None
                }
            })
            .collect();
        
        // Fetch all PRs by their IDs
        let mut prs = Vec::new();
        for pr_id in pr_ids {
            if let Some(pr) = r.get().primary::<PullRequest>(pr_id)? {
                prs.push(pr);
            }
        }
        
        Ok(prs)
    }
    
    /// Lists issues filtered by multiple criteria including project
    pub async fn list_issues_filtered(
        &self,
        repo_id: Option<&RepositoryId>,
        project_id: Option<&ProjectId>,
        state: Option<crate::types::IssueState>,
    ) -> Result<Vec<Issue>> {
        let r = self.db.r_transaction()?;
        
        let mut issues = if let Some(repo_id) = repo_id {
            // Start with repository filter
            self.list_issues_by_repository(repo_id).await?
        } else if let Some(project_id) = project_id {
            // Start with project filter
            self.get_issues_by_project(project_id).await?
        } else {
            // Get all issues
            r.scan().primary::<Issue>()?.all()?
                .collect::<Result<Vec<_>, _>>()?
        };
        
        // Apply additional filters
        if let Some(project_id) = project_id {
            if repo_id.is_some() {
                // If we already filtered by repo, now filter by project
                issues.retain(|issue| issue.project_ids.contains(project_id));
            }
        }
        
        if let Some(state) = state {
            issues.retain(|issue| issue.state == state);
        }
        
        Ok(issues)
    }
    
    /// Lists pull requests filtered by multiple criteria including project
    pub async fn list_pull_requests_filtered(
        &self,
        repo_id: Option<&RepositoryId>,
        project_id: Option<&ProjectId>,
        state: Option<crate::types::PullRequestState>,
    ) -> Result<Vec<PullRequest>> {
        let r = self.db.r_transaction()?;
        
        let mut prs = if let Some(repo_id) = repo_id {
            // Start with repository filter
            self.list_pull_requests_by_repository(repo_id).await?
        } else if let Some(project_id) = project_id {
            // Start with project filter
            self.get_pull_requests_by_project(project_id).await?
        } else {
            // Get all PRs
            r.scan().primary::<PullRequest>()?.all()?
                .collect::<Result<Vec<_>, _>>()?
        };
        
        // Apply additional filters
        if let Some(project_id) = project_id {
            if repo_id.is_some() {
                // If we already filtered by repo, now filter by project
                prs.retain(|pr| pr.project_ids.contains(project_id));
            }
        }
        
        if let Some(state) = state {
            prs.retain(|pr| pr.state == state);
        }
        
        Ok(prs)
    }
    
    /// Checks if an issue belongs to a project
    pub async fn is_issue_in_project(&self, issue_id: &IssueId, project_id: &ProjectId) -> Result<bool> {
        if let Some(issue) = self.get_issue(issue_id).await? {
            Ok(issue.project_ids.contains(project_id))
        } else {
            Ok(false)
        }
    }
    
    /// Checks if a pull request belongs to a project
    pub async fn is_pull_request_in_project(&self, pr_id: &PullRequestId, project_id: &ProjectId) -> Result<bool> {
        if let Some(pr) = self.get_pull_request(pr_id).await? {
            Ok(pr.project_ids.contains(project_id))
        } else {
            Ok(false)
        }
    }
    
    /// Gets all projects that contain a specific issue
    pub async fn get_projects_for_issue(&self, issue_id: &IssueId) -> Result<Vec<Project>> {
        if let Some(issue) = self.get_issue(issue_id).await? {
            let mut projects = Vec::new();
            for project_id in &issue.project_ids {
                if let Some(project) = self.get_project(project_id).await? {
                    projects.push(project);
                }
            }
            Ok(projects)
        } else {
            Ok(Vec::new())
        }
    }
    
    /// Gets all projects that contain a specific pull request
    pub async fn get_projects_for_pull_request(&self, pr_id: &PullRequestId) -> Result<Vec<Project>> {
        if let Some(pr) = self.get_pull_request(pr_id).await? {
            let mut projects = Vec::new();
            for project_id in &pr.project_ids {
                if let Some(project) = self.get_project(project_id).await? {
                    projects.push(project);
                }
            }
            Ok(projects)
        } else {
            Ok(Vec::new())
        }
    }
    
    // Search methods that delegate to SearchStore
    
    /// Search across all entities using the search store
    pub async fn search(&self, query: SearchQuery) -> Result<Vec<SearchResult>> {
        self.search_store.search(query).await
    }
    
    /// Search specifically for repositories
    pub async fn search_repositories(&self, query: &search_store::LanceDbQuery) -> Result<Vec<crate::types::GitHubRepository>> {
        self.search_store.search_repositories(query).await
    }
    
    /// Search specifically for issues
    pub async fn search_issues(&self, query: &search_store::LanceDbQuery) -> Result<Vec<crate::types::GitHubIssue>> {
        self.search_store.search_issues(query).await
    }
    
    /// Get access to the search store for advanced search operations
    pub fn search_store(&self) -> &SearchStore {
        &self.search_store
    }
}