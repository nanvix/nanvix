// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::setjmp;
use crate::jmp_buf::{
    jmp_buf,
    JMP_BUF_REGS,
};
use ::sysapi::ffi::c_int;

// On the host, setjmp is a non-functional stub that always reports a direct (zero) return. The
// real non-local control flow only exists in the guest assembly build, which cannot run under
// the host test harness.
#[test]
fn setjmp_stub_reports_direct_return() {
    let mut buf: jmp_buf = jmp_buf {
        regs: [0; JMP_BUF_REGS],
    };
    let result: c_int = unsafe { setjmp(&mut buf) };
    assert_eq!(result, 0);
}
