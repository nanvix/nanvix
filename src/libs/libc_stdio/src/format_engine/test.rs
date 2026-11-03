// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::vec::Vec;

/// Test write target that collects output into a Vec.
struct VecWriter {
    data: Vec<u8>,
}

impl VecWriter {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.data).expect("valid utf8")
    }
}

impl WriteTarget for VecWriter {
    fn write_byte(&mut self, b: u8) -> Result<(), ()> {
        self.data.push(b);
        Ok(())
    }

    fn total(&self) -> usize {
        self.data.len()
    }
}

/// Test argument source backed by slices.
struct TestArgs {
    ints: Vec<c_int>,
    uints: Vec<u32>,
    longs: Vec<i32>,
    ulongs: Vec<u32>,
    longlongs: Vec<i64>,
    ulonglongs: Vec<u64>,
    sizes: Vec<usize>,
    ptrs: Vec<usize>,
    strs: Vec<*const c_char>,
    doubles: Vec<f64>,
    int_idx: usize,
    uint_idx: usize,
    long_idx: usize,
    ulong_idx: usize,
    longlong_idx: usize,
    ulonglong_idx: usize,
    size_idx: usize,
    ptr_idx: usize,
    str_idx: usize,
    double_idx: usize,
}

impl TestArgs {
    fn new() -> Self {
        Self {
            ints: Vec::new(),
            uints: Vec::new(),
            longs: Vec::new(),
            ulongs: Vec::new(),
            longlongs: Vec::new(),
            ulonglongs: Vec::new(),
            sizes: Vec::new(),
            ptrs: Vec::new(),
            strs: Vec::new(),
            doubles: Vec::new(),
            int_idx: 0,
            uint_idx: 0,
            long_idx: 0,
            ulong_idx: 0,
            longlong_idx: 0,
            ulonglong_idx: 0,
            size_idx: 0,
            ptr_idx: 0,
            str_idx: 0,
            double_idx: 0,
        }
    }
}

impl ArgSource for TestArgs {
    fn next_int(&mut self) -> c_int {
        let v: c_int = self.ints[self.int_idx];
        self.int_idx += 1;
        v
    }
    fn next_uint(&mut self) -> u32 {
        let v: u32 = self.uints[self.uint_idx];
        self.uint_idx += 1;
        v
    }
    fn next_long(&mut self) -> i32 {
        let v: i32 = self.longs[self.long_idx];
        self.long_idx += 1;
        v
    }
    fn next_ulong(&mut self) -> u32 {
        let v: u32 = self.ulongs[self.ulong_idx];
        self.ulong_idx += 1;
        v
    }
    fn next_longlong(&mut self) -> i64 {
        let v: i64 = self.longlongs[self.longlong_idx];
        self.longlong_idx += 1;
        v
    }
    fn next_ulonglong(&mut self) -> u64 {
        let v: u64 = self.ulonglongs[self.ulonglong_idx];
        self.ulonglong_idx += 1;
        v
    }
    fn next_size(&mut self) -> usize {
        let v: usize = self.sizes[self.size_idx];
        self.size_idx += 1;
        v
    }
    fn next_ptr(&mut self) -> usize {
        let v: usize = self.ptrs[self.ptr_idx];
        self.ptr_idx += 1;
        v
    }
    fn next_str(&mut self) -> *const c_char {
        let v: *const c_char = self.strs[self.str_idx];
        self.str_idx += 1;
        v
    }
    fn next_double(&mut self) -> f64 {
        let v: f64 = self.doubles[self.double_idx];
        self.double_idx += 1;
        v
    }
}

/// Helper: format with test args and return the output string.
fn fmt(format: &[u8], args: &mut TestArgs) -> String {
    let mut writer: VecWriter = VecWriter::new();
    format_core(&mut writer, format.as_ptr().cast::<c_char>(), args);
    writer.as_str().to_string()
}

#[test]
fn test_plain_string() {
    let mut args: TestArgs = TestArgs::new();
    assert_eq!(fmt(b"hello\0", &mut args), "hello");
}

#[test]
fn test_percent_d() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(42);
    assert_eq!(fmt(b"%d\0", &mut args), "42");
}

#[test]
fn test_percent_d_negative() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(-7);
    assert_eq!(fmt(b"%d\0", &mut args), "-7");
}

#[test]
fn test_percent_u() {
    let mut args: TestArgs = TestArgs::new();
    args.uints.push(123);
    assert_eq!(fmt(b"%u\0", &mut args), "123");
}

#[test]
fn test_percent_x() {
    let mut args: TestArgs = TestArgs::new();
    args.uints.push(255);
    assert_eq!(fmt(b"%x\0", &mut args), "ff");
}

#[test]
fn test_percent_upper_x() {
    let mut args: TestArgs = TestArgs::new();
    args.uints.push(255);
    assert_eq!(fmt(b"%X\0", &mut args), "FF");
}

#[test]
fn test_percent_o() {
    let mut args: TestArgs = TestArgs::new();
    args.uints.push(8);
    assert_eq!(fmt(b"%o\0", &mut args), "10");
}

