// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Build script for nanvixd-vmm.
//!
//! Emits the `guest_arch`/`guest_is_native` cfgs expected by the OpenVMM
//! virtualization crates this crate depends on.

fn main() {
    build_rs_guest_arch::emit_guest_arch()
}
