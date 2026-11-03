// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
    let mut line: Vec<c_char> = make_c_string(b"/dev/sda1 /mnt/my\\040drive a\\011b x\\134y 0 0");
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
