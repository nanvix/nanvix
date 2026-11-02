// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    at_quick_exit,
    reset_at_quick_exit_handlers,
};

unsafe extern "C" fn dummy_handler() {}

#[test]
fn register_handler() {
    unsafe { reset_at_quick_exit_handlers() };
    assert_eq!(unsafe { at_quick_exit(Some(dummy_handler)) }, 0);
    unsafe { reset_at_quick_exit_handlers() };
}
