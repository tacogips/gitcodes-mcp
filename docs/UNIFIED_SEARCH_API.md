# Unified Search API Documentation

## Overview

The unified search API in GitDB provides a single interface for performing full-text search, semantic search, and hybrid search across GitHub repository data. This API combines the capabilities of LanceDB's full-text search (FTS) and vector search into a unified interface with flexible search modes.

## Search Modes

### 1. Full-Text Search (`SearchMode::FullText`)
- Uses LanceDB's built-in FTS index on the `searchable_content` field
- Searches through repository names, descriptions, issue titles, bodies, labels, etc.
- Best for keyword-based searches and exact phrase matching

### 2. Semantic Search (`SearchMode::Semantic`)
- Uses vector embeddings generated from the `searchable_content` field
- Finds semantically similar content even if exact keywords don't match
- Can accept either text (auto-generates embeddings) or pre-computed vectors
- Best for finding conceptually related issues/repositories

### 3. Hybrid Search (`SearchMode::Hybrid`)
- Combines both full-text and semantic search results
- Uses configurable reranking strategies to merge results:
  - **RRF (Reciprocal Rank Fusion)**: Default strategy with k=60.0
  - **Linear**: Weighted combination of text and vector scores
  - **TextOnly**: Uses only text search results
  - **VectorOnly**: Uses only vector search results
- Best for comprehensive searches that benefit from both exact matches and semantic similarity

## API Usage

### Basic Usage

```rust
use gitdb::storage::{SearchStore, UnifiedSearchQuery};

// Full-text search
let results = store.unified_search(
    UnifiedSearchQuery::full_text("memory leak")
        .with_limit(10)
).await?;

// Semantic search from text
let results = store.unified_search(
    UnifiedSearchQuery::semantic_from_text("authentication issues")
        .with_limit(10)
).await?;

// Hybrid search
let results = store.unified_search(
    UnifiedSearchQuery::hybrid("async runtime bug")
        .with_limit(10)
).await?;
```

### Advanced Options

```rust
use gitdb::storage::search_store::hybrid::RerankStrategy;

// With filters
let results = store.unified_search(
    UnifiedSearchQuery::full_text("bug")
        .with_filter("state = 'open' AND repository_id = 'rust-lang/rust'")
        .with_limit(20)
        .with_offset(0)
).await?;

// Custom reranking strategy for hybrid search
let results = store.unified_search(
    UnifiedSearchQuery::hybrid("performance")
        .with_rerank_strategy(RerankStrategy::Linear { 
            text_weight: 0.7, 
            vector_weight: 0.3 
        })
).await?;

// Semantic search with pre-computed embedding
let embedding: Vec<f32> = generate_embedding("your text"); // 384-dimensional vector
let results = store.unified_search(
    UnifiedSearchQuery::semantic_from_vector(embedding)
        .with_limit(5)
).await?;
```

## Filter Syntax

The unified search API supports SQL-style filter expressions:

- `state = 'open'` - Filter by issue/PR state
- `repository_id = 'owner/repo'` - Filter by repository
- `labels LIKE '%bug%'` - Filter by labels containing text
- Combined filters: `state = 'open' AND repository_id = 'rust-lang/rust'`

## Implementation Details

### Embedding Generation
- The `searchable_content` field is used to generate embeddings
- Default embedding dimension: 384 (suitable for models like all-MiniLM-L6-v2)
- Currently uses placeholder embeddings (zeros) - integrate with your preferred embedding model

### Index Configuration
- Full-text index on `searchable_content` field
- Vector index using IvfPq with 100 partitions and 16 sub-vectors
- Both repositories and issues tables have identical search capabilities

### Reranking Strategies

#### Reciprocal Rank Fusion (RRF)
```
score = 1 / (k + rank)
```
- Default k=60.0
- Combines rankings from text and vector search
- Robust to score scale differences

#### Linear Combination
```
score = text_weight * text_score + vector_weight * vector_score
```
- Requires normalized scores
- More control over contribution of each search type

## Migration from Separate APIs

If you're currently using separate text and vector search APIs:

```rust
// Old approach
let text_results = store.search_repositories(&query).await?;
let vector_results = store.vector_search_repositories(embedding, limit, filter).await?;

// New unified approach
let results = store.unified_search(
    UnifiedSearchQuery::hybrid("your query")
        .with_limit(limit)
        .with_filter(filter)
).await?;
```

## Performance Considerations

1. **Full-text search** is generally fastest for keyword matching
2. **Semantic search** requires embedding generation (if from text) and vector similarity computation
3. **Hybrid search** performs both searches and reranking, so it's slower but more comprehensive
4. Use filters to reduce the search space and improve performance

## Future Enhancements

1. **Real embedding model integration**: Replace placeholder embeddings with actual models
2. **Cached embeddings**: Store computed embeddings to avoid regeneration
3. **Query expansion**: Use synonyms and related terms for better recall
4. **Learning to rank**: Use ML models for more sophisticated reranking
5. **Multi-modal search**: Combine with code snippets, images, etc.