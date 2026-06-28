// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::Mntent;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// ASCII `#`, compared in `i32` space to avoid `c_char` sign-dependent casts.
const HASH: i32 = b'#' as i32;
/// ASCII `-`.
const MINUS: i32 = b'-' as i32;
/// ASCII `,`.
const COMMA: i32 = b',' as i32;
/// ASCII `=`.
const EQUALS: i32 = b'=' as i32;
/// ASCII `0`.
const ZERO: i32 = b'0' as i32;
/// ASCII `7`, the largest octal digit.
const SEVEN: i32 = b'7' as i32;
/// ASCII `9`.
const NINE: i32 = b'9' as i32;
/// ASCII `\`.
const BACKSLASH: i32 = b'\\' as i32;

//==================================================================================================
// Internal Helpers
//==================================================================================================

/// Returns `true` if `c` is an ASCII blank or line-separator character.
fn is_space(c: c_char) -> bool {
    matches!(i32::from(c), 0x20 | 0x09 | 0x0a | 0x0d | 0x0b | 0x0c)
}

/// Returns `true` if `c` is an ASCII octal digit (`0`–`7`).
fn is_octal(c: c_char) -> bool {
    (ZERO..=SEVEN).contains(&i32::from(c))
}

/// Decodes glibc-style octal escapes (`\NNN`) in the null-terminated string `s`, in place.
///
/// A backslash followed by exactly three octal digits encoding a byte value (`0..=255`) is replaced
/// by that byte; any other backslash is kept verbatim. This mirrors how `mount(8)`/glibc store
/// whitespace and backslashes in `mnt_*` fields (e.g. a space as `\040`). Decoding never lengthens
/// the string, so the rewrite always stays within the original buffer.
///
/// # Safety
///
/// `s` must be a writable, null-terminated C string.
unsafe fn unescape_in_place(s: *mut c_char) {
    let mut read: usize = 0;
    let mut write: usize = 0;
    loop {
        let c: c_char = *s.add(read);
        if c == 0 {
            break;
        }
        // A backslash followed by three octal digits is decoded to a single byte. The `&&`
        // short-circuits before reading past the terminator, since a null byte is not octal.
        if i32::from(c) == BACKSLASH
            && is_octal(*s.add(read + 1))
            && is_octal(*s.add(read + 2))
            && is_octal(*s.add(read + 3))
        {
            let d0: i32 = i32::from(*s.add(read + 1)) - ZERO;
            let d1: i32 = i32::from(*s.add(read + 2)) - ZERO;
            let d2: i32 = i32::from(*s.add(read + 3)) - ZERO;
            let value: i32 = d0 * 64 + d1 * 8 + d2;
            if let Ok(byte) = u8::try_from(value) {
                *s.add(write) = c_char::from_ne_bytes(byte.to_ne_bytes());
                write += 1;
                read += 4;
                continue;
            }
        }
        *s.add(write) = c;
        write += 1;
        read += 1;
    }
    *s.add(write) = 0;
}

/// Returns the length of the null-terminated C string `s`, excluding the terminator.
///
/// # Safety
///
/// `s` must be a valid null-terminated C string.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// Returns `true` if the first `n` bytes at `a` and `b` are equal.
///
/// # Safety
///
/// `a` and `b` must be readable for at least `n` bytes.
unsafe fn bytes_equal(a: *const c_char, b: *const c_char, n: usize) -> bool {
    let mut i: usize = 0;
    while i < n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
        i += 1;
    }
    true
}

/// Extracts the next whitespace-delimited token from `*p`, null-terminating it in place and
/// advancing `*p` past it.
///
/// # Return Value
///
/// A pointer to the token, or null if no token remains.
///
/// # Safety
///
/// `*p` must point into a writable, null-terminated buffer.
unsafe fn take_token(p: &mut *mut c_char) -> *mut c_char {
    while is_space(**p) {
        *p = (*p).add(1);
    }
    if **p == 0 {
        return core::ptr::null_mut();
    }
    let start: *mut c_char = *p;
    while **p != 0 && !is_space(**p) {
        *p = (*p).add(1);
    }
    if **p != 0 {
        **p = 0;
        *p = (*p).add(1);
    }
    start
}

/// Extracts the next token and parses it as a base-10 integer, defaulting to `0` when the token is
/// absent or malformed.
///
/// # Safety
///
/// `*p` must point into a writable, null-terminated buffer.
unsafe fn take_int(p: &mut *mut c_char) -> c_int {
    let token: *mut c_char = take_token(p);
    if token.is_null() {
        return 0;
    }

    let mut index: usize = 0;
    let negative: bool = i32::from(*token) == MINUS;
    if negative {
        index = 1;
    }

    // Accumulate the magnitude in `i64` so that `i32::MIN` — whose magnitude exceeds `c_int::MAX`,
    // and which `write_int` is able to emit — round-trips correctly. Saturate, then clamp into the
    // `c_int` range.
    let mut magnitude: i64 = 0;
    loop {
        let digit: i32 = i32::from(*token.add(index));
        if !(ZERO..=NINE).contains(&digit) {
            break;
        }
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(i64::from(digit - ZERO));
        index += 1;
    }

    let signed: i64 = if negative { -magnitude } else { magnitude };
    let clamped: i64 = signed.clamp(i64::from(c_int::MIN), i64::from(c_int::MAX));
    c_int::try_from(clamped).unwrap_or(0)
}

