// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//!
//! # Logging
//!
//! This module provides logging functionalities.
//!

//==================================================================================================
// Imports
//==================================================================================================

use ::flexi_logger::Logger;
use ::std::sync::Once;

//==================================================================================================
// Public Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Initializes the logger.
///
/// # Note
///
/// If the logger cannot be initialized, the function will panic.
///
pub fn initialize() {
    static INIT_LOG: Once = Once::new();
    INIT_LOG.call_once(|| {
        Logger::try_with_env()
            .expect("malformed RUST_LOG environment variable")
            .start()
            .expect("failed to initialize logger");
    });
}
