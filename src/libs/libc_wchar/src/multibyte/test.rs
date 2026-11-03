// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::{
    cchar_of,
    mbrlen,
    mbrtowc,
    mbsinit,
    mbsnrtowcs,
    mbsrtowcs,
    wcrtomb,
    wcsnrtombs,
    wcsrtombs,
    SIZE_INCOMPLETE,
};
use crate::{
    mbstate::mbstate_t,
    wchar_t::wchar_t,
};

fn as_chars(bytes: &[u8]) -> ::std::vec::Vec<::sysapi::ffi::c_char> {
    bytes.iter().map(|&byte| cchar_of(byte)).collect()
}

#[test]
fn test_mbsinit_null_is_initial() {
    // A null state pointer designates the initial conversion state.
    assert_eq!(unsafe { mbsinit(core::ptr::null()) }, 1);
}

#[test]
fn test_mbsinit_zero_count_is_initial() {
    let st: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };
    assert_eq!(unsafe { mbsinit(&st) }, 1);
}

#[test]
fn test_mbsinit_pending_is_not_initial() {
    // A non-zero pending-byte count means the state is mid-sequence.
    let st: mbstate_t = mbstate_t {
        count: 2,
        bytes: [0; 4],
    };
    assert_eq!(unsafe { mbsinit(&st) }, 0);
}

#[test]
fn test_mbrtowc_null_source_resets_state() {
    let mut st: mbstate_t = mbstate_t {
        count: 1,
        bytes: [0; 4],
    };
    let ret: usize = unsafe { mbrtowc(core::ptr::null_mut(), core::ptr::null(), 0, &mut st) };
    assert_eq!(ret, 0);
    assert_eq!(st.count, 0);
    assert_eq!(unsafe { mbsinit(&st) }, 1);
}

#[test]
fn test_mbrtowc_accepts_high_byte_in_posix_locale() {
    let mut st: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };
    let mut wc: wchar_t = 0;
    let input: [::sysapi::ffi::c_char; 1] = [cchar_of(0xff)];
    let ret: usize = unsafe { mbrtowc(&mut wc, input.as_ptr(), 1, &mut st) };
    assert_eq!(ret, 1);
    assert_eq!(wc, 0xff);
    assert_eq!(unsafe { mbsinit(&st) }, 1);
}

#[test]
fn test_mbrtowc_empty_input_is_incomplete() {
    let mut st: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };
    let input: [::sysapi::ffi::c_char; 1] = [cchar_of(0x61)];
    let ret: usize = unsafe { mbrtowc(core::ptr::null_mut(), input.as_ptr(), 0, &mut st) };
    assert_eq!(ret, SIZE_INCOMPLETE);
    assert_eq!(unsafe { mbsinit(&st) }, 1);
}

#[test]
fn test_mbrtowc_nul_character_returns_zero() {
    let mut st: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };
    let input: [::sysapi::ffi::c_char; 1] = [0];
    let mut wc: wchar_t = -1;
    let ret: usize = unsafe { mbrtowc(&mut wc, input.as_ptr(), 1, &mut st) };
    assert_eq!(ret, 0);
    assert_eq!(wc, 0);
    assert_eq!(unsafe { mbsinit(&st) }, 1);
}

#[test]
fn test_mbrlen_reports_complete_character_length() {
    let input = as_chars(&[0xff]);
    let mut st: mbstate_t = mbstate_t {
        count: 0,
        bytes: [0; 4],
    };
    let ret: usize = unsafe { mbrlen(input.as_ptr(), input.len(), &mut st) };
    assert_eq!(ret, 1);
    assert_eq!(unsafe { mbsinit(&st) }, 1);
}

#[test]
fn test_wcrtomb_encodes_nul_character() {
    let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
    let ret: usize = unsafe { wcrtomb(output.as_mut_ptr(), 0, core::ptr::null_mut()) };
    assert_eq!(ret, 1);
    assert_eq!(output[0], 0);
}

