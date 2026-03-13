//! Execution mode registry — maps mode names to factories.

use std::sync::Arc;

use crate::config::NanvixRuntimeConfig;
use crate::execution::ExecutionMode;

/// Error returned when an unknown execution mode is requested.
#[derive(Debug, thiserror::Error)]
#[error("unknown execution mode: {0}")]
pub struct UnknownMode(String);

/// Create an execution mode by name.
///
/// For V1, only `"standalone"` is supported. Future modes (e.g., "hyperlight",
/// "distributed") will be added here.
///
/// The caller (proto layer) passes the mode name from `NanvixRuntimeConfig::execution_mode`.
pub fn create_execution_mode(
    mode: &str,
    id: &str,
    config: &NanvixRuntimeConfig,
) -> Result<Arc<dyn ExecutionMode>, UnknownMode> {
    match mode {
        // The standalone mode is registered here but lives in a separate crate.
        // We return a trait object so the proto layer is decoupled from concrete modes.
        //
        // NOTE: The actual StandaloneMode construction happens in the binary crate
        // which has access to nanvix-shim-standalone. This function serves as the
        // extensibility point — future modes register here.
        _ => Err(UnknownMode(mode.to_string())),
    }
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
