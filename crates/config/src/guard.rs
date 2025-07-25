//! Guard configuration for verification and integrity checking

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Symlink handling policy for guard operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SymlinkPolicyConfig {
    /// Verify symlinks strictly - fail on any symlink issues
    Strict,
    /// Be lenient with bootstrap directories like /opt/pm/live/bin
    LenientBootstrap,
    /// Be lenient with all symlinks - log issues but don't fail
    LenientAll,
    /// Ignore symlinks entirely - skip symlink verification
    Ignore,
}

impl Default for SymlinkPolicyConfig {
    fn default() -> Self {
        Self::LenientBootstrap
    }
}

/// How to handle discrepancies found during verification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscrepancyHandling {
    /// Fail the operation when discrepancies are found
    FailFast,
    /// Report discrepancies but continue operation
    ReportOnly,
    /// Automatically heal discrepancies when possible
    AutoHeal,
    /// Auto-heal but fail if healing is not possible
    AutoHealOrFail,
}

impl Default for DiscrepancyHandling {
    fn default() -> Self {
        Self::FailFast
    }
}

/// Policy for handling user files
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserFilePolicy {
    /// Preserve user-created files
    Preserve,
    /// Remove user-created files
    Remove,
    /// Backup user-created files before removal
    Backup,
}

impl Default for UserFilePolicy {
    fn default() -> Self {
        Self::Preserve
    }
}

/// Performance configuration for guard operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfigToml {
    #[serde(default = "default_progressive_verification")]
    pub progressive_verification: bool,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
    #[serde(default = "default_verification_timeout_seconds")]
    pub verification_timeout_seconds: u64,
}

impl Default for PerformanceConfigToml {
    fn default() -> Self {
        Self {
            progressive_verification: default_progressive_verification(),
            max_concurrent_tasks: default_max_concurrent_tasks(),
            verification_timeout_seconds: default_verification_timeout_seconds(),
        }
    }
}

/// Guard-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfigToml {
    #[serde(default)]
    pub symlink_policy: SymlinkPolicyConfig,
    #[serde(default = "default_lenient_symlink_directories")]
    pub lenient_symlink_directories: Vec<PathBuf>,
}

impl Default for GuardConfigToml {
    fn default() -> Self {
        Self {
            symlink_policy: SymlinkPolicyConfig::default(),
            lenient_symlink_directories: default_lenient_symlink_directories(),
        }
    }
}

/// Verification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_verification_level")]
    pub level: String, // "quick", "standard", or "full"
    #[serde(default)]
    pub discrepancy_handling: DiscrepancyHandling,
    #[serde(default = "default_orphaned_file_action")]
    pub orphaned_file_action: String, // "remove", "preserve", or "backup"
    #[serde(default = "default_orphaned_backup_dir")]
    pub orphaned_backup_dir: PathBuf,
    #[serde(default)]
    pub user_file_policy: UserFilePolicy,

    // Enhanced guard configuration
    #[serde(default)]
    pub guard: GuardConfigToml,
    #[serde(default)]
    pub performance: PerformanceConfigToml,

    // Legacy compatibility fields - deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_discrepancy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_heal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_user_files: Option<bool>,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            enabled: false, // Disabled by default during development
            level: "standard".to_string(),
            discrepancy_handling: DiscrepancyHandling::default(),
            orphaned_file_action: "preserve".to_string(),
            orphaned_backup_dir: PathBuf::from("/opt/pm/orphaned-backup"),
            user_file_policy: UserFilePolicy::default(),
            guard: GuardConfigToml::default(),
            performance: PerformanceConfigToml::default(),
            fail_on_discrepancy: None,
            auto_heal: None,
            preserve_user_files: None,
        }
    }
}

/// Simplified symlink policy for top-level guard configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardSymlinkPolicy {
    /// Verify symlinks strictly - fail on any symlink issues
    Strict,
    /// Be lenient with symlinks - log issues but don't fail
    Lenient,
    /// Ignore symlinks entirely - skip symlink verification
    Ignore,
}

impl Default for GuardSymlinkPolicy {
    fn default() -> Self {
        Self::Lenient
    }
}

/// Performance configuration for top-level guard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardPerformanceConfig {
    #[serde(default = "default_progressive_verification")]
    pub progressive_verification: bool,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
    #[serde(default = "default_verification_timeout_seconds")]
    pub verification_timeout_seconds: u64,
}

impl Default for GuardPerformanceConfig {
    fn default() -> Self {
        Self {
            progressive_verification: default_progressive_verification(),
            max_concurrent_tasks: default_max_concurrent_tasks(),
            verification_timeout_seconds: default_verification_timeout_seconds(),
        }
    }
}

/// Directory configuration for lenient symlink handling (array of tables approach)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardDirectoryConfig {
    pub path: PathBuf,
}

/// Top-level guard configuration (alternative to verification.guard approach)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardConfiguration {
    #[serde(default = "default_guard_enabled")]
    pub enabled: bool,
    #[serde(default = "default_verification_level")]
    pub verification_level: String, // "quick", "standard", or "full"
    #[serde(default)]
    pub discrepancy_handling: DiscrepancyHandling,
    #[serde(default)]
    pub symlink_policy: GuardSymlinkPolicy,
    #[serde(default = "default_orphaned_file_action")]
    pub orphaned_file_action: String, // "remove", "preserve", or "backup"
    #[serde(default = "default_orphaned_backup_dir")]
    pub orphaned_backup_dir: PathBuf,
    #[serde(default)]
    pub user_file_policy: UserFilePolicy,

    // Nested configuration sections
    #[serde(default)]
    pub performance: GuardPerformanceConfig,
    #[serde(default = "default_guard_lenient_symlink_directories")]
    pub lenient_symlink_directories: Vec<GuardDirectoryConfig>,

    // Legacy compatibility fields - deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_heal: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fail_on_discrepancy: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preserve_user_files: Option<bool>,
}