//==================================================================================================
// Crate-Visible Functions
//==================================================================================================

/// Parses one `fstab`/`mtab` line in place, filling `ent` with pointers into `line`.
///
/// # Return Value
///
/// `true` when a populated entry was parsed, `false` for blank or comment lines or lines that lack
/// the four mandatory string fields.
///
/// # Safety
///
/// `line` must be a writable, null-terminated buffer and `ent` must be valid for writes.
pub(crate) unsafe fn parse_line(line: *mut c_char, ent: *mut Mntent) -> bool {
    let mut p: *mut c_char = line;

    // Detect blank and comment lines.
    while is_space(*p) {
        p = p.add(1);
    }
    if *p == 0 || i32::from(*p) == HASH {
        return false;
    }

    let fsname: *mut c_char = take_token(&mut p);
    let dir: *mut c_char = take_token(&mut p);
    let typ: *mut c_char = take_token(&mut p);
    let opts: *mut c_char = take_token(&mut p);
    if fsname.is_null() || dir.is_null() || typ.is_null() || opts.is_null() {
        return false;
    }

    // Decode glibc-style octal escapes in the four string fields (e.g. `\040` -> space). This is
    // safe to do after tokenizing because escaped whitespace contains no literal separator, so the
    // fields were not split on it.
    unescape_in_place(fsname);
    unescape_in_place(dir);
    unescape_in_place(typ);
    unescape_in_place(opts);

    let freq: c_int = take_int(&mut p);
    let passno: c_int = take_int(&mut p);

    (*ent).mnt_fsname = fsname;
    (*ent).mnt_dir = dir;
    (*ent).mnt_type = typ;
    (*ent).mnt_opts = opts;
    (*ent).mnt_freq = freq;
    (*ent).mnt_passno = passno;
    true
}

