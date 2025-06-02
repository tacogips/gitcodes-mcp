use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct StoragePaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl StoragePaths {
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

    pub fn database_path(&self) -> PathBuf {
        self.data_dir.join("gitdb.lance")
    }

    pub fn repositories_path(&self) -> PathBuf {
        self.data_dir.join("repositories.json")
    }

    pub fn embeddings_dir(&self) -> PathBuf {
        self.data_dir.join("embeddings")
    }

    pub fn sync_cache_dir(&self) -> PathBuf {
        self.data_dir.join("sync_cache")
    }

    pub fn config_file_path(&self) -> PathBuf {
        self.config_dir.join("config.json")
    }

    pub fn cross_references_db_path(&self) -> PathBuf {
        self.data_dir.join("cross_references.db")
    }
}
