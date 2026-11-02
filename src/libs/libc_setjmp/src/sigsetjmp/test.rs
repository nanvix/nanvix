// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::sigsetjmp;
use crate::sigjmp_buf::sigjmp_buf;
use ::sysapi::ffi::c_int;

// On the host, sigsetjmp is a non-functional stub that always reports a direct (zero) return.
// The real non-local control flow only exists in the guest assembly build, which cannot run
// under the host test harness.
#[test]
fn sigsetjmp_stub_reports_direct_return() {
    let mut buf: sigjmp_buf = sigjmp_buf {
        regs: [0; crate::jmp_buf::JMP_BUF_REGS],
        savemask: 0,
    };
    let result: c_int = unsafe { sigsetjmp(&mut buf, 1) };
    assert_eq!(result, 0);
}
