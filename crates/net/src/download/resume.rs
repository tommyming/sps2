//! Resumable download logic for package downloads

use super::config::PackageDownloadConfig;
use sps2_errors::Error;
use std::path::Path;
use tokio::fs as tokio_fs;
use tokio::io::AsyncReadExt;

/// Get the offset for resuming a download
pub(super) async fn get_resume_offset(
    config: &PackageDownloadConfig,
    dest_path: &Path,
) -> Result<u64, Error> {
    match tokio_fs::metadata(dest_path).await {
        Ok(metadata) => {
            let size = metadata.len();
            if size >= config.min_chunk_size {
                Ok(size)
            } else {
                // File is too small to resume, start over
                let _ = tokio_fs::remove_file(dest_path).await;
                Ok(0)
            }
        }
        Err(_) => Ok(0), // File doesn't exist
    }
}

/// Calculate hash of existing file content for resume
pub(super) async fn calculate_existing_file_hash(
    config: &PackageDownloadConfig,
    dest_path: &Path,
    bytes: u64,
) -> Result<blake3::Hasher, Error> {
    let mut file = tokio_fs::File::open(dest_path).await?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0; config.buffer_size];
    let mut remaining = bytes;

    while remaining > 0 {
        let to_read =
            usize::try_from(std::cmp::min(buffer.len() as u64, remaining)).unwrap_or(buffer.len());
        let bytes_read = file.read(&mut buffer[..to_read]).await?;

        if bytes_read == 0 {
            break;
        }

        hasher.update(&buffer[..bytes_read]);
        remaining -= bytes_read as u64;
    }

    Ok(hasher)
}