impl Default for GuardConfiguration {
    fn default() -> Self {
        Self {
            enabled: default_guard_enabled(),
            verification_level: default_verification_level(),
            discrepancy_handling: DiscrepancyHandling::default(),
            symlink_policy: GuardSymlinkPolicy::default(),
            orphaned_file_action: default_orphaned_file_action(),
            orphaned_backup_dir: default_orphaned_backup_dir(),
            user_file_policy: UserFilePolicy::default(),
            performance: GuardPerformanceConfig::default(),
            lenient_symlink_directories: default_guard_lenient_symlink_directories(),
            auto_heal: None,
            fail_on_discrepancy: None,
            preserve_user_files: None,
        }
    }
}

impl VerificationConfig {
    /// Apply legacy fields if present and convert to new enum-based fields
    pub fn apply_legacy_fields(&mut self) {
        // Handle legacy fail_on_discrepancy and auto_heal
        match (self.fail_on_discrepancy, self.auto_heal) {
            (Some(true), Some(true)) => {
                self.discrepancy_handling = DiscrepancyHandling::AutoHealOrFail;
            }
            (Some(false), Some(true)) => self.discrepancy_handling = DiscrepancyHandling::AutoHeal,
            (Some(true), Some(false)) => self.discrepancy_handling = DiscrepancyHandling::FailFast,
            (Some(false), Some(false)) => {
                self.discrepancy_handling = DiscrepancyHandling::ReportOnly;
            }
            _ => {} // Keep current discrepancy_handling
        }

        // Handle legacy preserve_user_files
        if let Some(preserve) = self.preserve_user_files {
            self.user_file_policy = if preserve {
                UserFilePolicy::Preserve
            } else {
                UserFilePolicy::Remove
            };
        }

        // Clear legacy fields after conversion
        self.fail_on_discrepancy = None;
        self.auto_heal = None;
        self.preserve_user_files = None;
    }

    /// Check if should fail on discrepancy (for backward compatibility)
    #[must_use]
    pub fn should_fail_on_discrepancy(&self) -> bool {
        matches!(
            self.discrepancy_handling,
            DiscrepancyHandling::FailFast | DiscrepancyHandling::AutoHealOrFail
        )
    }

    /// Check if should auto-heal (for backward compatibility)
    #[must_use]
    pub fn should_auto_heal(&self) -> bool {
        matches!(
            self.discrepancy_handling,
            DiscrepancyHandling::AutoHeal | DiscrepancyHandling::AutoHealOrFail
        )
    }
}

impl GuardConfiguration {
    /// Apply legacy fields if present and convert to new enum-based fields
    pub fn apply_legacy_fields(&mut self) {
        // Handle legacy fail_on_discrepancy and auto_heal
        match (self.fail_on_discrepancy, self.auto_heal) {
            (Some(true), Some(true)) => {
                self.discrepancy_handling = DiscrepancyHandling::AutoHealOrFail;
            }
            (Some(false), Some(true)) => self.discrepancy_handling = DiscrepancyHandling::AutoHeal,
            (Some(true), Some(false)) => self.discrepancy_handling = DiscrepancyHandling::FailFast,
            (Some(false), Some(false)) => {
                self.discrepancy_handling = DiscrepancyHandling::ReportOnly;
            }
            _ => {} // Keep current discrepancy_handling
        }

        // Handle legacy preserve_user_files
        if let Some(preserve) = self.preserve_user_files {
            self.user_file_policy = if preserve {
                UserFilePolicy::Preserve
            } else {
                UserFilePolicy::Remove
            };
        }

        // Clear legacy fields after conversion
        self.fail_on_discrepancy = None;
        self.auto_heal = None;
        self.preserve_user_files = None;
    }

    /// Check if should fail on discrepancy (for backward compatibility)
    #[must_use]
    pub fn should_fail_on_discrepancy(&self) -> bool {
        matches!(
            self.discrepancy_handling,
            DiscrepancyHandling::FailFast | DiscrepancyHandling::AutoHealOrFail
        )
    }

    /// Check if should auto-heal (for backward compatibility)
    #[must_use]
    pub fn should_auto_heal(&self) -> bool {
        matches!(
            self.discrepancy_handling,
            DiscrepancyHandling::AutoHeal | DiscrepancyHandling::AutoHealOrFail
        )
    }
}

// Default value functions for serde
fn default_progressive_verification() -> bool {
    true
}

fn default_max_concurrent_tasks() -> usize {
    8
}

fn default_verification_timeout_seconds() -> u64 {
    300 // 5 minutes
}

fn default_guard_enabled() -> bool {
    true // Enable guard by default for state verification
}

fn default_guard_lenient_symlink_directories() -> Vec<GuardDirectoryConfig> {
    vec![
        GuardDirectoryConfig {
            path: PathBuf::from("/opt/pm/live/bin"),
        },
        GuardDirectoryConfig {
            path: PathBuf::from("/opt/pm/live/sbin"),
        },
    ]
}

fn default_lenient_symlink_directories() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/opt/pm/live/bin"),
        PathBuf::from("/opt/pm/live/sbin"),
    ]
}

fn default_enabled() -> bool {
    true // Enable verification by default for state integrity
}

fn default_verification_level() -> String {
    "standard".to_string()
}

fn default_orphaned_file_action() -> String {
    "preserve".to_string()
}

fn default_orphaned_backup_dir() -> PathBuf {
    PathBuf::from("/opt/pm/orphaned-backup")
}
