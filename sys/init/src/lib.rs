/*
 * Copyright(c) 2011-2024 The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#![no_std]

//==============================================================================
// Imports
//==============================================================================

// use libnanvix::{
//     misc::{
//         self,
//         KernelModule,
//     },
//     pm,
// };

//==============================================================================
// Standalone Functions
//==============================================================================

/// Print information on all kernel modules.
// fn spawn_servers() {
//     for index in 1.. {
//         let mut kmod: KernelModule = misc::KernelModule::default();

//         if misc::kmod_get(&mut kmod, index) != 0 {
//             break;
//         }

//         let cmdline = core::str::from_utf8(&kmod.cmdline).unwrap();
//         libnanvix::log!("Spawning Server #{}: (cmdline={})", index, cmdline);

//         // Spawn server.
//         let pid: i32 = pm::spawn(kmod.start as *const core::ffi::c_void);
//         if pid < 0 {
//             nanvix::log!("failed to spawn server");
//         }
//     }
// }

#[no_mangle]
pub fn main() {
    libnanvix::log!("Running init server...");
    // spawn_servers();
}
