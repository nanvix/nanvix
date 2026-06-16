// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Execution mode registry — maps mode names to factories.

use std::sync::Arc;

use crate::{
    config::NanvixRuntimeConfig,
    execution::ExecutionMode,
};

/// Error returned when an unknown execution mode is requested.
#[derive(Debug, thiserror::Error)]
#[error("unknown execution mode: {0}")]
pub struct UnknownMode(String);

/// Create an execution mode by name.
///
/// This function does not provide any built-in execution modes.
/// Callers should use [`ModeRegistry`] to register and create modes such as
/// `"standalone"`.
///
/// The caller (proto layer) passes the mode name from
/// `NanvixRuntimeConfig::execution_mode`.
pub fn create_execution_mode(
    mode: &str,
    _id: &str,
    _config: &NanvixRuntimeConfig,
) -> Result<Arc<dyn ExecutionMode>, UnknownMode> {
    // No built-in modes; use ModeRegistry instead.
    Err(UnknownMode(mode.to_string()))
}

/// Type alias for a factory function that creates an ExecutionMode.
pub type ExecutionModeFactory =
    fn(id: &str, config: &NanvixRuntimeConfig) -> Arc<dyn ExecutionMode>;

/// Registry that maps mode names to factory functions.
/// Used by the binary crate to register concrete modes before starting the shim.
pub struct ModeRegistry {
    entries: Vec<(&'static str, ExecutionModeFactory)>,
}

impl ModeRegistry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Register a new execution mode factory.
    pub fn register(&mut self, name: &'static str, factory: ExecutionModeFactory) {
        self.entries.push((name, factory));
    }

    /// Create an execution mode by name.
    pub fn create(
        &self,
        mode: &str,
        id: &str,
        config: &NanvixRuntimeConfig,
    ) -> Result<Arc<dyn ExecutionMode>, UnknownMode> {
        for (name, factory) in &self.entries {
            if *name == mode {
                return Ok(factory(id, config));
            }
        }
        Err(UnknownMode(mode.to_string()))
    }
}

impl Default for ModeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
