//! Builder pattern for StateVerificationGuard

use crate::core::guard::StateVerificationGuard;
use crate::types::{GuardConfig, VerificationLevel};
use sps2_errors::{Error, OpsError};
use sps2_events::EventSender;
use sps2_state::StateManager;
use sps2_store::PackageStore;

/// Builder for `StateVerificationGuard`
pub struct StateVerificationGuardBuilder {
    state_manager: Option<StateManager>,
    store: Option<PackageStore>,
    tx: Option<EventSender>,
    config: GuardConfig,
}

impl StateVerificationGuardBuilder {
    /// Create a new builder
    #[must_use]
    pub fn new() -> Self {
        Self {
            state_manager: None,
            store: None,
            tx: None,
            config: GuardConfig::default(),
        }
    }

    /// Set the state manager
    #[must_use]
    pub fn with_state_manager(mut self, state_manager: StateManager) -> Self {
        self.state_manager = Some(state_manager);
        self
    }

    /// Set the package store
    #[must_use]
    pub fn with_store(mut self, store: PackageStore) -> Self {
        self.store = Some(store);
        self
    }

    /// Set the event sender
    #[must_use]
    pub fn with_event_sender(mut self, tx: EventSender) -> Self {
        self.tx = Some(tx);
        self
    }

    /// Set the verification level
    #[must_use]
    pub fn with_level(mut self, level: VerificationLevel) -> Self {
        self.config.verification_level = level;
        self
    }

    /// Set the guard configuration
    #[must_use]
    pub fn with_config(mut self, config: GuardConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the guard
    ///
    /// # Errors
    ///
    /// Returns an error if any required component is missing.
    pub fn build(self) -> Result<StateVerificationGuard, Error> {
        let state_manager = self
            .state_manager
            .ok_or_else(|| OpsError::MissingComponent {
                component: "StateManager".to_string(),
            })?;

        let store = self.store.ok_or_else(|| OpsError::MissingComponent {
            component: "PackageStore".to_string(),
        })?;

        let tx = self.tx.ok_or_else(|| OpsError::MissingComponent {
            component: "EventSender".to_string(),
        })?;

        Ok(StateVerificationGuard::new(
            state_manager,
            store,
            tx,
            self.config,
        ))
    }
}

impl Default for StateVerificationGuardBuilder {
    fn default() -> Self {
        Self::new()
    }
}
