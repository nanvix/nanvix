// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::anyhow::Result;
use ::std::sync::{
    Mutex,
    MutexGuard,
    OnceLock,
    atomic::{
        AtomicBool,
        Ordering,
    },
};

//==================================================================================================
// Global Variables
//==================================================================================================

/// Tracks whether warnings should be treated as fatal errors.
static WARNINGS_FATAL_ENABLED: AtomicBool = AtomicBool::new(false);
/// Indicates whether a warning has been emitted since fatal mode was enabled.
static WARNING_TRIGGERED: AtomicBool = AtomicBool::new(false);
/// Stores the first warning message captured while fatal mode is active.
static WARNING_MESSAGE: OnceLock<Mutex<Option<String>>> = OnceLock::new();

//==================================================================================================
// Helper Functions
//==================================================================================================

///
/// # Description
///
/// Returns the mutex used to store the warning message, initializing it on first use.
///
/// # Return Value
///
/// Returns a reference to the lazily initialized mutex that stores the warning message.
///
fn warning_message_storage() -> &'static Mutex<Option<String>> {
    WARNING_MESSAGE.get_or_init(|| Mutex::new(None))
}

///
/// # Description
///
/// Acquires the warning message storage guard, tolerating poisoned mutex states.
///
/// # Parameters
///
/// - `storage`: Mutex that stores the optional warning message.
///
/// # Return Value
///
/// Returns a guard that grants mutable access to the stored warning message, even when the mutex
/// was previously poisoned.
///
fn lock_warning_message<'a>(storage: &'a Mutex<Option<String>>) -> MutexGuard<'a, Option<String>> {
    match storage.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Configures whether warnings should be promoted to fatal errors.
///
/// # Parameters
///
/// - `fatal_enabled`: Indicates whether warnings should cause the test run to fail.
///
pub(crate) fn configure(fatal_enabled: bool) {
    WARNINGS_FATAL_ENABLED.store(fatal_enabled, Ordering::Relaxed);
    if !fatal_enabled {
        WARNING_TRIGGERED.store(false, Ordering::Relaxed);
        if let Some(storage) = WARNING_MESSAGE.get() {
            let mut guard: MutexGuard<'_, Option<String>> = lock_warning_message(storage);
            *guard = None;
        }
    }
}

///
/// # Description
///
/// Records that a warning was emitted and caches the associated message for diagnostics.
///
/// # Parameters
///
/// - `message`: Fully formatted warning message.
///
pub(crate) fn record_warning(message: String) {
    if !WARNINGS_FATAL_ENABLED.load(Ordering::Relaxed) {
        return;
    }

    WARNING_TRIGGERED.store(true, Ordering::Relaxed);

    let storage: &Mutex<Option<String>> = warning_message_storage();
    let mut guard: MutexGuard<'_, Option<String>> = lock_warning_message(storage);
    if guard.is_none() {
        *guard = Some(message);
    }
}

///
/// # Description
///
/// Fails the current operation if a warning was recorded while fatal mode is enabled.
///
/// # Parameters
///
/// - `context`: Human-readable label that describes where the warning was detected.
///
/// # Return Value
///
/// Returns `Ok(())` when no warnings were captured with fatal mode enabled; otherwise returns an
/// error that surfaces the first warning message.
///
pub(crate) fn fail_if_triggered(context: &str) -> Result<()> {
    if !WARNINGS_FATAL_ENABLED.load(Ordering::Relaxed) {
        return Ok(());
    }

    if !WARNING_TRIGGERED.load(Ordering::Relaxed) {
        return Ok(());
    }

    WARNING_TRIGGERED.store(false, Ordering::Relaxed);
    let storage: &Mutex<Option<String>> = warning_message_storage();
    let mut guard: MutexGuard<'_, Option<String>> = lock_warning_message(storage);
    let warning_message: String = guard
        .take()
        .unwrap_or_else(|| "warning emitted while fatal mode active".to_string());

    let reason: String =
        format!("warning treated as fatal (context={}, message={})", context, warning_message);
    Err(::anyhow::anyhow!(reason))
}
