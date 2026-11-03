// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::bzero;
use ::std::vec::Vec;

#[test]
fn test_bzero_basic() {
    let mut buf: Vec<u8> = vec![0xFF; 10];
    unsafe { bzero(buf.as_mut_ptr().cast(), 10) };
    assert!(buf.iter().all(|&b| b == 0), "all bytes should be zero");
}
