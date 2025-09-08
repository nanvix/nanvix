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
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the logger.
///
/// # Parameters
///
/// - `log_to_file`: Log to file?
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
#[allow(clippy::expect_used)]
pub fn initialize(log_to_file: bool) {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        let logger =
            Logger::try_with_env_or_str("error").expect("malformed RUST_LOG environment variable");
        if log_to_file {
            logger
                .log_to_file(FileSpec::default())
                .start()
                .expect("failed to initialize logger");
        } else {
            logger.start().expect("failed to initialize logger");
        }
    });
}
