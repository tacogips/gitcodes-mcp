use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::types::{
    GitHubComment, GitHubIssue, GitHubPullRequest, GitHubPullRequestFile, GitHubRepository,
    GitHubUser, FullId,
};

pub struct SearchStore {
    _data_dir: PathBuf,
}

impl SearchStore {
    pub async fn new(data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self {
            _data_dir: data_dir,
        })
    }

    pub async fn save_repository(&self, _repo: &GitHubRepository) -> Result<()> {
        // TODO: Implement actual storage
        Ok(())
    }

    pub async fn get_repository(&self, _full_id: &FullId) -> Result<Option<GitHubRepository>> {
        // TODO: Implement actual retrieval
        Ok(None)
    }

    pub async fn search_repositories(&self, _query: &LanceDbQuery) -> Result<Vec<GitHubRepository>> {
        // TODO: Implement actual search
        Ok(vec![])
    }

    pub async fn save_issue(&self, _issue: &GitHubIssue) -> Result<()> {
        // TODO: Implement actual storage
        Ok(())
    }

    pub async fn search_issues(&self, _query: &LanceDbQuery) -> Result<Vec<GitHubIssue>> {
        // TODO: Implement actual search
        Ok(vec![])
    }

    pub async fn search_all(&self, _query: &LanceDbQuery) -> Result<Vec<SearchResult>> {
        // TODO: Implement actual search
        Ok(vec![])
    }

    pub async fn search(&self, _query: SearchQuery) -> Result<Vec<SearchResult>> {
        // TODO: Implement actual search
        Ok(vec![])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchResult {
    Repository(GitHubRepository),
    Issue(GitHubIssue),
    PullRequest(GitHubPullRequest),
    Comment(GitHubComment),
    User(GitHubUser),
    File(GitHubPullRequestFile),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub text: String,
    pub repository: Option<String>,
    pub state: Option<String>,
    pub label: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanceDbQuery {
    pub text: String,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub filter: Option<String>,
    pub search_fields: Option<Vec<String>>,
    pub select_fields: Option<Vec<String>>,
    pub fast_search: bool,
    pub postfilter: bool,
}

impl LanceDbQuery {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            limit: None,
            offset: None,
            filter: None,
            search_fields: None,
            select_fields: None,
            fast_search: false,
            postfilter: false,
        }
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = Some(filter.into());
        self
    }

    pub fn with_search_fields(mut self, fields: Vec<String>) -> Self {
        self.search_fields = Some(fields);
        self
    }

    pub fn with_select_fields(mut self, fields: Vec<String>) -> Self {
        self.select_fields = Some(fields);
        self
    }

    pub fn enable_fast_search(mut self) -> Self {
        self.fast_search = true;
        self
    }

    pub fn enable_postfilter(mut self) -> Self {
        self.postfilter = true;
        self
    }
}

pub mod hybrid {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub enum RerankStrategy {
        RRF { k: f32 },
        Linear { text_weight: f32, vector_weight: f32 },
        TextOnly,
        VectorOnly,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HybridSearchQuery {
        pub text_query: Option<String>,
        pub vector_query: Option<Vec<f32>>,
        pub base_params: LanceDbQuery,
        pub rerank_strategy: RerankStrategy,
    }

    impl HybridSearchQuery {
        pub fn new() -> Self {
            Self {
                text_query: None,
                vector_query: None,
                base_params: LanceDbQuery::new(""),
                rerank_strategy: RerankStrategy::RRF { k: 60.0 },
            }
        }

        pub fn with_text(mut self, text: impl Into<String>) -> Self {
            self.text_query = Some(text.into());
            self
        }

        pub fn with_vector(mut self, vector: Vec<f32>) -> Self {
            self.vector_query = Some(vector);
            self
        }

        pub fn with_rerank_strategy(mut self, strategy: RerankStrategy) -> Self {
            self.rerank_strategy = strategy;
            self
        }
    }
}