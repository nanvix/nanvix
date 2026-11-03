// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

/// The kcall trap vector must never be `0x80`, because that vector collides with the Linux
/// i386 syscall gate and makes Nanvix user binaries behave unpredictably when run on a Linux
/// host. See the module-level documentation of [`KCALL_VECTOR`] for details.
#[test]
fn test_kcall_vector_avoids_linux_syscall_gate() {
    assert_ne!(KCALL_VECTOR, 0x80);
}

#[test]
fn test_signal_kcall_numbers_round_trip() {
    let signal_kcalls: [(KcallNumber, u32); 6] = [
        (KcallNumber::Sigaction, 41),
        (KcallNumber::Sigprocmask, 42),
        (KcallNumber::Kill, 43),
        (KcallNumber::Sigreturn, 44),
        (KcallNumber::Sigpending, 45),
        (KcallNumber::Sigsuspend, 46),
    ];

    for (kcall, number) in signal_kcalls {
        assert_eq!(u32::from(kcall), number);
        assert_eq!(KcallNumber::from(number), kcall);
    }
}
