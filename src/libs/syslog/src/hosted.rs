// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::flexi_logger::{
    FileSpec,
    Logger,
};
use ::std::sync::Once;

//==================================================================================================
// Re-Exports
//==================================================================================================

pub use ::log::{
    debug,
    error,
    info,
    trace,
    warn,
};

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the logger.
///
/// # Parameters
///
/// - `log_to_file`: Log to file?
/// - `default_level`: Default log level (overridden by RUST_LOG environment variable if set).
/// - `log_dir`: Directory to write log files to (if `log_to_file` is true).
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn init(log_to_file: bool, default_level: &str, log_dir: String) {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        let logger = Logger::try_with_env_or_str(default_level)
            .expect("malformed RUST_LOG environment variable");
        if log_to_file {
            logger
                .log_to_file(FileSpec::default().directory(log_dir))
                .start()
                .expect("failed to initialize logger");
        } else {
            logger.start().expect("failed to initialize logger");
        }
    });
}
