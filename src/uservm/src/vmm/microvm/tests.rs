// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn shutdown_signal_sets_kvm_immediate_exit() {
    let mut immediate_exit: u8 = 7;
    let immediate_exit_ptr: *mut u8 = ::core::ptr::addr_of_mut!(immediate_exit);
    // SAFETY: `immediate_exit` remains live until after the guard is dropped.
    let guard: KvmImmediateExitGuard =
        unsafe { KvmImmediateExitGuard::register(immediate_exit_ptr) };

    assert_eq!(immediate_exit, 0);
    SHUTDOWN.with(|shutdown| shutdown.store(false, Ordering::SeqCst));

    vcpu_thread_signal_handler(INTERRUPT_SIGNAL);

    assert!(SHUTDOWN.with(|shutdown| shutdown.load(Ordering::SeqCst)));
    assert_eq!(immediate_exit, 1);

    drop(guard);
    assert!(KVM_IMMEDIATE_EXIT.with(|slot| slot.get().is_null()));
    SHUTDOWN.with(|shutdown| shutdown.store(false, Ordering::SeqCst));
}
