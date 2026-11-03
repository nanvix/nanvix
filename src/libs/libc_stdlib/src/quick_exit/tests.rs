// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    at_quick_exit,
    AT_QUICK_EXIT_COUNT,
    AT_QUICK_EXIT_HANDLERS,
    MAX_AT_QUICK_EXIT_HANDLERS,
};

unsafe extern "C" fn dummy_handler() {}

/// Resets the quick-exit handler table. Used for test isolation.
unsafe fn reset_at_quick_exit_handlers() {
    AT_QUICK_EXIT_COUNT = 0;
    let mut i: usize = 0;
    while i < MAX_AT_QUICK_EXIT_HANDLERS {
        AT_QUICK_EXIT_HANDLERS[i] = None;
        i += 1;
    }
}

#[test]
fn register_handler() {
    unsafe { reset_at_quick_exit_handlers() };
    assert_eq!(unsafe { at_quick_exit(Some(dummy_handler)) }, 0);
    unsafe { reset_at_quick_exit_handlers() };
}
