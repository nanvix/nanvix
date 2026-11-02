// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::executor::combine_args_env;

#[cfg(windows)]
mod gateway_input_framing {
    use super::*;

    fn collect_frame(payload: &[u8]) -> Vec<u8> {
        let frame: Option<GatewayInputFrame<'_>> =
            encode_gateway_input_frame(payload).expect("gateway input frame encoding failed");
        let Some((header, payload)) = frame else {
            return Vec::new();
        };

        let mut bytes: Vec<u8> = Vec::with_capacity(header.len() + payload.len());
        bytes.extend_from_slice(&header);
        bytes.extend_from_slice(payload);
        bytes
    }

    #[test]
    fn payload_frame_contains_little_endian_length_and_payload() {
        let encoded: Vec<u8> = collect_frame(b"abc");
        assert_eq!(
            encoded,
            vec![3, 0, 0, 0, b'a', b'b', b'c'],
            "payload frame wire format changed"
        );
    }

    #[test]
    fn empty_payload_is_suppressed() {
        let encoded: Option<GatewayInputFrame<'_>> =
            encode_gateway_input_frame(&[]).expect("empty payload encoding failed");
        assert!(encoded.is_none(), "empty payload must not encode as EOF");
    }

    #[test]
    fn eof_frame_is_zero_length_record() {
        let encoded: [u8; GATEWAY_INPUT_FRAME_HEADER_LEN] = encode_gateway_input_eof_frame();
        assert_eq!(encoded, [0, 0, 0, 0], "EOF frame wire format changed");
    }

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn payload_larger_than_u32_is_rejected() {
        let maximum_len: usize = usize::try_from(u32::MAX).expect("usize should be 64-bit");
        let maximum_header: Option<[u8; GATEWAY_INPUT_FRAME_HEADER_LEN]> =
            encode_gateway_input_frame_header(maximum_len)
                .expect("maximum payload length should be accepted");
        assert_eq!(
            maximum_header,
            Some([u8::MAX; GATEWAY_INPUT_FRAME_HEADER_LEN]),
            "u32::MAX payload length should remain encodable"
        );

        let oversized_len: usize =
            usize::try_from(u64::from(u32::MAX) + 1).expect("usize should be 64-bit");
        let error: ::std::io::Error = encode_gateway_input_frame_header(oversized_len)
            .expect_err("oversized payload length should be rejected");
        assert_eq!(
            error.kind(),
            ::std::io::ErrorKind::InvalidInput,
            "oversized payload should return InvalidInput"
        );
    }

    #[test]
    fn sequential_payload_frames_preserve_record_order() {
        let payloads: [&[u8]; 2] = [b"a", b"bc"];
        let mut encoded: Vec<u8> = Vec::new();
        for payload in payloads {
            encoded.extend_from_slice(&collect_frame(payload));
        }

        assert_eq!(
            encoded,
            vec![1, 0, 0, 0, b'a', 2, 0, 0, 0, b'b', b'c'],
            "sequential payload frames changed order or boundaries"
        );
    }
}

/// Helper that builds a `UserVmArgs` with the given program_args and program_env, then
/// calls `combined_program_args()`.
fn combine(args: Option<&str>, env: Option<&str>) -> String {
    let uva: UserVmArgs =
        UserVmArgs::new("t", "a", "p", args, env).expect("UserVmArgs::new failed");
    uva.combined_program_args()
}

#[test]
fn combined_no_args_no_env() {
    assert_eq!(combine(None, None), "");
}

#[test]
fn combined_args_only() {
    assert_eq!(combine(Some("arg1 arg2"), None), "arg1 arg2");
}

#[test]
fn combined_env_only() {
    assert_eq!(combine(None, Some("VAR=x")), ";VAR=x");
}

#[test]
fn combined_args_and_env() {
    assert_eq!(combine(Some("arg1"), Some("VAR=x")), "arg1;VAR=x");
}

#[test]
fn combined_escapes_semicolons_in_args() {
    // A literal `;` in args must be escaped to `\;` so split_cmdline() treats it as data.
    assert_eq!(combine(Some("a;b"), Some("VAR=x")), "a\\;b;VAR=x");
}

#[test]
fn combined_escapes_semicolons_in_env() {
    // A literal `;` in env must be escaped to `\;` so split_cmdline() treats it as data.
    assert_eq!(combine(Some("arg1"), Some("PATH=a;b")), "arg1;PATH=a\\;b");
}

#[test]
fn combined_escapes_semicolons_even_without_env() {
    // Semicolons in args are always escaped so split_cmdline() treats them as data.
    assert_eq!(combine(Some("a;b"), None), "a\\;b");
}

#[test]
fn combined_roundtrip_with_split_cmdline() {
    // Verify the combined string round-trips through the kernel's split_cmdline().
    let combined: String = combine(Some("path/to;file arg2"), Some("FOO=bar BAZ=qux"));
    let mut buf: Vec<u8> = combined.into_bytes();
    let (args, env) = ::cmdline::split_cmdline(&mut buf);
    assert_eq!(args, "path/to;file arg2");
    assert_eq!(env, "FOO=bar BAZ=qux");
}

#[test]
fn combined_roundtrip_semicolon_in_env_value() {
    // Verify that a literal `;` inside an env value survives the round-trip
    // because it is escaped to `\;` and split_cmdline() unescapes it.
    let combined: String = combine(Some("hello"), Some("PATH=a;b"));
    let mut buf: Vec<u8> = combined.into_bytes();
    let (args, env) = ::cmdline::split_cmdline(&mut buf);
    assert_eq!(args, "hello");
    assert_eq!(env, "PATH=a;b");
}

#[test]
fn combined_empty_env_string_treated_as_absent() {
    // `Some("")` must behave the same as `None` — no trailing `;`.
    assert_eq!(combine(Some("arg1"), Some("")), "arg1");
}

#[test]
fn combined_empty_args_string_treated_as_absent() {
    // `Some("")` must behave the same as `None` — no leading `;` when only env is set.
    assert_eq!(combine(Some(""), Some("VAR=x")), ";VAR=x");
}

#[test]
fn combined_program_args_matches_free_function() {
    // The free function should produce identical results to the UserVmArgs method.
    assert_eq!(combine_args_env(Some("arg1"), Some("VAR=x")), combine(Some("arg1"), Some("VAR=x")));
    assert_eq!(combine_args_env(None, None), combine(None, None));
    assert_eq!(
        combine_args_env(Some("a;b"), Some("PATH=a;b")),
        combine(Some("a;b"), Some("PATH=a;b"))
    );
}
