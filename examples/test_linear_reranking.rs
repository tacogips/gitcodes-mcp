use gitdb::storage::{SearchStore, UnifiedSearchQuery, SearchResult};
use gitdb::storage::search_store::hybrid::RerankStrategy;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Use a temporary directory for this example
    let temp_dir = PathBuf::from("/tmp/gitdb_linear_example");
    std::fs::create_dir_all(&temp_dir)?;
    
    println!("Linear Reranking Example\n");
    
    // Create search store
    let store = SearchStore::new(temp_dir.clone()).await?;
    println!("Search store created");
    
    // Example: Create a hybrid search query with Linear reranking (default)
    let query1 = UnifiedSearchQuery::hybrid("rust async")
        .with_limit(10);
    
    println!("Query 1 - Default Linear strategy:");
    match &query1.search_mode {
        gitdb::storage::SearchMode::Hybrid { rerank_strategy } => {
            match rerank_strategy {
                RerankStrategy::Linear { text_weight, vector_weight } => {
                    println!("  Text weight: {}", text_weight);
                    println!("  Vector weight: {}", vector_weight);
                }
                _ => println!("  Unexpected strategy"),
            }
        }
        _ => println!("  Not hybrid mode"),
    }
    
    // Example: Create a hybrid search with custom Linear weights
    let query2 = UnifiedSearchQuery::hybrid("database search")
        .with_rerank_strategy(RerankStrategy::Linear { 
            text_weight: 0.9, 
            vector_weight: 0.1 
        })
        .with_limit(10);
    
    println!("\nQuery 2 - Custom Linear strategy (text-heavy):");
    match &query2.search_mode {
        gitdb::storage::SearchMode::Hybrid { rerank_strategy } => {
            match rerank_strategy {
                RerankStrategy::Linear { text_weight, vector_weight } => {
                    println!("  Text weight: {}", text_weight);
                    println!("  Vector weight: {}", vector_weight);
                }
                _ => println!("  Unexpected strategy"),
            }
        }
        _ => println!("  Not hybrid mode"),
    }
    
    // Example: Create a hybrid search with RRF
    let query3 = UnifiedSearchQuery::hybrid("performance optimization")
        .with_rerank_strategy(RerankStrategy::RRF { k: 60.0 })
        .with_limit(10);
    
    println!("\nQuery 3 - RRF strategy:");
    match &query3.search_mode {
        gitdb::storage::SearchMode::Hybrid { rerank_strategy } => {
            match rerank_strategy {
                RerankStrategy::RRF { k } => {
                    println!("  k parameter: {}", k);
                }
                _ => println!("  Unexpected strategy"),
            }
        }
        _ => println!("  Not hybrid mode"),
    }
    
    // Clean up
    std::fs::remove_dir_all(&temp_dir)?;
    
    println!("\nLinear reranking implementation complete!");
    println!("\nKey differences:");
    println!("- Linear: Uses actual similarity scores with weighted combination");
    println!("- RRF: Uses only ranking positions, ignoring actual scores");
    println!("- Linear is better for homogeneous content (issues & PRs)");
    println!("- RRF is more robust for heterogeneous search systems");
    
    Ok(())
}