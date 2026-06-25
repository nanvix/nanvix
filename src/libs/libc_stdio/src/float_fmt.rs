// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
//! Floating-point conversion for the `printf` family.
//!
//! This module implements the `%f`/`%F`, `%e`/`%E` and `%g`/`%G` conversions. The in-tree C library
//! historically supported only integer and string conversions, so a `double` specifier was emitted
//! literally (e.g. `printf("%f", x)` printed the text `%f`). The digit generation here is performed
//! entirely with integer arithmetic — every finite `double` equals `mantissa * 2^exp2` with a 53-bit
//! integer mantissa, and the scaled value `mantissa * 2^exp2 * 10^k` is evaluated in a 128-bit
//! fixed-point accumulator and rounded to nearest, ties to even. The result is therefore correctly
//! rounded for the range of magnitudes and precisions that fit in that accumulator, which covers the
//! conversions used in practice (up to ~38 significant digits and magnitudes up to ~1e38). Requested
//! precisions beyond that are clamped to the supported number of significant digits, and larger
//! magnitudes degrade to a bounded best-effort rather than producing incorrect digits for the common
//! case; neither occurs for the workloads this library targets.
//==================================================================================================

//==================================================================================================
// Imports
//==================================================================================================

use crate::format_engine::{
    write_padding,
    FormatFlags,
    WriteTarget,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Default precision applied when a conversion does not specify one.
const DEFAULT_PRECISION: usize = 6;

/// Maximum number of significant decimal digits that fit in the 128-bit accumulator.
const MAX_SIG_DIGITS: usize = 38;

/// Size of the stack buffer used to assemble the numeric body (sign and field padding excluded).
const BODY_MAX: usize = 512;

//==================================================================================================
// Public Entry Point
//==================================================================================================

/// Formats a floating-point conversion (`spec` is one of `f F e E g G`) and writes it to `writer`.
///
/// The caller supplies the parsed flags, field `width`, and precision (with `has_precision`
/// indicating whether one was given). All padding and sign handling required by the field width is
/// performed here.
pub(crate) fn format_float<W: WriteTarget>(
    writer: &mut W,
    value: f64,
    spec: u8,
    flags: &FormatFlags,
    width: usize,
    has_precision: bool,
    precision: usize,
) -> Result<(), ()> {
    let upper: bool = spec.is_ascii_uppercase();
    let lower: u8 = spec | 0x20;
    let neg: bool = value.is_sign_negative();
    let sign: Option<u8> = sign_char(neg, flags);

    // Non-finite values format as text and are never zero-padded.
    if value.is_nan() {
        let text: &[u8] = if upper { b"NAN" } else { b"nan" };
        return emit(writer, sign, text.len(), flags, width, false, |w| w.write_bytes(text));
    }
    if value.is_infinite() {
        let text: &[u8] = if upper { b"INF" } else { b"inf" };
        return emit(writer, sign, text.len(), flags, width, false, |w| w.write_bytes(text));
    }

    let (_, mantissa, exp2) = decompose_f64(value);

    // Resolve the effective precision and clamp it to the number of significant digits the integer
    // accumulator can represent. This bounds the digits written to `buf` so that an arbitrarily large
    // requested precision cannot overflow it (the output is a bounded best-effort in that case).
    let precision: usize = if has_precision {
        precision
    } else {
        DEFAULT_PRECISION
    };
    let precision: usize = precision.min(MAX_SIG_DIGITS);

    let mut buf: [u8; BODY_MAX] = [0u8; BODY_MAX];
    let len: usize = match lower {
        b'f' => build_fixed(mantissa, exp2, precision, flags.alternate, &mut buf),
        b'e' => build_scientific(mantissa, exp2, precision, flags.alternate, upper, &mut buf),
        // `g`/`G`
        _ => build_general(mantissa, exp2, precision, flags.alternate, upper, &mut buf),
    };

    emit(writer, sign, len, flags, width, true, |w| w.write_bytes(&buf[..len]))
}

//==================================================================================================
// Field Emission
//==================================================================================================

/// Returns the sign character to print for a value, honoring the `+` and space flags.
fn sign_char(neg: bool, flags: &FormatFlags) -> Option<u8> {
    if neg {
        Some(b'-')
    } else if flags.force_sign {
        Some(b'+')
    } else if flags.space_sign {
        Some(b' ')
    } else {
        Option::None
    }
}

/// Emits a sign and body within a field of the requested width, applying the alignment flags.
///
/// `body_len` is the number of bytes the `body` closure writes (the sign is accounted separately).
/// `zero_pad_allowed` disables `0`-padding for conversions where it does not apply (the non-finite
/// `inf`/`nan` spellings).
fn emit<W, F>(
    writer: &mut W,
    sign: Option<u8>,
    body_len: usize,
    flags: &FormatFlags,
    width: usize,
    zero_pad_allowed: bool,
    mut body: F,
) -> Result<(), ()>
where
    W: WriteTarget,
    F: FnMut(&mut W) -> Result<(), ()>,
{
    let sign_len: usize = usize::from(sign.is_some());
    let content: usize = sign_len + body_len;
    let pad: usize = width.saturating_sub(content);

    if flags.left_align {
        if let Some(s) = sign {
            writer.write_byte(s)?;
        }
        body(writer)?;
        write_padding(writer, b' ', pad)?;
    } else if zero_pad_allowed && flags.zero_pad {
        if let Some(s) = sign {
            writer.write_byte(s)?;
        }
        write_padding(writer, b'0', pad)?;
        body(writer)?;
    } else {
        write_padding(writer, b' ', pad)?;
        if let Some(s) = sign {
            writer.write_byte(s)?;
        }
        body(writer)?;
    }
    Ok(())
}

//==================================================================================================
// Float Decomposition and Fixed-Point Scaling
//==================================================================================================

/// Decomposes a finite `f64` into `(sign, mantissa, exp2)` with magnitude `mantissa * 2^exp2`.
///
/// The mantissa is the full 53-bit significand, including the implicit leading bit for normal
/// values; subnormals carry their stored fraction with the fixed minimum exponent.
fn decompose_f64(value: f64) -> (bool, u64, i32) {
    let bits: u64 = value.to_bits();
    let sign: bool = (bits >> 63) != 0;
    let exp_field: u32 = ((bits >> 52) & 0x7ff) as u32;
    let frac: u64 = bits & 0x000f_ffff_ffff_ffff;
    if exp_field == 0 {
        // Subnormal (or zero): no implicit leading bit, exponent fixed at the smallest normal scale.
        (sign, frac, -1074)
    } else {
        // Normal: restore the implicit leading bit. The unbiased exponent is `exp_field - 1023`, and
        // the significand is scaled by `2^-52`, so `exp2 = exp_field - 1023 - 52`.
        (sign, frac | (1u64 << 52), exp_field as i32 - 1075)
    }
}

/// Returns `5^n` as a `u128`, or [`None`] on overflow.
fn pow5(n: u32) -> Option<u128> {
    let mut result: u128 = 1;
    for _ in 0..n {
        result = result.checked_mul(5)?;
    }
    Some(result)
}

/// Shifts `value` left by `shift`, returning [`None`] when significant bits would be lost.
fn shl_checked(value: u128, shift: u32) -> Option<u128> {
    if value == 0 {
        return Some(0);
    }
    if shift >= 128 || value.leading_zeros() < shift {
        return Option::None;
    }
    Some(value << shift)
}

/// Computes `round(mantissa * 2^exp2 * 10^pow10)` to nearest, ties to even, as a `u128`.
///
/// Returns [`None`] when the exact intermediate does not fit in the 128-bit accumulator.
fn round_scaled(mantissa: u64, exp2: i32, pow10: i32) -> Option<u128> {
    // value * 10^pow10 = mantissa * 2^(exp2 + pow10) * 5^pow10.
    let pow2: i32 = exp2 + pow10;
    let pow5_exp: i32 = pow10;

    let mut num: u128 = mantissa as u128;
    let mut den: u128 = 1;

    if pow5_exp >= 0 {
        num = num.checked_mul(pow5(pow5_exp as u32)?)?;
    } else {
        den = den.checked_mul(pow5((-pow5_exp) as u32)?)?;
    }

    if pow2 >= 0 {
        num = shl_checked(num, pow2 as u32)?;
    } else {
        den = shl_checked(den, (-pow2) as u32)?;
    }

    Some(round_div(num, den))
}

/// Returns `round(num / den)` to nearest, ties to even. `den` must be non-zero.
fn round_div(num: u128, den: u128) -> u128 {
    let quotient: u128 = num / den;
    let remainder: u128 = num % den;
    // Compare the remainder against half of the divisor without risking overflow: `remainder` and
    // `den - remainder` straddle the half-way point.
    let complement: u128 = den - remainder;
    if remainder > complement {
        quotient + 1
    } else if remainder < complement {
        quotient
    } else {
        // Exactly half-way: round to even.
        quotient + (quotient & 1)
    }
}

/// Writes the decimal digits of `value` (most-significant first) into `out`, returning the count.
fn u128_to_digits(value: u128, out: &mut [u8]) -> usize {
    if value == 0 {
        out[0] = b'0';
        return 1;
    }
    let mut tmp: [u8; 40] = [0u8; 40];
    let mut count: usize = 0;
    let mut v: u128 = value;
    while v > 0 {
        tmp[count] = b'0' + (v % 10) as u8;
        v /= 10;
        count += 1;
    }
    for i in 0..count {
        out[i] = tmp[count - 1 - i];
    }
    count
}

//==================================================================================================
// Fixed Notation (%f)
//==================================================================================================

/// Assembles the body of a `%f` conversion into `buf`, returning the number of bytes written.
fn build_fixed(
    mantissa: u64,
    exp2: i32,
    precision: usize,
    alternate: bool,
    buf: &mut [u8],
) -> usize {
    match round_scaled(mantissa, exp2, precision as i32) {
        Some(scaled) => {
            let mut digits: [u8; 40] = [0u8; 40];
            let count: usize = u128_to_digits(scaled, &mut digits);
            let mut pos: usize = 0;

            if precision == 0 {
                // No fractional digits: the whole number is the integer part.
                for &d in &digits[..count] {
                    push(buf, &mut pos, d);
                }
                if alternate {
                    push(buf, &mut pos, b'.');
                }
            } else if count > precision {
                // The integer part is the leading `count - precision` digits.
                let int_len: usize = count - precision;
                for &d in &digits[..int_len] {
                    push(buf, &mut pos, d);
                }
                push(buf, &mut pos, b'.');
                for &d in &digits[int_len..count] {
                    push(buf, &mut pos, d);
                }
            } else {
                // The value is below 1: a single `0`, then leading fractional zeros, then the digits.
                push(buf, &mut pos, b'0');
                push(buf, &mut pos, b'.');
                for _ in 0..(precision - count) {
                    push(buf, &mut pos, b'0');
                }
                for &d in &digits[..count] {
                    push(buf, &mut pos, d);
                }
            }
            pos
        },
        // Magnitude/precision beyond the 128-bit accumulator (not reached for practical inputs).
        Option::None => build_fixed_fallback(mantissa, exp2, precision, alternate, buf),
    }
}

/// Best-effort `%f` for magnitudes/precisions that overflow the exact path: prints the integer part
/// (correct for magnitudes below `2^128`) with a zero fraction.
fn build_fixed_fallback(
    mantissa: u64,
    exp2: i32,
    precision: usize,
    alternate: bool,
    buf: &mut [u8],
) -> usize {
    let integer: u128 = if exp2 >= 0 {
        shl_checked(mantissa as u128, exp2 as u32).unwrap_or(u128::MAX)
    } else {
        let shift: u32 = (-exp2) as u32;
        if shift >= 128 {
            0
        } else {
            (mantissa as u128) >> shift
        }
    };

    let mut digits: [u8; 40] = [0u8; 40];
    let count: usize = u128_to_digits(integer, &mut digits);
    let mut pos: usize = 0;
    for &d in &digits[..count] {
        push(buf, &mut pos, d);
    }
    if precision > 0 {
        push(buf, &mut pos, b'.');
        for _ in 0..precision {
            push(buf, &mut pos, b'0');
        }
    } else if alternate {
        push(buf, &mut pos, b'.');
    }
    pos
}

//==================================================================================================
// Scientific Notation (%e)
//==================================================================================================

/// Computes the `sig` leading significant digits (most-significant first) and the decimal exponent
/// `x` such that the value is `d.ddd... * 10^x`. `mantissa` must be non-zero.
fn scientific_digits(mantissa: u64, exp2: i32, sig: usize, digits: &mut [u8; 40]) -> i32 {
    let sig: usize = sig.clamp(1, MAX_SIG_DIGITS);

    // Estimate the decimal exponent from the binary one: log10(value) ≈ log2(value) * log10(2), with
    // log10(2) ≈ 1233 / 4096. The estimate is within one or two of the true value and is corrected
    // below by checking the digit count of the scaled integer.
    let msb: i32 = 63 - mantissa.leading_zeros() as i32;
    let approx_log2: i32 = exp2 + msb;
    let mut x: i32 = ((approx_log2 as i64 * 1233) >> 12) as i32;

    let mut guard: u32 = 0;
    loop {
        guard += 1;
        let pow: i32 = (sig as i32 - 1) - x;
        let scaled: u128 = round_scaled(mantissa, exp2, pow).unwrap_or(0);
        let count: usize = u128_to_digits(scaled, digits);
        if count == sig {
            return x;
        }
        if count > sig {
            x += (count - sig) as i32;
        } else {
            x -= (sig - count) as i32;
        }
        if guard > 8 {
            // Convergence guard for inputs at the edge of the accumulator's range.
            return x;
        }
    }
}

/// Assembles the body of a `%e` conversion into `buf`, returning the number of bytes written.
fn build_scientific(
    mantissa: u64,
    exp2: i32,
    precision: usize,
    alternate: bool,
    upper: bool,
    buf: &mut [u8],
) -> usize {
    let sig: usize = (precision + 1).clamp(1, MAX_SIG_DIGITS);
    let mut digits: [u8; 40] = [0u8; 40];
    let exponent: i32 = if mantissa == 0 {
        digits[0] = b'0';
        0
    } else {
        scientific_digits(mantissa, exp2, sig, &mut digits)
    };
    // Only one digit is meaningful for zero; otherwise the buffer holds `sig` significant digits.
    let available: usize = if mantissa == 0 { 1 } else { sig };

    let mut pos: usize = 0;
    push(buf, &mut pos, digits[0]);
    if precision > 0 || alternate {
        push(buf, &mut pos, b'.');
    }
    // The fractional part has `precision` digits: the significant digits after the leading one
    // (`digits[1..available]`, capped at `precision`), followed by zero padding.
    let significant: usize = (available - 1).min(precision);
    for &digit in &digits[1..1 + significant] {
        push(buf, &mut pos, digit);
    }
    for _ in significant..precision {
        push(buf, &mut pos, b'0');
    }
    push_exponent(buf, &mut pos, exponent, upper);
    pos
}

/// Appends the exponent suffix (`e±dd`, at least two exponent digits) of a scientific conversion.
fn push_exponent(buf: &mut [u8], pos: &mut usize, exponent: i32, upper: bool) {
    push(buf, pos, if upper { b'E' } else { b'e' });
    let negative: bool = exponent < 0;
    push(buf, pos, if negative { b'-' } else { b'+' });

    let mut magnitude: u32 = exponent.unsigned_abs();
    let mut tmp: [u8; 8] = [0u8; 8];
    let mut count: usize = 0;
    loop {
        tmp[count] = b'0' + (magnitude % 10) as u8;
        magnitude /= 10;
        count += 1;
        if magnitude == 0 {
            break;
        }
    }
    while count < 2 {
        tmp[count] = b'0';
        count += 1;
    }
    for i in (0..count).rev() {
        push(buf, pos, tmp[i]);
    }
}

//==================================================================================================
// General Notation (%g)
//==================================================================================================

/// Assembles the body of a `%g` conversion into `buf`, returning the number of bytes written.
fn build_general(
    mantissa: u64,
    exp2: i32,
    precision: usize,
    alternate: bool,
    upper: bool,
    buf: &mut [u8],
) -> usize {
    // A precision of 0 is treated as 1 significant digit, per the conversion's definition.
    let sig: usize = if precision == 0 { 1 } else { precision };

    // Determine the exponent the value would have in scientific notation.
    let exponent: i32 = if mantissa == 0 {
        0
    } else {
        let mut digits: [u8; 40] = [0u8; 40];
        scientific_digits(mantissa, exp2, sig, &mut digits)
    };

    let len: usize = if exponent >= -4 && exponent < sig as i32 {
        // Fixed notation with precision chosen so that `sig` significant digits are shown.
        let fixed_precision: usize = (sig as i32 - 1 - exponent).max(0) as usize;
        build_fixed(mantissa, exp2, fixed_precision, alternate, buf)
    } else {
        build_scientific(mantissa, exp2, sig - 1, alternate, upper, buf)
    };

    if alternate {
        // The alternate form keeps trailing zeros and the decimal point.
        len
    } else {
        strip_trailing_zeros(buf, len)
    }
}

/// Removes trailing fractional zeros (and a now-trailing decimal point) from a `%g` body, leaving
/// any exponent suffix intact. Returns the new length.
fn strip_trailing_zeros(buf: &mut [u8], len: usize) -> usize {
    // Locate an exponent marker, if any; only the mantissa is trimmed.
    let exp_pos: Option<usize> = buf[..len].iter().position(|&c| c == b'e' || c == b'E');
    let mantissa_end: usize = exp_pos.unwrap_or(len);

    // Trimming only applies when a decimal point is present.
    if !buf[..mantissa_end].contains(&b'.') {
        return len;
    }

    let mut end: usize = mantissa_end;
    while end > 0 && buf[end - 1] == b'0' {
        end -= 1;
    }
    if end > 0 && buf[end - 1] == b'.' {
        end -= 1;
    }

    match exp_pos {
        None => end,
        Some(start) => {
            // Shift the exponent suffix down to close the gap left by the trimmed zeros.
            let suffix_len: usize = len - start;
            for i in 0..suffix_len {
                buf[end + i] = buf[start + i];
            }
            end + suffix_len
        },
    }
}

//==================================================================================================
// Helpers
//==================================================================================================

/// Writes `byte` at `*pos` in `buf` and advances `pos`. A write that would fall past the end of
/// `buf` is silently dropped and leaves `pos` unchanged.
fn push(buf: &mut [u8], pos: &mut usize, byte: u8) {
    if *pos < buf.len() {
        buf[*pos] = byte;
        *pos += 1;
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
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
            assert!(
                out.len() < BODY_MAX,
                "{} body length {} is not bounded",
                spec as char,
                out.len()
            );
        }
    }
}
