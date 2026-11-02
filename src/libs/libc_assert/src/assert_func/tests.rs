// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    emit_assert,
    emit_assert_func,
    format_uint,
    write_cstr,
    write_int,
    Sink,
    U32_MAX_DIGITS,
};

/// Test sink that accumulates written bytes in memory for inspection.
struct VecSink(::std::vec::Vec<u8>);

impl Sink for VecSink {
    fn write(&mut self, buf: &[u8]) {
        self.0.extend_from_slice(buf);
    }
}

#[test]
fn format_uint_zero() {
    let mut buf = [0u8; U32_MAX_DIGITS];
    assert_eq!(format_uint(0, &mut buf), b"0");
}

#[test]
fn format_uint_single_digit() {
    let mut buf = [0u8; U32_MAX_DIGITS];
    assert_eq!(format_uint(7, &mut buf), b"7");
}

#[test]
fn format_uint_multi_digit() {
    let mut buf = [0u8; U32_MAX_DIGITS];
    assert_eq!(format_uint(1000, &mut buf), b"1000");
}

#[test]
fn format_uint_max() {
    let mut buf = [0u8; U32_MAX_DIGITS];
    assert_eq!(format_uint(u32::MAX, &mut buf), b"4294967295");
}

#[test]
fn assert_func_message_is_well_formed() {
    let mut sink = VecSink(::std::vec::Vec::new());
    // SAFETY: every pointer references a valid null-terminated C string literal.
    unsafe {
        emit_assert_func(
            &mut sink,
            c"file.c".as_ptr(),
            42,
            c"my_func".as_ptr(),
            c"x == 1".as_ptr(),
        );
    }
    assert_eq!(sink.0, b"file.c:42: my_func: Assertion `x == 1' failed.\n");
}

#[test]
fn assert_message_is_well_formed() {
    let mut sink = VecSink(::std::vec::Vec::new());
    // SAFETY: every pointer references a valid null-terminated C string literal.
    unsafe {
        emit_assert(&mut sink, c"main.c".as_ptr(), 7, c"ptr != NULL".as_ptr());
    }
    assert_eq!(sink.0, b"main.c:7: Assertion `ptr != NULL' failed.\n");
}

#[test]
fn write_cstr_ignores_null() {
    let mut sink = VecSink(::std::vec::Vec::new());
    // SAFETY: passing a null pointer is explicitly supported and writes nothing.
    unsafe {
        write_cstr(&mut sink, ::core::ptr::null());
    }
    assert!(sink.0.is_empty());
}

#[test]
fn assert_func_tolerates_null_function() {
    let mut sink = VecSink(::std::vec::Vec::new());
    // SAFETY: file/expression are valid C strings; a null function name is supported.
    unsafe {
        emit_assert_func(&mut sink, c"file.c".as_ptr(), 42, ::core::ptr::null(), c"x".as_ptr());
    }
    assert_eq!(sink.0, b"file.c:42: : Assertion `x' failed.\n");
}

#[test]
fn write_int_preserves_negative_sign() {
    let mut sink = VecSink(::std::vec::Vec::new());
    write_int(&mut sink, -42);
    assert_eq!(sink.0, b"-42");
}

#[test]
fn write_int_formats_min_value() {
    let mut sink = VecSink(::std::vec::Vec::new());
    write_int(&mut sink, ::sysapi::ffi::c_int::MIN);
    assert_eq!(sink.0, b"-2147483648");
}

#[test]
fn assert_func_formats_negative_line() {
    let mut sink = VecSink(::std::vec::Vec::new());
    // SAFETY: every pointer references a valid null-terminated C string literal.
    unsafe {
        emit_assert_func(
            &mut sink,
            c"file.c".as_ptr(),
            -1,
            c"my_func".as_ptr(),
            c"x == 1".as_ptr(),
        );
    }
    assert_eq!(sink.0, b"file.c:-1: my_func: Assertion `x == 1' failed.\n");
}

#[test]
fn assert_formats_negative_line() {
    let mut sink = VecSink(::std::vec::Vec::new());
    // SAFETY: every pointer references a valid null-terminated C string literal.
    unsafe {
        emit_assert(&mut sink, c"main.c".as_ptr(), -7, c"ptr != NULL".as_ptr());
    }
    assert_eq!(sink.0, b"main.c:-7: Assertion `ptr != NULL' failed.\n");
}