#[test]
fn test_percent_s() {
    let mut args: TestArgs = TestArgs::new();
    let s: &[u8] = b"world\0";
    args.strs.push(s.as_ptr().cast::<c_char>());
    assert_eq!(fmt(b"hello %s\0", &mut args), "hello world");
}

#[test]
fn test_percent_s_null() {
    let mut args: TestArgs = TestArgs::new();
    args.strs.push(core::ptr::null());
    assert_eq!(fmt(b"%s\0", &mut args), "(null)");
}

#[test]
fn test_percent_c() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(b'A' as c_int);
    assert_eq!(fmt(b"%c\0", &mut args), "A");
}

#[test]
fn test_percent_ls() {
    let mut args: TestArgs = TestArgs::new();
    // A wide string is an array of wchar_t (i32); %ls renders it as bytes in the C locale.
    let ws: [i32; 6] = [
        b'w' as i32,
        b'o' as i32,
        b'r' as i32,
        b'l' as i32,
        b'd' as i32,
        0,
    ];
    args.strs.push(ws.as_ptr().cast::<c_char>());
    assert_eq!(fmt(b"hi %ls\0", &mut args), "hi world");
}

#[test]
fn test_percent_ls_null() {
    let mut args: TestArgs = TestArgs::new();
    args.strs.push(core::ptr::null());
    assert_eq!(fmt(b"%ls\0", &mut args), "(null)");
}

#[test]
fn test_percent_ls_precision() {
    let mut args: TestArgs = TestArgs::new();
    let ws: [i32; 6] = [
        b'w' as i32,
        b'o' as i32,
        b'r' as i32,
        b'l' as i32,
        b'd' as i32,
        0,
    ];
    args.strs.push(ws.as_ptr().cast::<c_char>());
    assert_eq!(fmt(b"%.3ls\0", &mut args), "wor");
}

#[test]
fn test_percent_lc() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(b'A' as c_int);
    assert_eq!(fmt(b"%lc\0", &mut args), "A");
}

#[test]
fn test_percent_percent() {
    let mut args: TestArgs = TestArgs::new();
    assert_eq!(fmt(b"100%%\0", &mut args), "100%");
}

#[test]
fn test_width_right_align() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(42);
    assert_eq!(fmt(b"%10d\0", &mut args), "        42");
}

#[test]
fn test_width_left_align() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(42);
    assert_eq!(fmt(b"%-10d\0", &mut args), "42        ");
}

#[test]
fn test_zero_pad() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(42);
    assert_eq!(fmt(b"%05d\0", &mut args), "00042");
}

#[test]
fn test_plus_flag() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(42);
    assert_eq!(fmt(b"%+d\0", &mut args), "+42");
}

#[test]
fn test_space_flag() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(42);
    assert_eq!(fmt(b"% d\0", &mut args), " 42");
}

#[test]
fn test_alternate_hex() {
    let mut args: TestArgs = TestArgs::new();
    args.uints.push(255);
    assert_eq!(fmt(b"%#x\0", &mut args), "0xff");
}

#[test]
fn test_string_precision() {
    let mut args: TestArgs = TestArgs::new();
    let s: &[u8] = b"hello world\0";
    args.strs.push(s.as_ptr().cast::<c_char>());
    assert_eq!(fmt(b"%.5s\0", &mut args), "hello");
}

#[test]
fn test_string_width_right() {
    let mut args: TestArgs = TestArgs::new();
    let s: &[u8] = b"hi\0";
    args.strs.push(s.as_ptr().cast::<c_char>());
    assert_eq!(fmt(b"%10s\0", &mut args), "        hi");
}

#[test]
fn test_long_long_d() {
    let mut args: TestArgs = TestArgs::new();
    args.longlongs.push(1234567890123_i64);
    assert_eq!(fmt(b"%lld\0", &mut args), "1234567890123");
}

#[test]
fn test_pointer() {
    let mut args: TestArgs = TestArgs::new();
    args.ptrs.push(0xDEAD);
    assert_eq!(fmt(b"%p\0", &mut args), "0xdead");
}

#[test]
fn test_zero_value_d() {
    let mut args: TestArgs = TestArgs::new();
    args.ints.push(0);
    assert_eq!(fmt(b"%d\0", &mut args), "0");
}

#[test]
fn test_null_format() {
    let mut args: TestArgs = TestArgs::new();
    let mut writer: VecWriter = VecWriter::new();
    let ret: c_int = format_core(&mut writer, core::ptr::null(), &mut args);
    assert_eq!(ret, -1);
}

#[test]
fn test_buf_writer_truncation() {
    let mut buf: [u8; 6] = [0xFF; 6];
    let mut writer: BufWriter = BufWriter::new(buf.as_mut_ptr(), 6);
    // Write "hello world" — should truncate to "hello".
    let data: &[u8] = b"hello world";
    for &b in data {
        let _ = writer.write_byte(b);
    }
    writer.null_terminate();
    assert_eq!(writer.total(), 11);
    assert_eq!(&buf, b"hello\0");
}
