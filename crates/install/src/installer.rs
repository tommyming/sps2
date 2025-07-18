//! Main installer implementation

use crate::{
    AtomicInstaller, InstallContext, InstallOperation, InstallResult, UninstallContext,
    UninstallOperation, UpdateContext, UpdateOperation,
};
use sps2_errors::{Error, InstallError};
// EventSender not used directly in this module but imported for potential future use
use sps2_resolver::Resolver;
use sps2_state::StateManager;
use sps2_store::PackageStore;
use uuid::Uuid;

/// Installer configuration
#[derive(Clone, Debug)]
pub struct InstallConfig {
    /// Maximum concurrent downloads
    pub max_concurrency: usize,
    /// Download timeout in seconds
    pub download_timeout: u64,
    /// Enable APFS optimizations
    pub enable_apfs: bool,
    /// State retention policy (number of states to keep)
    pub state_retention: usize,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            download_timeout: 300, // 5 minutes
            enable_apfs: cfg!(target_os = "macos"),
            state_retention: 10,
        }
    }
}

impl InstallConfig {
    /// Create config with custom concurrency
    #[must_use]
    pub fn with_concurrency(mut self, max_concurrency: usize) -> Self {
        self.max_concurrency = max_concurrency;
        self
    }

    /// Set download timeout
    #[must_use]
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.download_timeout = timeout_seconds;
        self
    }

    /// Enable/disable APFS optimizations
    #[must_use]
    pub fn with_apfs(mut self, enable: bool) -> Self {
        self.enable_apfs = enable;
        self
    }

    /// Set state retention policy
    #[must_use]
    pub fn with_retention(mut self, count: usize) -> Self {
        self.state_retention = count;
        self
    }
}

/// Main installer for sps2 packages
#[derive(Clone)]
pub struct Installer {
    /// Configuration
    config: InstallConfig,
    /// Dependency resolver
    resolver: Resolver,
    /// State manager
    state_manager: StateManager,
    /// Package store
    store: PackageStore,
}