/// Searches the comma-separated option list `opts` for an option named `opt`.
///
/// # Return Value
///
/// A pointer to the matching option within `opts`, or null when not present.
///
/// # Safety
///
/// `opts` and `opt` must be null or valid null-terminated C strings.
pub(crate) unsafe fn option_search(opts: *const c_char, opt: *const c_char) -> *mut c_char {
    if opts.is_null() || opt.is_null() {
        return core::ptr::null_mut();
    }
    let optlen: usize = c_strlen(opt);
    if optlen == 0 {
        return core::ptr::null_mut();
    }

    let mut p: *const c_char = opts;
    loop {
        // Skip separators.
        while i32::from(*p) == COMMA {
            p = p.add(1);
        }
        if *p == 0 {
            return core::ptr::null_mut();
        }

        let token: *const c_char = p;
        while *p != 0 && i32::from(*p) != COMMA {
            p = p.add(1);
        }
        let tokenlen: usize = usize::try_from(p.offset_from(token)).unwrap_or(0);

        // Match the option name, requiring a whole-token (or `name=value`) match.
        if tokenlen >= optlen && bytes_equal(token, opt, optlen) {
            let trailing: i32 = i32::from(*token.add(optlen));
            if trailing == 0 || trailing == COMMA || trailing == EQUALS {
                return token.cast_mut();
            }
        }

        if *p == 0 {
            return core::ptr::null_mut();
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        option_search,
        parse_line,
    };
    use crate::Mntent;
    use ::std::vec::Vec;
    use ::sysapi::ffi::{
        c_char,
        c_int,
    };

    fn make_c_string(bytes: &[u8]) -> Vec<c_char> {
        let mut v: Vec<c_char> = bytes
            .iter()
            .map(|b| c_char::try_from(*b).expect("byte fits in c_char"))
            .collect();
        v.push(0);
        v
    }

    fn c_str_to_bytes(p: *const c_char) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        let mut i: usize = 0;
        unsafe {
            while *p.add(i) != 0 {
                v.push(u8::from_ne_bytes((*p.add(i)).to_ne_bytes()));
                i += 1;
            }
        }
        v
    }

    fn empty_entry() -> Mntent {
        Mntent {
            mnt_fsname: core::ptr::null_mut(),
            mnt_dir: core::ptr::null_mut(),
            mnt_type: core::ptr::null_mut(),
            mnt_opts: core::ptr::null_mut(),
            mnt_freq: 0,
            mnt_passno: 0,
        }
    }

    #[test]
    fn parses_a_full_entry() {
        let mut line: Vec<c_char> = make_c_string(b"/dev/sda1 / ext4 rw,relatime 1 2");
        let mut ent: Mntent = empty_entry();
        let ok: bool = unsafe { parse_line(line.as_mut_ptr(), &mut ent) };
        assert!(ok);
        assert_eq!(c_str_to_bytes(ent.mnt_fsname), b"/dev/sda1");
        assert_eq!(c_str_to_bytes(ent.mnt_dir), b"/");
        assert_eq!(c_str_to_bytes(ent.mnt_type), b"ext4");
        assert_eq!(c_str_to_bytes(ent.mnt_opts), b"rw,relatime");
        assert_eq!(ent.mnt_freq, 1);
        assert_eq!(ent.mnt_passno, 2);
    }

    #[test]
    fn defaults_missing_dump_and_pass_fields() {
        let mut line: Vec<c_char> = make_c_string(b"\tproc   /proc\tproc  defaults\n");
        let mut ent: Mntent = empty_entry();
        let ok: bool = unsafe { parse_line(line.as_mut_ptr(), &mut ent) };
        assert!(ok);
        assert_eq!(c_str_to_bytes(ent.mnt_fsname), b"proc");
        assert_eq!(c_str_to_bytes(ent.mnt_dir), b"/proc");
        assert_eq!(c_str_to_bytes(ent.mnt_type), b"proc");
        assert_eq!(c_str_to_bytes(ent.mnt_opts), b"defaults");
        assert_eq!(ent.mnt_freq, 0);
        assert_eq!(ent.mnt_passno, 0);
    }

    #[test]
    fn rejects_comment_and_blank_lines() {
        let mut comment: Vec<c_char> = make_c_string(b"   # a comment");
        let mut blank: Vec<c_char> = make_c_string(b"   \t  ");
        let mut partial: Vec<c_char> = make_c_string(b"only two");
        let mut ent: Mntent = empty_entry();
        unsafe {
            assert!(!parse_line(comment.as_mut_ptr(), &mut ent));
            assert!(!parse_line(blank.as_mut_ptr(), &mut ent));
            assert!(!parse_line(partial.as_mut_ptr(), &mut ent));
        }
    }

    #[test]
    fn finds_whole_options_only() {
        let opts: Vec<c_char> = make_c_string(b"rw,noexec,nosuid");
        unsafe {
            // Whole-token matches succeed.
            assert!(!option_search(opts.as_ptr(), make_c_string(b"rw").as_ptr()).is_null());
            assert!(!option_search(opts.as_ptr(), make_c_string(b"noexec").as_ptr()).is_null());
            assert!(!option_search(opts.as_ptr(), make_c_string(b"nosuid").as_ptr()).is_null());
            // A substring of an option is not a match ("exec" in "noexec", "suid" in "nosuid").
            assert!(option_search(opts.as_ptr(), make_c_string(b"exec").as_ptr()).is_null());
            assert!(option_search(opts.as_ptr(), make_c_string(b"suid").as_ptr()).is_null());
            assert!(option_search(opts.as_ptr(), make_c_string(b"ro").as_ptr()).is_null());
        }
    }

    #[test]
    fn matches_option_with_value() {
        let opts: Vec<c_char> = make_c_string(b"rw,uid=1000,gid=1000");
        unsafe {
            let hit: *mut c_char = option_search(opts.as_ptr(), make_c_string(b"uid").as_ptr());
            assert!(!hit.is_null());
            assert_eq!(c_str_to_bytes(hit), b"uid=1000,gid=1000");
            assert!(option_search(opts.as_ptr(), make_c_string(b"id").as_ptr()).is_null());
        }
    }

    #[test]
    fn decodes_octal_escapes_in_fields() {
        // `/mnt/my\040drive` (space), type `a\011b` (tab), opts `x\134y` (backslash).
        let mut line: Vec<c_char> =
            make_c_string(b"/dev/sda1 /mnt/my\\040drive a\\011b x\\134y 0 0");
        let mut ent: Mntent = empty_entry();
        let ok: bool = unsafe { parse_line(line.as_mut_ptr(), &mut ent) };
        assert!(ok);
        assert_eq!(c_str_to_bytes(ent.mnt_fsname), b"/dev/sda1");
        assert_eq!(c_str_to_bytes(ent.mnt_dir), b"/mnt/my drive");
        assert_eq!(c_str_to_bytes(ent.mnt_type), b"a\tb");
        assert_eq!(c_str_to_bytes(ent.mnt_opts), b"x\\y");
    }

    #[test]
    fn keeps_lone_backslash_verbatim() {
        // A backslash not followed by three octal digits is preserved as-is.
        let mut line: Vec<c_char> = make_c_string(b"a\\b /mnt ext4 rw 0 0");
        let mut ent: Mntent = empty_entry();
        let ok: bool = unsafe { parse_line(line.as_mut_ptr(), &mut ent) };
        assert!(ok);
        assert_eq!(c_str_to_bytes(ent.mnt_fsname), b"a\\b");
    }

    #[test]
    fn parses_extreme_dump_and_pass_values() {
        // i32::MIN must round-trip the value written by write_int; large values saturate.
        let mut line: Vec<c_char> = make_c_string(b"none none none none -2147483648 2147483647");
        let mut ent: Mntent = empty_entry();
        let ok: bool = unsafe { parse_line(line.as_mut_ptr(), &mut ent) };
        assert!(ok);
        assert_eq!(ent.mnt_freq, c_int::MIN);
        assert_eq!(ent.mnt_passno, c_int::MAX);
    }
}
