// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::vec::Vec;

///
/// # Description
///
/// Builds the lexically canonical absolute path of `path` into `out`, null-terminated. A relative
/// `path` is resolved against the absolute `cwd`.
///
/// # Returns
///
/// The length excluding the terminator, or [`None`] if the result does not fit in `out`.
///
fn canonicalize(path: &[u8], cwd: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut len: usize = 0;

    // A relative path is resolved against the current working directory.
    if path.first() != Some(&b'/') && !append_components(cwd, out, &mut len) {
        return None;
    }
    if !append_components(path, out, &mut len) {
        return None;
    }

    // An empty accumulator denotes the root directory.
    if len == 0 {
        if out.is_empty() {
            return None;
        }
        out[0] = b'/';
        len = 1;
    }

    if len >= out.len() {
        return None;
    }
    out[len] = 0;
    Some(len)
}

// Exercise fixed storage without calling guest-ABI lstat/readlink on the host.
fn append_next(adapter: &mut Realpath<'_>) -> Result<usize, c_int> {
    match adapter.next_component() {
        Some(Component::Name(name)) => adapter.push(name),
        _ => Err(EINVAL),
    }
}

#[test]
fn byte_names_and_link_suffix_are_preserved() -> Result<(), c_int> {
    let mut out = [0; PATH_MAX];
    let mut adapter = Realpath::new(b"/dir/\xff-link/../child//", b"/", &mut out)?;
    append_next(&mut adapter)?;
    let parent = append_next(&mut adapter)?;
    assert_eq!(&adapter.out[..=adapter.len], b"/dir/\xff-link\0");
    assert!(adapter.requires_directory());
    adapter.splice_target(parent, b"../\xfe-target")?;
    assert_eq!(&adapter.pending[..adapter.pending_len], b"../\xfe-target/../child//");
    assert_eq!(adapter.next_component(), Some(Component::Parent));
    adapter.parent()?;
    append_next(&mut adapter)?;
    assert_eq!(&adapter.out[..=adapter.len], b"/\xfe-target\0");
    Ok(())
}

#[test]
fn link_targets_reanchor_without_collapsing_components() -> Result<(), c_int> {
    for (target, expected_parent, expected_pending) in [
        (b"next/..".as_slice(), 4, b"next/../suffix".as_slice()),
        (b"/root/..", 0, b"/root/../suffix"),
    ] {
        let mut out = [0; PATH_MAX];
        let mut adapter = Realpath::new(b"/dir/link/suffix", b"/", &mut out)?;
        append_next(&mut adapter)?;
        let parent = append_next(&mut adapter)?;
        adapter.splice_target(parent, target)?;
        assert_eq!(adapter.len, expected_parent);
        assert_eq!(&adapter.pending[..adapter.pending_len], expected_pending);
    }
    Ok(())
}

#[test]
fn fixed_buffers_reserve_terminator_space() -> Result<(), c_int> {
    let mut out = [0; 5];
    let mut adapter = Realpath::new(b"/abc", b"/", &mut out)?;
    append_next(&mut adapter)?;
    assert_eq!(adapter.finish(), Ok(4));
    assert_eq!(&out, b"/abc\0");
    let mut adapter = Realpath::new(b"/abcd", b"/", &mut out)?;
    assert_eq!(append_next(&mut adapter), Err(ENAMETOOLONG));
    assert!(matches!(Realpath::new(&[b'x'; PATH_MAX], b"/", &mut out), Err(ENAMETOOLONG)));
    assert!(matches!(Realpath::new(b".", b"/long-cwd", &mut out), Err(ENAMETOOLONG)));
    assert_eq!(Realpath::new(b"/", b"/", &mut [])?.finish(), Err(ENAMETOOLONG));
    assert_eq!(Realpath::new(b"/", b"/", &mut [0])?.finish(), Err(ENAMETOOLONG));
    Ok(())
}

#[test]
fn expanded_target_and_suffix_share_the_existing_bound() -> Result<(), c_int> {
    let mut out = [0; PATH_MAX];
    let mut adapter = Realpath::new(b"link/x", b"/", &mut out)?;
    let parent = append_next(&mut adapter)?;
    assert_eq!(adapter.splice_target(parent, b""), Err(ENAMETOOLONG));
    assert_eq!(adapter.splice_target(parent, &[b'x'; PATH_MAX]), Err(ENAMETOOLONG));
    assert_eq!(adapter.splice_target(parent, &[b'x'; PATH_MAX - 2]), Err(ENAMETOOLONG));
    adapter.splice_target(parent, &[b'x'; PATH_MAX - 3])?;
    assert_eq!(adapter.pending_len, PATH_MAX - 1);
    assert_eq!(&adapter.pending[PATH_MAX - 3..PATH_MAX - 1], b"/x");
    Ok(())
}

#[test]
fn cwd_seeding_and_root_parent_behavior_are_unchanged() -> Result<(), c_int> {
    let mut out = [0; PATH_MAX];
    let mut adapter = Realpath::new(b".", b"/dir/../cwd//", &mut out)?;
    assert_eq!(adapter.next_component(), Some(Component::Current));
    assert_eq!(adapter.next_component(), None);
    assert_eq!(adapter.finish(), Ok(4));
    assert_eq!(&out[..5], b"/cwd\0");
    let mut adapter = Realpath::new(b"/../../", b"/unused", &mut out)?;
    for _ in 0..2 {
        assert_eq!(adapter.next_component(), Some(Component::Parent));
        adapter.parent()?;
    }
    assert_eq!(adapter.next_component(), None);
    assert_eq!(adapter.finish(), Ok(1));
    assert_eq!(&out[..2], b"/\0");
    Ok(())
}

/// Canonicalizes `path` against `cwd`, returning the resulting bytes.
fn run(path: &str, cwd: &str) -> Option<Vec<u8>> {
    let mut out: [u8; 1024] = [0; 1024];
    let len: usize = canonicalize(path.as_bytes(), cwd.as_bytes(), &mut out)?;
    Some(out[..len].to_vec())
}

#[test]
fn test_absolute_dotdot() {
    assert_eq!(run("/tmp/../tmp/hello.txt", "/").as_deref(), Some(&b"/tmp/hello.txt"[..]));
}

#[test]
fn test_relative_against_cwd() {
    assert_eq!(run("a/b", "/home/u").as_deref(), Some(&b"/home/u/a/b"[..]));
}

#[test]
fn test_dot_and_double_slash() {
    assert_eq!(run("/a/./b//c", "/").as_deref(), Some(&b"/a/b/c"[..]));
}

#[test]
fn test_dotdot_past_root_stays_root() {
    assert_eq!(run("/a/../../x", "/").as_deref(), Some(&b"/x"[..]));
}

#[test]
fn test_root() {
    assert_eq!(run("/", "/").as_deref(), Some(&b"/"[..]));
    assert_eq!(run("/..", "/").as_deref(), Some(&b"/"[..]));
}

#[test]
fn test_trailing_slash_and_dot() {
    assert_eq!(run("/a/b/.", "/").as_deref(), Some(&b"/a/b"[..]));
    assert_eq!(run("/a/b/", "/").as_deref(), Some(&b"/a/b"[..]));
}

#[test]
fn test_overflow_returns_none() {
    let mut out: [u8; 8] = [0; 8];
    assert_eq!(canonicalize(b"/aaaaaaaaaaaaaaaa", b"/", &mut out), None);
}
