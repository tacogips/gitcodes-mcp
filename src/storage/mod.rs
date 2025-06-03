pub mod database;
pub mod enhanced_search;
#[cfg(feature = "lancedb-backend")]
pub mod lancedb_store;
pub mod models;
pub mod paths;
#[cfg(test)]
mod search_tests;

pub use database::*;
pub use enhanced_search::{EnhancedSearch, SearchConfig, SearchQueryBuilder, SearchResult as EnhancedSearchResult};
#[cfg(feature = "lancedb-backend")]
pub use lancedb_store::{LanceDbStore, SearchResult};
pub use models::*;
pub use paths::*;
