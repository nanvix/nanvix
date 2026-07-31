// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod capctl;
mod create_thread;
mod detach_thread;
mod duplicate;
mod execv;
mod get_thread_data_area;
mod gettime;
mod join_thread;
mod kill;
mod lock_mutex;
mod mcopy;
mod mctrl;
mod mmap;
mod munmap;
mod set_thread_data_area;
mod sig_restorer;
mod sigaction;
mod signal_cond;
mod sigpending;
mod sigprocmask;
mod sigreturn;
mod sigsuspend;
mod sleep;
#[cfg(target_arch = "aarch64")]
mod sync_instruction_cache;
mod terminate;
mod unlock_mutex;
mod wait_cond;

//==================================================================================================
// Exports
//==================================================================================================

pub use capctl::capctl;
pub use create_thread::create_thread;
pub use detach_thread::detach_thread;
pub use duplicate::duplicate;
pub use execv::execv;
pub use get_thread_data_area::get_thread_data_area;
pub use gettime::gettime;
pub use join_thread::join_thread;
pub use kill::kill;
pub use lock_mutex::lock_mutex;
pub use mcopy::mcopy;
pub use mctrl::mctrl;
pub use mmap::mmap;
pub use munmap::munmap;
pub use set_thread_data_area::set_thread_data_area;
pub use sig_restorer::sig_restorer;
pub use sigaction::sigaction;
pub use signal_cond::signal_cond;
pub use sigpending::sigpending;
pub use sigprocmask::sigprocmask;
pub use sigreturn::sigreturn;
pub use sigsuspend::sigsuspend;
pub use sleep::sleep;
#[cfg(target_arch = "aarch64")]
pub use sync_instruction_cache::sync_instruction_cache;
pub use terminate::terminate;
pub use unlock_mutex::unlock_mutex;
pub use wait_cond::wait_cond;
