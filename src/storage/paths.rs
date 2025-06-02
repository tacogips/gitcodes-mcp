use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct StoragePaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl StoragePaths {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .context("Failed to get config directory")?
            .join("gitdb");
        
        let data_dir = dirs::data_dir()
            .context("Failed to get data directory")?
            .join("gitdb");
        
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