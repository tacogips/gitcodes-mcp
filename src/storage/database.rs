use anyhow::{Context, Result};
use chrono::Utc;
use native_db::*;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, doc};

use crate::ids::{IssueId, PullRequestId, RepositoryId, UserId};
use crate::storage::models::*;
use crate::storage::paths::StoragePaths;
use crate::types::ResourceType;

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
    models
});

pub struct GitDatabase {
    db: Database<'static>,
    search_index: Index,
    index_writer: Arc<std::sync::Mutex<IndexWriter>>,
    index_reader: IndexReader,
    paths: StoragePaths,
}

impl GitDatabase {
    /// Creates a new GitDatabase instance with search index and storage.
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
    /// - Tantivy search index creation or opening fails
    pub async fn new() -> Result<Self> {
        let paths = StoragePaths::new()?;

        // Open native_db database
        let db = Builder::new()
            .create(&*MODELS, &paths.database_path())
            .context("Failed to open native_db database")?;

        // Create tantivy search index
        let mut schema_builder = Schema::builder();

        // Define fields for search
        schema_builder.add_text_field("id", STRING | STORED);
        schema_builder.add_text_field("type", STRING | STORED);
        schema_builder.add_text_field("repository_id", STRING | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("body", TEXT | STORED);
        schema_builder.add_text_field("author", TEXT | STORED);
        schema_builder.add_text_field("labels", TEXT | STORED);
        schema_builder.add_text_field("assignees", TEXT | STORED);
        schema_builder.add_text_field("commenters", TEXT | STORED);
        schema_builder.add_text_field("participants", TEXT | STORED);

        let schema = schema_builder.build();

        // Open or create search index
        let search_index = if paths.search_index_path().exists() {
            Index::open_in_dir(&paths.search_index_path())
                .context("Failed to open search index")?
        } else {
            std::fs::create_dir_all(&paths.search_index_path())?;
            Index::create_in_dir(&paths.search_index_path(), schema.clone())
                .context("Failed to create search index")?
        };

        let index_writer = Arc::new(std::sync::Mutex::new(
            search_index
                .writer(50_000_000)
                .context("Failed to create index writer")?,
        ));

        let index_reader = search_index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()
            .context("Failed to create index reader")?;

        Ok(Self {
            db,
            search_index,
            index_writer,
            index_reader,
            paths,
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

        // Index for search
        self.index_repository(repo).await?;
        Ok(())
    }

    pub async fn get_repository(&self, id: &RepositoryId) -> Result<Option<Repository>> {
        let r = self.db.r_transaction()?;
        Ok(r.get().primary(id.clone())?)
    }

    pub async fn get_repository_by_full_name(&self, full_name: &str) -> Result<Option<Repository>> {
        let r = self.db.r_transaction()?;
        Ok(r.get()
            .secondary(RepositoryKey::full_name, full_name)?)
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

        // Index for search
        self.index_issue(issue).await?;
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

        // Index for search
        self.index_pull_request(pr).await?;
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

    // Search operations
    async fn index_repository(&self, repo: &Repository) -> Result<()> {
        let mut writer = self.index_writer.lock().unwrap();
        let schema = self.search_index.schema();

        let id_field = schema.get_field("id").unwrap();
        let type_field = schema.get_field("type").unwrap();
        let repository_id_field = schema.get_field("repository_id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();

        let mut doc = doc!();
        doc.add_text(id_field, &repo.id.to_string());
        doc.add_text(type_field, "repository");
        doc.add_text(repository_id_field, &repo.id.to_string());
        doc.add_text(title_field, &repo.full_name);
        if let Some(desc) = &repo.description {
            doc.add_text(body_field, desc);
        }
        doc.add_text(author_field, &repo.owner);

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    async fn index_issue(&self, issue: &Issue) -> Result<()> {
        // Get all participants for this issue first (before locking)
        let participants = self.get_issue_participants(issue.id).await?;
        let participant_logins: Vec<String> = participants.iter()
            .map(|u| u.login.clone())
            .collect();
        
        // Separate commenters (those who are participants but not assignees or author)
        let mut commenters: Vec<String> = participant_logins.iter()
            .filter(|login| **login != issue.author && !issue.assignees.contains(login))
            .cloned()
            .collect();
        commenters.sort();
        commenters.dedup();

        // Now lock and index
        let mut writer = self.index_writer.lock().unwrap();
        let schema = self.search_index.schema();

        let id_field = schema.get_field("id").unwrap();
        let type_field = schema.get_field("type").unwrap();
        let repository_id_field = schema.get_field("repository_id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();
        let labels_field = schema.get_field("labels").unwrap();
        let assignees_field = schema.get_field("assignees").unwrap();
        let commenters_field = schema.get_field("commenters").unwrap();
        let participants_field = schema.get_field("participants").unwrap();

        let mut doc = doc!();
        doc.add_text(id_field, &issue.id.to_string());
        doc.add_text(type_field, "issue");
        doc.add_text(repository_id_field, &issue.repository_id.to_string());
        doc.add_text(title_field, &issue.title);
        if let Some(body) = &issue.body {
            doc.add_text(body_field, body);
        }
        doc.add_text(author_field, &issue.author);
        doc.add_text(labels_field, &issue.labels.join(" "));
        doc.add_text(assignees_field, &issue.assignees.join(" "));
        doc.add_text(commenters_field, &commenters.join(" "));
        doc.add_text(participants_field, &participant_logins.join(" "));

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    async fn index_pull_request(&self, pr: &PullRequest) -> Result<()> {
        // Get all participants for this PR first (before locking)
        let participants = self.get_pull_request_participants(pr.id).await?;
        let participant_logins: Vec<String> = participants.iter()
            .map(|u| u.login.clone())
            .collect();
        
        // Separate commenters (those who are participants but not assignees or author)
        let mut commenters: Vec<String> = participant_logins.iter()
            .filter(|login| **login != pr.author && !pr.assignees.contains(login))
            .cloned()
            .collect();
        commenters.sort();
        commenters.dedup();

        // Now lock and index
        let mut writer = self.index_writer.lock().unwrap();
        let schema = self.search_index.schema();

        let id_field = schema.get_field("id").unwrap();
        let type_field = schema.get_field("type").unwrap();
        let repository_id_field = schema.get_field("repository_id").unwrap();
        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();
        let labels_field = schema.get_field("labels").unwrap();
        let assignees_field = schema.get_field("assignees").unwrap();
        let commenters_field = schema.get_field("commenters").unwrap();
        let participants_field = schema.get_field("participants").unwrap();

        let mut doc = doc!();
        doc.add_text(id_field, &pr.id.to_string());
        doc.add_text(type_field, "pull_request");
        doc.add_text(repository_id_field, &pr.repository_id.to_string());
        doc.add_text(title_field, &pr.title);
        if let Some(body) = &pr.body {
            doc.add_text(body_field, body);
        }
        doc.add_text(author_field, &pr.author);
        doc.add_text(labels_field, &pr.labels.join(" "));
        doc.add_text(assignees_field, &pr.assignees.join(" "));
        doc.add_text(commenters_field, &commenters.join(" "));
        doc.add_text(participants_field, &participant_logins.join(" "));

        writer.add_document(doc)?;
        writer.commit()?;
        Ok(())
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let searcher = self.index_reader.searcher();
        let schema = self.search_index.schema();

        let title_field = schema.get_field("title").unwrap();
        let body_field = schema.get_field("body").unwrap();
        let author_field = schema.get_field("author").unwrap();
        let labels_field = schema.get_field("labels").unwrap();
        let assignees_field = schema.get_field("assignees").unwrap();
        let commenters_field = schema.get_field("commenters").unwrap();
        let participants_field = schema.get_field("participants").unwrap();

        let query_parser = QueryParser::for_index(
            &self.search_index,
            vec![title_field, body_field, author_field, labels_field, assignees_field, commenters_field, participants_field],
        );

        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let id = retrieved_doc
                .get_first(schema.get_field("id").unwrap())
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let item_type = retrieved_doc
                .get_first(schema.get_field("type").unwrap())
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let repository_id = retrieved_doc
                .get_first(schema.get_field("repository_id").unwrap())
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let title = retrieved_doc
                .get_first(schema.get_field("title").unwrap())
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            let body = retrieved_doc
                .get_first(schema.get_field("body").unwrap())
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            results.push(SearchResult {
                id: id.to_string(),
                item_type: item_type.to_string(),
                repository_id: repository_id.to_string(),
                title: title.to_string(),
                body,
            });
        }

        Ok(results)
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
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub item_type: String,
    pub repository_id: String,
    pub title: String,
    pub body: Option<String>,
}