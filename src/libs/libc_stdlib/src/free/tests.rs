// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[test]
fn null_deallocation() {
    unsafe {
        super::free(core::ptr::null_mut());
    }
}