#[test]
fn test_mbsrtowcs_updates_source_after_partial_output() {
    let input = as_chars(b"ab\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let mut output: [wchar_t; 1] = [0];
    let ret: usize =
        unsafe { mbsrtowcs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut()) };
    assert_eq!(ret, 1);
    assert_eq!(output, [0x61]);
    assert_eq!(src, unsafe { input.as_ptr().add(1) });
}

#[test]
fn test_mbsrtowcs_nulls_source_after_terminator() {
    let input = as_chars(b"a\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let mut output: [wchar_t; 2] = [-1; 2];
    let ret: usize =
        unsafe { mbsrtowcs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut()) };
    assert_eq!(ret, 1);
    assert_eq!(output, [0x61, 0]);
    assert!(src.is_null());
}

#[test]
fn test_wcsrtombs_updates_source_after_partial_output() {
    let input: [wchar_t; 3] = [0x61, 0xff, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 1] = [0];
    let ret: usize =
        unsafe { wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut()) };
    assert_eq!(ret, 1);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(src, unsafe { input.as_ptr().add(1) });
}

#[test]
fn test_wcsrtombs_updates_source_on_encoding_error_after_prefix() {
    let input: [wchar_t; 3] = [0x61, 0x100, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 2] = [-1; 2];
    let ret: usize =
        unsafe { wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut()) };
    assert_eq!(ret, usize::MAX);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(src, unsafe { input.as_ptr().add(1) });
}

#[test]
fn test_wcsrtombs_nulls_source_after_terminator() {
    let input: [wchar_t; 2] = [0x61, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 2] = [-1; 2];
    let ret: usize =
        unsafe { wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut()) };
    assert_eq!(ret, 1);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(output[1], 0);
    assert!(src.is_null());
}

