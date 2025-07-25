//! Resource manager for coordinating concurrent operations
//!
//! This module provides the main `ResourceManager` struct that coordinates
//! semaphores and resource limits for concurrent operations.

use crate::limits::{ResourceAvailability, ResourceLimits};
use crate::semaphore::{acquire_semaphore_permit, create_semaphore, try_acquire_semaphore_permit};
use sps2_errors::Error;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Resource manager for coordinating resource usage
///
/// This structure manages semaphores and resource limits for concurrent
/// operations, ensuring we don't exceed system capabilities.
#[derive(Debug)]
pub struct ResourceManager {
    /// Semaphore for download operations
    pub download_semaphore: Arc<Semaphore>,
    /// Semaphore for decompression operations
    pub decompression_semaphore: Arc<Semaphore>,
    /// Semaphore for installation operations
    pub installation_semaphore: Arc<Semaphore>,
    /// Resource limits configuration
    pub limits: ResourceLimits,
    /// Current memory usage
    pub memory_usage: Arc<AtomicU64>,
}

impl ResourceManager {
    /// Create a new resource manager with the given limits
    #[must_use]
    pub fn new(limits: ResourceLimits) -> Self {
        Self {
            download_semaphore: create_semaphore(limits.concurrent_downloads),
            decompression_semaphore: create_semaphore(limits.concurrent_decompressions),
            installation_semaphore: create_semaphore(limits.concurrent_installations),
            memory_usage: Arc::new(AtomicU64::new(0)),
            limits,
        }
    }

    /// Create a resource manager with system-based limits
    #[must_use]
    pub fn from_system() -> Self {
        Self::new(ResourceLimits::from_system())
    }

    /// Acquire a download permit
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed or acquisition fails.
    pub async fn acquire_download_permit(&self) -> Result<OwnedSemaphorePermit, Error> {
        acquire_semaphore_permit(self.download_semaphore.clone(), "download").await
    }

    /// Acquire a decompression permit
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed or acquisition fails.
    pub async fn acquire_decompression_permit(&self) -> Result<OwnedSemaphorePermit, Error> {
        acquire_semaphore_permit(self.decompression_semaphore.clone(), "decompression").await
    }

    /// Acquire an installation permit
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed or acquisition fails.
    pub async fn acquire_installation_permit(&self) -> Result<OwnedSemaphorePermit, Error> {
        acquire_semaphore_permit(self.installation_semaphore.clone(), "installation").await
    }

    /// Try to acquire a download permit without blocking
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed.
    pub fn try_acquire_download_permit(&self) -> Result<Option<OwnedSemaphorePermit>, Error> {
        try_acquire_semaphore_permit(&self.download_semaphore)
    }

    /// Try to acquire a decompression permit without blocking
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed.
    pub fn try_acquire_decompression_permit(&self) -> Result<Option<OwnedSemaphorePermit>, Error> {
        try_acquire_semaphore_permit(&self.decompression_semaphore)
    }

    /// Try to acquire an installation permit without blocking
    ///
    /// # Errors
    ///
    /// Returns an error if the semaphore is closed.
    pub fn try_acquire_installation_permit(&self) -> Result<Option<OwnedSemaphorePermit>, Error> {
        try_acquire_semaphore_permit(&self.installation_semaphore)
    }

    /// Check if memory usage is within limits
    #[must_use]
    pub fn is_memory_within_limits(&self, current_usage: u64) -> bool {
        match self.limits.memory_usage {
            Some(limit) => current_usage <= limit,
            None => true, // No limit set
        }
    }

    /// Get current resource availability
    #[must_use]
    pub fn get_resource_availability(&self) -> ResourceAvailability {
        ResourceAvailability {
            download: self.download_semaphore.available_permits(),
            decompression: self.decompression_semaphore.available_permits(),
            installation: self.installation_semaphore.available_permits(),
        }
    }

    /// Clean up resources
    ///
    /// # Errors
    ///
    /// Returns an error if cleanup operations fail.
    pub fn cleanup(&self) -> Result<(), Error> {
        // Nothing to do here for now, but this can be used to clean up
        // any temporary files or other resources created by the resource manager.
        Ok(())
    }
}

impl Default for ResourceManager {
    fn default() -> Self {
        Self::new(ResourceLimits::default())
    }
}
