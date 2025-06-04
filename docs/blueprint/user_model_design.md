# User Model Design for GitDB

## Overview

This document outlines the design for implementing a dedicated User model to persist GitHub participants uniquely in the GitDB system.

## Proposed User Model

```rust
use chrono::{DateTime, Utc};
use native_db::*;
use native_model::{native_model, Model};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[native_model(id = 8, version = 1)]
#[native_db]
pub struct User {
    #[primary_key]
    pub id: UserId,                    // GitHub user ID (unique)
    #[secondary_key(unique)]
    pub login: String,                 // GitHub username (unique)
    pub avatar_url: Option<String>,    // Avatar URL
    pub html_url: Option<String>,      // Profile URL
    pub user_type: String,             // "User" or "Bot"
    pub site_admin: bool,              // GitHub site admin flag
    pub first_seen_at: DateTime<Utc>,  // When we first encountered this user
    pub last_updated_at: DateTime<Utc>, // Last time we updated user info
}
```

## ID Type Definition

```rust
// In ids.rs
define_id!(UserId, i64);
```

## Model Changes Required

### 1. Update Issue Model

Replace `author: String` with `author_id: UserId` and add `author_login: String` for backward compatibility:

```rust
pub struct Issue {
    // ... existing fields ...
    pub author_id: UserId,
    pub author_login: String,  // Keep for display/search purposes
    pub assignee_ids: Vec<UserId>,
    pub assignee_logins: Vec<String>,  // Keep for display/search purposes
    // ... rest of fields ...
}
```

### 2. Update PullRequest Model

Similar changes as Issue:

```rust
pub struct PullRequest {
    // ... existing fields ...
    pub author_id: UserId,
    pub author_login: String,
    pub assignee_ids: Vec<UserId>,
    pub assignee_logins: Vec<String>,
    // ... rest of fields ...
}
```

### 3. Update Comment Models

```rust
pub struct IssueComment {
    // ... existing fields ...
    pub author_id: UserId,
    pub author_login: String,
    // ... rest of fields ...
}

pub struct PullRequestComment {
    // ... existing fields ...
    pub author_id: UserId,
    pub author_login: String,
    // ... rest of fields ...
}
```

## Implementation Strategy

### Phase 1: Add User Model (Non-breaking)

1. Create the User model with native_db annotations
2. Add UserId to ids.rs
3. Register the model in database.rs
4. Create database methods for user operations

### Phase 2: Extend GitHub Client

1. Extract full user information from octocrab responses
2. Create a user cache/lookup during sync operations
3. Implement `get_or_create_user` method

### Phase 3: Migration (Breaking changes)

1. Add new fields to existing models (author_id, assignee_ids)
2. Keep old string fields for backward compatibility
3. Update sync logic to populate both old and new fields
4. Update search logic to include user data

## Database Operations

New methods needed in GitDatabase:

```rust
impl GitDatabase {
    /// Get or create a user by GitHub login
    pub async fn get_or_create_user(&self, 
        github_id: i64, 
        login: &str,
        user_type: &str,
        site_admin: bool,
        avatar_url: Option<String>,
        html_url: Option<String>
    ) -> Result<User> {
        // Try to get by ID first (primary key lookup)
        // If not found, create new user
        // Handle unique constraint on login
    }
    
    /// Get user by login
    pub async fn get_user_by_login(&self, login: &str) -> Result<Option<User>> {
        // Secondary key lookup
    }
    
    /// Get multiple users by IDs
    pub async fn get_users_by_ids(&self, ids: &[UserId]) -> Result<Vec<User>> {
        // Batch lookup
    }
    
    /// Get all unique participants for a repository
    pub async fn get_repository_participants(&self, repo_id: &RepositoryId) -> Result<Vec<User>> {
        // Query all issues, PRs, and comments
        // Extract unique user IDs
        // Return user details
    }
}
```

## Search Index Updates

The tantivy search index should be updated to include:
- User IDs for filtering
- Separate fields for author names vs IDs
- Participant count as a searchable field

## Benefits

1. **Data Integrity**: Unique users across the system
2. **Performance**: Faster lookups by ID instead of string matching
3. **Rich User Data**: Store additional user metadata
4. **Relationships**: Proper foreign key relationships
5. **Analytics**: Easy to query for user statistics

## Migration Path

1. Deploy Phase 1 (non-breaking)
2. Run a migration script to populate User table from existing data
3. Deploy Phase 2 to start collecting user data during sync
4. Deploy Phase 3 with dual-write to both old and new fields
5. Eventually deprecate string-based author/assignee fields

## Considerations

- **Storage**: Additional table and indexes will increase storage requirements
- **Sync Performance**: Need to efficiently batch user lookups during sync
- **API Rate Limits**: May need additional API calls to fetch user details
- **Backward Compatibility**: Keep string fields initially for compatibility