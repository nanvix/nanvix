// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

#[no_mangle]
pub extern "C" fn kill(_pid: i32, _signal: i32) -> i32 {
    ::nvx::log!("kill(): pid = {}, signal = {}", _pid, _signal);
    // TODO: Implement this system call.
    -1
}