#[test]
fn test_mbsrtowcs_null_dst_leaves_source_unchanged() {
    // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched.
    let input = as_chars(b"ab\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let ret: usize =
        unsafe { mbsrtowcs(core::ptr::null_mut(), &mut src, 0, core::ptr::null_mut()) };
    assert_eq!(ret, 2);
    assert_eq!(src, input.as_ptr());
}

#[test]
fn test_wcsrtombs_null_dst_leaves_source_unchanged() {
    // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched.
    let input: [wchar_t; 3] = [0x61, 0x62, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let ret: usize =
        unsafe { wcsrtombs(core::ptr::null_mut(), &mut src, 0, core::ptr::null_mut()) };
    assert_eq!(ret, 2);
    assert_eq!(src, input.as_ptr());
}

#[test]
fn test_wcsrtombs_no_room_for_terminator_keeps_source() {
    // When the buffer is exactly filled by the content, there is no room for the terminating
    // null byte, so conversion stops early due to the length limit and `*src` is left pointing
    // at the terminator rather than being nulled.
    let input: [wchar_t; 3] = [0x61, 0x62, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 2] = [-1; 2];
    let ret: usize =
        unsafe { wcsrtombs(output.as_mut_ptr(), &mut src, output.len(), core::ptr::null_mut()) };
    assert_eq!(ret, 2);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(output[1], cchar_of(0x62));
    assert_eq!(src, unsafe { input.as_ptr().add(2) });
}

#[test]
fn test_mbsnrtowcs_stops_at_byte_budget() {
    // Only the first two of the three available bytes may be read, so conversion stops with
    // `*src` pointing just past the last converted byte and no null is written.
    let input = as_chars(b"abc\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let mut output: [wchar_t; 4] = [-1; 4];
    let ret: usize = unsafe {
        mbsnrtowcs(output.as_mut_ptr(), &mut src, 2, output.len(), core::ptr::null_mut())
    };
    assert_eq!(ret, 2);
    assert_eq!(output[0], 0x61);
    assert_eq!(output[1], 0x62);
    assert_eq!(src, unsafe { input.as_ptr().add(2) });
}

#[test]
fn test_mbsnrtowcs_nulls_source_after_terminator_within_budget() {
    // The terminator lies within the byte budget, so it is converted and `*src` is nulled.
    let input = as_chars(b"ab\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let mut output: [wchar_t; 4] = [-1; 4];
    let ret: usize = unsafe {
        mbsnrtowcs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
    };
    assert_eq!(ret, 2);
    assert_eq!(output[0], 0x61);
    assert_eq!(output[1], 0x62);
    assert_eq!(output[2], 0);
    assert!(src.is_null());
}

#[test]
fn test_mbsnrtowcs_stops_when_destination_full() {
    // The destination holds a single wide character, so conversion stops there even though more
    // input and byte budget remain.
    let input = as_chars(b"abc\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let mut output: [wchar_t; 1] = [0];
    let ret: usize = unsafe {
        mbsnrtowcs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
    };
    assert_eq!(ret, 1);
    assert_eq!(output[0], 0x61);
    assert_eq!(src, unsafe { input.as_ptr().add(1) });
}

#[test]
fn test_mbsnrtowcs_null_dst_leaves_source_unchanged() {
    // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched, while the byte
    // budget still bounds the reported length.
    let input = as_chars(b"abcd\0");
    let mut src: *const ::sysapi::ffi::c_char = input.as_ptr();
    let ret: usize =
        unsafe { mbsnrtowcs(core::ptr::null_mut(), &mut src, 3, 0, core::ptr::null_mut()) };
    assert_eq!(ret, 3);
    assert_eq!(src, input.as_ptr());
}

#[test]
fn test_wcsnrtombs_stops_at_wide_char_budget() {
    // Only the first two of the three available wide characters may be read, so conversion stops
    // with `*src` pointing just past the last converted wide character.
    let input: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
    let ret: usize = unsafe {
        wcsnrtombs(output.as_mut_ptr(), &mut src, 2, output.len(), core::ptr::null_mut())
    };
    assert_eq!(ret, 2);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(output[1], cchar_of(0x62));
    assert_eq!(src, unsafe { input.as_ptr().add(2) });
}

#[test]
fn test_wcsnrtombs_nulls_source_after_terminator_within_budget() {
    // The terminator lies within the wide-character budget, so it is converted and `*src` is
    // nulled.
    let input: [wchar_t; 3] = [0x61, 0x62, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
    let ret: usize = unsafe {
        wcsnrtombs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
    };
    assert_eq!(ret, 2);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(output[1], cchar_of(0x62));
    assert_eq!(output[2], 0);
    assert!(src.is_null());
}

#[test]
fn test_wcsnrtombs_reports_error_on_unencodable_wide_char() {
    // A wide character outside the single-byte range aborts conversion with `EILSEQ` and leaves
    // `*src` pointing at the offending character.
    let input: [wchar_t; 3] = [0x61, 0x100, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let mut output: [::sysapi::ffi::c_char; 4] = [-1; 4];
    let ret: usize = unsafe {
        wcsnrtombs(output.as_mut_ptr(), &mut src, 8, output.len(), core::ptr::null_mut())
    };
    assert_eq!(ret, usize::MAX);
    assert_eq!(output[0], cchar_of(0x61));
    assert_eq!(src, unsafe { input.as_ptr().add(1) });
}

#[test]
fn test_wcsnrtombs_null_dst_leaves_source_unchanged() {
    // In measuring mode (null `dst`) POSIX requires `*src` to be left untouched, while the
    // wide-character budget still bounds the reported length.
    let input: [wchar_t; 4] = [0x61, 0x62, 0x63, 0];
    let mut src: *const wchar_t = input.as_ptr();
    let ret: usize =
        unsafe { wcsnrtombs(core::ptr::null_mut(), &mut src, 2, 0, core::ptr::null_mut()) };
    assert_eq!(ret, 2);
    assert_eq!(src, input.as_ptr());
}
