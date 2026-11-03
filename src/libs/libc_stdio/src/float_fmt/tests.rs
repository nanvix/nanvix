// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::std::{
    format,
    string::String,
    vec::Vec,
};

/// Collects formatted bytes into a string for assertions.
struct VecWriter {
    data: Vec<u8>,
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

/// Formats `value` with the given specifier, flags, width and precision.
fn run(
    value: f64,
    spec: u8,
    flags: &FormatFlags,
    width: usize,
    has_precision: bool,
    precision: usize,
) -> String {
    let mut writer: VecWriter = VecWriter { data: Vec::new() };
    format_float(&mut writer, value, spec, flags, width, has_precision, precision)
        .expect("format_float writes to an in-memory buffer");
    String::from_utf8(writer.data).expect("ascii output")
}

/// Plain flags (no `-`, `+`, space, `0` or `#`).
fn plain() -> FormatFlags {
    FormatFlags {
        left_align: false,
        force_sign: false,
        space_sign: false,
        zero_pad: false,
        alternate: false,
    }
}

#[test]
fn fixed_matches_std_default_precision() {
    let cases: [f64; 9] = [
        0.0,
        1.0,
        1.5,
        2.5,
        3.40192,
        1234.5,
        0.1,
        0.0009765625,
        9999.99995,
    ];
    for &v in &cases {
        assert_eq!(run(v, b'f', &plain(), 0, false, 0), format!("{:.6}", v), "value {v}");
    }
}

#[test]
fn fixed_matches_std_various_precisions() {
    let values: [f64; 6] = [0.0, 1.0, 3.40192837465019, 2.5, 0.125, 1234.875];
    for &v in &values {
        for p in 0..=15usize {
            assert_eq!(
                run(v, b'f', &plain(), 0, true, p),
                format!("{:.*}", p, v),
                "value {v} precision {p}"
            );
        }
    }
}

#[test]
fn fixed_negative_and_integers() {
    assert_eq!(run(-3.5, b'f', &plain(), 0, true, 2), "-3.50");
    assert_eq!(run(-0.0, b'f', &plain(), 0, true, 1), "-0.0");
    assert_eq!(run(2.0, b'f', &plain(), 0, true, 0), "2");
    assert_eq!(run(2.0, b'f', &plain(), 0, false, 0), "2.000000");
}

#[test]
fn fixed_rounding_half_to_even() {
    // 0.5 and 2.5 at precision 0 round to the nearest even integer.
    assert_eq!(run(0.5, b'f', &plain(), 0, true, 0), "0");
    assert_eq!(run(1.5, b'f', &plain(), 0, true, 0), "2");
    assert_eq!(run(2.5, b'f', &plain(), 0, true, 0), "2");
    assert_eq!(run(3.5, b'f', &plain(), 0, true, 0), "4");
}

#[test]
fn scientific_matches_std() {
    let values: [f64; 7] = [0.0, 1.0, 1234.5, 0.000123, 6.022e23, 9.999e9, 5.0e-7];
    for &v in &values {
        for p in 0..=12usize {
            assert_eq!(
                run(v, b'e', &plain(), 0, true, p),
                c_scientific(v, p),
                "value {v} precision {p}"
            );
        }
    }
}

/// Renders `value` in C `%e` style by taking the mantissa from Rust's formatter and rewriting the
/// exponent in C form (explicit sign, at least two digits). Rust uses a minimal exponent without a
/// sign for non-negative exponents, so the suffix cannot be compared directly.
fn c_scientific(value: f64, precision: usize) -> String {
    let std: String = format!("{:.*e}", precision, value);
    let (mantissa, exp) = std.split_once('e').expect("scientific output");
    let exp: i32 = exp.parse().expect("exponent");
    format!("{}e{}{:02}", mantissa, if exp < 0 { '-' } else { '+' }, exp.abs())
}

#[test]
fn scientific_exponent_has_two_digits() {
    assert_eq!(run(1.0, b'e', &plain(), 0, true, 2), "1.00e+00");
    assert_eq!(run(1234.5, b'e', &plain(), 0, true, 4), "1.2345e+03");
    assert_eq!(run(0.0009765625, b'e', &plain(), 0, true, 3), "9.766e-04");
}

#[test]
fn scientific_uppercase() {
    assert_eq!(run(1234.5, b'E', &plain(), 0, true, 2), "1.23E+03");
    assert_eq!(run(f64::INFINITY, b'E', &plain(), 0, false, 0), "INF");
}

#[test]
fn general_strips_trailing_zeros() {
    assert_eq!(run(1.0, b'g', &plain(), 0, false, 0), "1");
    assert_eq!(run(100000.0, b'g', &plain(), 0, false, 0), "100000");
    assert_eq!(run(0.0001, b'g', &plain(), 0, false, 0), "0.0001");
    // Beyond 1e-4 the conversion switches to scientific notation.
    assert_eq!(run(0.00001, b'g', &plain(), 0, false, 0), "1e-05");
    assert_eq!(run(1234567.0, b'g', &plain(), 0, false, 0), "1.23457e+06");
}

#[test]
fn general_alternate_keeps_zeros() {
    let mut flags: FormatFlags = plain();
    flags.alternate = true;
    assert_eq!(run(1.0, b'g', &flags, 0, true, 6), "1.00000");
}

#[test]
fn special_values() {
    assert_eq!(run(f64::NAN, b'f', &plain(), 0, false, 0), "nan");
    assert_eq!(run(f64::INFINITY, b'f', &plain(), 0, false, 0), "inf");
    assert_eq!(run(f64::NEG_INFINITY, b'f', &plain(), 0, false, 0), "-inf");
}

#[test]
fn width_and_alignment() {
    assert_eq!(run(3.5, b'f', &plain(), 10, true, 2), "      3.50");

    let mut left: FormatFlags = plain();
    left.left_align = true;
    assert_eq!(run(3.5, b'f', &left, 10, true, 2), "3.50      ");

    let mut zero: FormatFlags = plain();
    zero.zero_pad = true;
    assert_eq!(run(3.5, b'f', &zero, 10, true, 2), "0000003.50");

    let mut zero_neg: FormatFlags = plain();
    zero_neg.zero_pad = true;
    assert_eq!(run(-3.5, b'f', &zero_neg, 10, true, 2), "-000003.50");
}

#[test]
fn sign_flags() {
    let mut plus: FormatFlags = plain();
    plus.force_sign = true;
    assert_eq!(run(3.5, b'f', &plus, 0, true, 1), "+3.5");

    let mut space: FormatFlags = plain();
    space.space_sign = true;
    assert_eq!(run(3.5, b'f', &space, 0, true, 1), " 3.5");
}

#[test]
fn alternate_forces_point() {
    let mut flags: FormatFlags = plain();
    flags.alternate = true;
    assert_eq!(run(4.0, b'f', &flags, 0, true, 0), "4.");
    assert_eq!(run(4.0, b'e', &flags, 0, true, 0), "4.e+00");
}

#[test]
fn large_precision_is_bounded() {
    // A precision far beyond the accumulator's range is clamped to the supported number of
    // significant digits, so the body stays well within `buf` instead of being truncated at its
    // end (which an unclamped precision would do, leaving the length pinned at `BODY_MAX`).
    for spec in *b"feg" {
        let out: String = run(1.0, spec, &plain(), 0, true, 1000);
        assert!(out.len() < BODY_MAX, "{} body length {} is not bounded", spec as char, out.len());
    }
}
