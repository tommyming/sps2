//! Index caching functionality

use crate::models::Index;
use sps2_errors::{Error, StorageError};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Index cache manager
#[derive(Clone, Debug)]
pub struct IndexCache {
    cache_dir: PathBuf,
}

impl IndexCache {
    /// Create a new cache manager
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
        }
    }

    /// Get the index cache file path
    fn index_path(&self) -> PathBuf {
        self.cache_dir.join("index.json")
    }

    /// Get the index metadata file path (for `ETag`, etc.)
    fn metadata_path(&self) -> PathBuf {
        self.cache_dir.join("index.meta")
    }

    /// Load index from cache
    ///
    /// # Errors
    ///
    /// Returns an error if the cache file doesn't exist or contains invalid data.
    pub async fn load(&self) -> Result<Index, Error> {
        let path = self.index_path();

        let content = fs::read_to_string(&path)
            .await
            .map_err(|_e| StorageError::PathNotFound {
                path: path.display().to_string(),
            })?;

        Index::from_json(&content)
    }

    /// Save index to cache
    ///
    /// # Errors
    ///
    /// Returns an error if the cache directory cannot be created or the file cannot be written.
    pub async fn save(&self, index: &Index) -> Result<(), Error> {
        // Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir)
            .await
            .map_err(|e| StorageError::IoError {
                message: format!("failed to create cache dir: {e}"),
            })?;

        let path = self.index_path();
        let json = index.to_json()?;

        // Write to temporary file first
        let temp_path = path.with_extension("tmp");
        fs::write(&temp_path, &json)
            .await
            .map_err(|e| StorageError::IoError {
                message: format!("failed to write cache: {e}"),
            })?;

        // Atomic rename
        fs::rename(&temp_path, &path)
            .await
            .map_err(|e| StorageError::IoError {
                message: format!("failed to rename cache file: {e}"),
            })?;

        Ok(())
    }

    /// Check if cache exists
    pub async fn exists(&self) -> bool {
        fs::metadata(self.index_path()).await.is_ok()
    }

    /// Get cache age in seconds
    ///
    /// # Errors
    ///
    /// Returns an error if file metadata cannot be read or timestamps are invalid.
    pub async fn age(&self) -> Result<Option<u64>, Error> {
        let path = self.index_path();

        match fs::metadata(&path).await {
            Ok(metadata) => {
                let modified = metadata.modified().map_err(|e| StorageError::IoError {
                    message: format!("failed to get modification time: {e}"),
                })?;

                let age = std::time::SystemTime::now()
                    .duration_since(modified)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);

                Ok(Some(age))
            }
            Err(_) => Ok(None),
        }
    }

    /// Clear the cache
    ///
    /// # Errors
    ///
    /// This function does not return errors as file removal failures are ignored.
    pub async fn clear(&self) -> Result<(), Error> {
        let _ = fs::remove_file(self.index_path()).await;
        let _ = fs::remove_file(self.metadata_path()).await;
        Ok(())
    }

    /// Load cached `ETag`
    ///
    /// # Errors
    ///
    /// Does not return errors - missing files return `None`.
    pub async fn load_etag(&self) -> Result<Option<String>, Error> {
        let path = self.metadata_path();

        match fs::read_to_string(&path).await {
            Ok(content) => {
                // Simple format: first line is ETag
                Ok(content.lines().next().map(String::from))
            }
            Err(_) => Ok(None),
        }
    }

    /// Save `ETag`
    ///
    /// # Errors
    ///
    /// Returns an error if the metadata file cannot be written.
    pub async fn save_etag(&self, etag: &str) -> Result<(), Error> {
        let path = self.metadata_path();

        fs::write(&path, etag)
            .await
            .map_err(|e| StorageError::IoError {
                message: format!("failed to save ETag: {e}"),
            })?;

        Ok(())
    }
}
