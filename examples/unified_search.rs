use anyhow::Result;
use gitdb::storage::{SearchStore, UnifiedSearchQuery, SearchMode};
use gitdb::storage::search_store::hybrid::RerankStrategy;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize the search store
    let data_dir = PathBuf::from("./lance_data");
    let store = SearchStore::new(data_dir).await?;

    // Example 1: Full Text Search
    println!("=== Full Text Search ===");
    let results = store.unified_search(
        UnifiedSearchQuery::full_text("memory leak")
            .with_limit(5)
            .with_filter("state = 'open'")
    ).await?;
    println!("Found {} results for full text search", results.len());

    // Example 2: Semantic Search from text
    println!("\n=== Semantic Search (from text) ===");
    let results = store.unified_search(
        UnifiedSearchQuery::semantic_from_text("authentication issues")
            .with_limit(5)
    ).await?;
    println!("Found {} results for semantic search", results.len());

    // Example 3: Semantic Search from pre-computed vector
    println!("\n=== Semantic Search (from vector) ===");
    // In real usage, this would be a pre-computed embedding
    let embedding = vec![0.1; 384]; // Example 384-dimensional vector
    let results = store.unified_search(
        UnifiedSearchQuery::semantic_from_vector(embedding)
            .with_limit(5)
            .with_filter("repository_id = 'rust-lang/rust'")
    ).await?;
    println!("Found {} results for vector search", results.len());

    // Example 4: Hybrid Search with default RRF reranking
    println!("\n=== Hybrid Search (default RRF) ===");
    let results = store.unified_search(
        UnifiedSearchQuery::hybrid("performance optimization")
            .with_limit(10)
    ).await?;
    println!("Found {} results for hybrid search", results.len());

    // Example 5: Hybrid Search with custom reranking strategy
    println!("\n=== Hybrid Search (custom reranking) ===");
    let results = store.unified_search(
        UnifiedSearchQuery::hybrid("async runtime bug")
            .with_rerank_strategy(RerankStrategy::Linear { 
                text_weight: 0.7, 
                vector_weight: 0.3 
            })
            .with_limit(10)
    ).await?;
    println!("Found {} results for hybrid search with linear reranking", results.len());

    // Example 6: Using SearchMode enum directly
    println!("\n=== Using SearchMode enum ===");
    let query = UnifiedSearchQuery::new(SearchMode::Hybrid { 
        rerank_strategy: RerankStrategy::RRF { k: 50.0 } 
    });
    // Need to provide text for hybrid search
    let query = UnifiedSearchQuery {
        text: Some("test query".to_string()),
        ..query
    };
    let results = store.unified_search(query.with_limit(5)).await?;
    println!("Found {} results", results.len());

    Ok(())
}