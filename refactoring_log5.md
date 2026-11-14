# Refactoring Log 5: Global RepositoryManager Singleton Implementation

## Overview

Implemented a global singleton pattern for the `RepositoryManager` using `once_cell` to ensure that a single instance with a consistent process ID is maintained throughout the application lifetime. This prevents resource conflicts when multiple repositories are cloned in the same process.

## Changes Made

### 1. Added `process_id` to `RepositoryManager`

- Added a unique process-specific identifier to `RepositoryManager` that differentiates it from other processes
- Implemented a `generate_process_id()` method that combines the process ID and a random UUID
- Updated constructor methods to generate and set this process ID

```rust
#[derive(Clone)]
pub struct RepositoryManager {
    pub github_token: Option<String>,
    pub local_repository_cache_dir_base: PathBuf,
    /// Unique identifier for this repository manager instance
    pub process_id: String,
}

impl RepositoryManager {
    fn generate_process_id() -> String {
        use std::process;
        use uuid::Uuid;
        
        let pid = process::id();
        let uuid = Uuid::new_v4();
        
        format!("{}_{}", pid, uuid.simple())
    }
    
    // Constructor updated to set process_id
    pub fn new(
        github_token: Option<String>,
        local_repository_cache_dir_base: Option<PathBuf>,
    ) -> Result<Self, String> {
        // ... existing code ...
        
        // Generate a unique process ID for this repository manager instance
        let process_id = Self::generate_process_id();

        Ok(Self {
            github_token,
            local_repository_cache_dir_base,
            process_id,
        })
    }
}
```

### 2. Modified `LocalRepository` to Use Process ID

- Updated `new_local_repository_to_clone` to accept an optional process ID parameter
- Modified directory naming to include the process ID when provided

```rust
pub fn new_local_repository_to_clone(
    remote_repository_info: GitRemoteRepositoryInfo, 
    process_id: Option<&str>
) -> Self {
    let hash_value = Self::generate_repository_hash(&remote_repository_info);
    
    // Include process_id in directory name if provided
    let dir_name = if let Some(pid) = process_id {
        format!(
            "mcp_gitcodes_{}_{}_{}_pid{}",
            remote_repository_info.user, remote_repository_info.repo, hash_value, pid
        )
    } else {
        format!(
            "mcp_gitcodes_{}_{}_{}" ,
            remote_repository_info.user, remote_repository_info.repo, hash_value
        )
    };

    let mut repo_dir = std::env::temp_dir();
    repo_dir.push(dir_name);

    Self::new(repo_dir)
}
```

### 3. Implemented Global Singleton with `once_cell`

- Created a new module `repository_manager/instance.rs` for the global singleton
- Used `once_cell::sync::OnceCell` to implement the singleton pattern
- Added initialization and access functions

```rust
use once_cell::sync::OnceCell;
use std::path::PathBuf;

use super::RepositoryManager;

/// Global RepositoryManager instance
static GLOBAL_REPOSITORY_MANAGER: OnceCell<RepositoryManager> = OnceCell::new();

/// Initialize the global RepositoryManager instance with the given parameters
pub fn init_repository_manager(
    github_token: Option<String>,
    repository_cache_dir: Option<PathBuf>,
) -> &'static RepositoryManager {
    GLOBAL_REPOSITORY_MANAGER.get_or_init(move || {
        RepositoryManager::new(github_token, repository_cache_dir)
            .expect("Failed to initialize global repository manager")
    })
}

/// Get the global RepositoryManager instance
pub fn get_repository_manager() -> &'static RepositoryManager {
    GLOBAL_REPOSITORY_MANAGER
        .get_or_init(|| RepositoryManager::with_default_cache_dir())
}
```

### 4. Updated Application Startup Code

- Modified both `run_stdio_server` and `run_http_server` to initialize the global manager at startup
- This ensures that the same process ID is used for all subsequent operations

```rust
async fn run_stdio_server(
    debug: bool, 
    github_token: Option<String>,
    repository_cache_dir: Option<std::path::PathBuf>
) -> Result<()> {
    // Initialize the global repository manager at startup
    // This ensures a single process_id is used throughout the application lifetime
    let _ = gitcodes_mcp::gitcodes::repository_manager::instance::init_repository_manager(
        github_token.clone(), 
        repository_cache_dir.clone()
    );
    
    // ... rest of the function ...
}
```

### 5. Updated `GitHubCodeTools` to Use Global Instance

- Changed constructor methods to use the global repository manager
- Updated `with_service` method to always return tools with the global instance

```rust
pub fn new(github_token: Option<String>, repository_cache_dir: Option<PathBuf>) -> Self {
    // Initialize the global repository manager with these parameters
    // This will only have an effect the first time it's called
    let manager = repository_manager::instance::init_repository_manager(github_token, repository_cache_dir);
    
    Self {
        manager: manager.clone(),
    }
}

pub fn with_service(_manager: RepositoryManager) -> Self {
    // Get the global repository manager
    let manager = repository_manager::instance::get_repository_manager();
    Self { manager: manager.clone() }
}
```

### 6. Added Dependencies

- Added `once_cell` version 1.18 to Cargo.toml
- Added `uuid` version 1.4 with the "v4" feature for UUID generation

## Benefits

1. **Consistent Process Identification**
   - A single process ID is maintained throughout the application lifetime
   - All repository cloning operations use the same ID, preventing conflicts

2. **Resource Efficiency**
   - Avoids creating multiple RepositoryManager instances with different IDs
   - Reduces overhead from multiple initialization operations

3. **Process Isolation**
   - Each process still has its own unique identifier
   - Multiple processes running simultaneously won't conflict with each other

## Documentation Updates

- Added detailed explanations in lib.rs about the singleton pattern
- Updated devlog.md with a new section on the Global Singleton Pattern

## Implementation Strategy

The implementation follows a classic singleton pattern with lazy initialization:

1. A static `OnceCell` holds the singleton RepositoryManager instance
2. At application startup, the global instance is initialized with the provided parameters
3. All subsequent requests for a RepositoryManager use the existing instance
4. If a request comes in before initialization, a default instance is created

This ensures that:
- The process_id remains constant throughout the application
- Parameters like github_token and repository_cache_dir are set correctly
- Multiple components can safely share the same RepositoryManager instance