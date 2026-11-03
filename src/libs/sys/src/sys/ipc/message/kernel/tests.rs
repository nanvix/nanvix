// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::PushArgs;

#[test]
fn push_args_decode_deferred_flag() {
    let mut args: PushArgs = PushArgs::zeroed();
    assert!(!args.is_deferred().expect("zero flags should be valid"));

    args.flags = PushArgs::FLAG_DEFERRED;
    assert!(args.is_deferred().expect("deferred flag should be valid"));
    assert!(!args
        .cancels_deferred()
        .expect("deferred flag should be valid"));

    args.flags = PushArgs::FLAG_CANCEL_DEFERRED;
    assert!(!args.is_deferred().expect("cancel flag should be valid"));
    assert!(args
        .cancels_deferred()
        .expect("cancel flag should be valid"));
}

#[test]
fn push_args_reject_unknown_flags() {
    let mut args: PushArgs = PushArgs::zeroed();
    args.flags = PushArgs::FLAG_CANCEL_DEFERRED << 1;
    assert!(args.is_deferred().is_err());
}
