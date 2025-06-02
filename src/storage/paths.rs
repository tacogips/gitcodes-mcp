use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct StoragePaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl StoragePaths {
    /// Creates a new StoragePaths instance with appropriate platform-specific paths.
    ///
    /// # Returns
    ///
    /// Returns a Result containing the StoragePaths instance with configured directories.
    ///
    /// # Environment Variables
    ///
    /// - `GITDB_DATA_DIR` - Override the default data directory location
    /// - `GITDB_CONFIG_DIR` - Override the default config directory location
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Platform-specific directories cannot be determined
    /// - Directory creation fails
    pub fn new() -> Result<Self> {
        // Check for environment variable override (useful for testing)
        let data_dir = if let Ok(override_path) = std::env::var("GITDB_DATA_DIR") {
            PathBuf::from(override_path)
        } else {
            dirs::data_dir()
                .context("Failed to get data directory")?
                .join("gitdb")
        };

        let config_dir = if let Ok(override_path) = std::env::var("GITDB_CONFIG_DIR") {
            PathBuf::from(override_path)
        } else {
            dirs::config_dir()
                .context("Failed to get config directory")?
                .join("gitdb")
        };

        // Create directories if they don't exist
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&data_dir)?;

        Ok(Self {
            config_dir,
            data_dir,
        })
    }

    /// Returns the path to the Lance database file.
    ///
    /// # Returns
    ///
    /// PathBuf pointing to `{data_dir}/gitdb.lance`
    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("gitdb.lance")
    }

    /// Returns the path to the repositories metadata JSON file.
    ///
    /// # Returns
    ///
    /// PathBuf pointing to `{data_dir}/repositories.json`
    pub fn repositories_path(&self) -> PathBuf {
        self.data_dir.join("repositories.json")
    }

    /// Returns the path to the embeddings directory.
    ///
    /// # Returns
    ///
    /// PathBuf pointing to `{data_dir}/embeddings`
    pub fn embeddings_dir(&self) -> PathBuf {
        self.data_dir.join("embeddings")
    }

    /// Returns the path to the sync cache directory.
    ///
    /// # Returns
    ///
    /// PathBuf pointing to `{data_dir}/sync_cache`
    pub fn sync_cache_dir(&self) -> PathBuf {
        self.data_dir.join("sync_cache")
    }

    /// Returns the path to the configuration file.
    ///
    /// # Returns
    ///
    /// PathBuf pointing to `{config_dir}/config.json`
    pub fn config_file_path(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    /// Returns the path to the cross-references SQLite database.
    ///
    /// # Returns
    ///
    /// PathBuf pointing to `{data_dir}/cross_references.db`
    pub fn cross_references_db_path(&self) -> PathBuf {
        self.data_dir.join("cross_references.db")
    }
}