impl std::fmt::Debug for Installer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Installer")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl Installer {
    /// Create new installer
    #[must_use]
    pub fn new(
        config: InstallConfig,
        resolver: Resolver,
        state_manager: StateManager,
        store: PackageStore,
    ) -> Self {
        Self {
            config,
            resolver,
            state_manager,
            store,
        }
    }

    /// Install packages
    ///
    /// # Errors
    ///
    /// Returns an error if package resolution fails, download fails, or installation fails.
    pub async fn install(&mut self, context: InstallContext) -> Result<InstallResult, Error> {
        // Validate context
        Self::validate_install_context(&context)?;

        // Create install operation
        let mut operation = InstallOperation::new(
            self.resolver.clone(),
            self.state_manager.clone(),
            self.store.clone(),
        )?;

        // Execute installation
        let result = operation.execute(context).await?;

        // Trigger garbage collection
        self.cleanup_old_states().await?;

        Ok(result)
    }

    /// Uninstall packages
    ///
    /// # Errors
    ///
    /// Returns an error if package validation fails or uninstall operation fails.
    pub async fn uninstall(&mut self, context: UninstallContext) -> Result<InstallResult, Error> {
        // Validate context
        Self::validate_uninstall_context(&context)?;

        // Create uninstall operation
        let mut operation = UninstallOperation::new(self.state_manager.clone(), self.store.clone());

        // Execute uninstallation
        let result = operation.execute(context).await?;

        // Trigger garbage collection
        self.cleanup_old_states().await?;

        Ok(result)
    }

    /// Update packages
    ///
    /// # Errors
    ///
    /// Returns an error if package resolution fails, download fails, or update fails.
    pub async fn update(&mut self, context: UpdateContext) -> Result<InstallResult, Error> {
        // Validate context
        Self::validate_update_context(&context);

        // Create update operation
        let mut operation = UpdateOperation::new(
            self.resolver.clone(),
            self.state_manager.clone(),
            self.store.clone(),
        )?;

        // Execute update
        let result = operation.execute(context).await?;

        // Trigger garbage collection
        self.cleanup_old_states().await?;

        Ok(result)
    }

    /// Rollback to a previous state
    ///
    /// # Errors
    ///
    /// Returns an error if the target state doesn't exist or rollback operation fails.
    pub async fn rollback(&mut self, target_state_id: Uuid) -> Result<(), Error> {
        // Validate target state exists
        if !self.state_manager.state_exists(&target_state_id).await? {
            return Err(InstallError::StateNotFound {
                state_id: target_state_id.to_string(),
            }
            .into());
        }

        // Create atomic installer for rollback
        let mut atomic_installer =
            AtomicInstaller::new(self.state_manager.clone(), self.store.clone()).await?;

        // Perform rollback
        atomic_installer.rollback(target_state_id).await?;

        Ok(())
    }

    /// List available states for rollback
    ///
    /// # Errors
    ///
    /// Returns an error if querying the state database fails.
    pub async fn list_states(&self) -> Result<Vec<StateInfo>, Error> {
        let states = self.state_manager.list_states_detailed().await?;

        let mut state_infos = Vec::new();
        for state in states {
            let packages = self
                .state_manager
                .get_state_packages(&state.state_id())
                .await?;

            // Parse parent_id if present
            let parent_id = state
                .parent_id
                .as_ref()
                .and_then(|id| uuid::Uuid::parse_str(id).ok());

            state_infos.push(StateInfo {
                id: state.state_id(),
                timestamp: state.timestamp(),
                parent_id,
                package_count: packages.len(),
                packages: packages
                    .into_iter()
                    .take(5)
                    .map(|name| sps2_types::PackageId::new(name, sps2_types::Version::new(1, 0, 0)))
                    .collect(), // First 5 packages as sample
            });
        }

        Ok(state_infos)
    }

    /// Get current state information
    ///
    /// # Errors
    ///
    /// Returns an error if the current state cannot be found or accessed.
    pub async fn current_state(&self) -> Result<StateInfo, Error> {
        let current_id = self.state_manager.get_current_state_id().await?;

        let states = self.list_states().await?;
        states
            .into_iter()
            .find(|state| state.id == current_id)
            .ok_or_else(|| {
                InstallError::StateNotFound {
                    state_id: current_id.to_string(),
                }
                .into()
            })
    }

    /// Cleanup old states according to retention policy
    async fn cleanup_old_states(&self) -> Result<(), Error> {
        self.state_manager
            .cleanup_old_states(self.config.state_retention)
            .await?;
        self.store.garbage_collect()?;
        Ok(())
    }

    /// Validate install context
    fn validate_install_context(context: &InstallContext) -> Result<(), Error> {
        if context.packages.is_empty() && context.local_files.is_empty() {
            return Err(InstallError::NoPackagesSpecified.into());
        }

        // Validate local file paths exist
        for path in &context.local_files {
            if !path.exists() {
                return Err(InstallError::LocalPackageNotFound {
                    path: path.display().to_string(),
                }
                .into());
            }

            if path.extension().is_none_or(|ext| ext != "sp") {
                return Err(InstallError::InvalidPackageFile {
                    path: path.display().to_string(),
                    message: "file must have .sp extension".to_string(),
                }
                .into());
            }
        }

        Ok(())
    }

    /// Validate uninstall context
    fn validate_uninstall_context(context: &UninstallContext) -> Result<(), Error> {
        if context.packages.is_empty() {
            return Err(InstallError::NoPackagesSpecified.into());
        }

        Ok(())
    }

    /// Validate update context
    fn validate_update_context(_context: &UpdateContext) {
        // Update context is always valid (empty packages means update all)
    }
}

/// State information for listing
#[derive(Debug, Clone)]
pub struct StateInfo {
    /// State ID
    pub id: Uuid,
    /// Creation timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Parent state ID
    pub parent_id: Option<Uuid>,
    /// Number of packages in this state
    pub package_count: usize,
    /// Sample of packages (for display)
    pub packages: Vec<sps2_types::PackageId>,
}

impl StateInfo {
    /// Check if this is the root state
    #[must_use]
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Get age of this state
    #[must_use]
    pub fn age(&self) -> chrono::Duration {
        chrono::Utc::now() - self.timestamp
    }

    /// Format package list for display
    #[must_use]
    pub fn package_summary(&self) -> String {
        if self.packages.is_empty() {
            "No packages".to_string()
        } else if self.packages.len() <= 3 {
            self.packages
                .iter()
                .map(|pkg| format!("{}-{}", pkg.name, pkg.version))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            let first_three: Vec<String> = self
                .packages
                .iter()
                .take(3)
                .map(|pkg| format!("{}-{}", pkg.name, pkg.version))
                .collect();
            format!(
                "{} and {} more",
                first_three.join(", "),
                self.package_count - 3
            )
        }
    }
}
