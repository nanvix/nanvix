// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Direct conversion of C floating-point subject sequences to IEEE-754 binary128.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    float_lex::{
        digit_val,
        hex_digit_val,
        is_digit,
        is_hex_digit,
        is_whitespace,
        match_keyword,
    },
    set_errno,
};
use ::alloc::{
    vec,
    vec::Vec,
};
use ::core::cmp::Ordering;
use ::sysapi::{
    errno::ERANGE,
    ffi::c_char,
};

//==================================================================================================
// Constants
//==================================================================================================

const BINARY128_FRACTION_BITS: usize = 112;
const BINARY128_PRECISION: usize = BINARY128_FRACTION_BITS + 1;
const BINARY128_MAX_EXPONENT: i64 = 16_383;
const BINARY128_MIN_NORMAL_EXPONENT: i64 = -16_382;
const BINARY128_MIN_SUBNORMAL_EXPONENT: i64 = -16_494;
const MAX_DIRECT_DECIMAL_ORDER: i64 = 6_000;

//==================================================================================================
// Structures
//==================================================================================================

/// Bit representation of an IEEE-754 binary128 value in the AAPCS64 memory byte order.
///
/// The low 64 bits contain the low fraction bits and the high 64 bits contain the sign, exponent,
/// and high fraction bits. Returning this C-compatible pair lets the AArch64 ABI shim move it to
/// `q0` without representing `long double` in Rust.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binary128 {
    /// Low 64 bits of the IEEE-754 encoding.
    pub low: u64,
    /// High 64 bits of the IEEE-754 encoding.
    pub high: u64,
}

impl Binary128 {
    fn zero(negative: bool) -> Self {
        Self {
            low: 0,
            high: if negative { 1_u64 << 63 } else { 0 },
        }
    }

    fn infinity(negative: bool) -> Self {
        Self {
            low: 0,
            high: (if negative { 1_u64 << 63 } else { 0 }) | (0x7fff_u64 << 48),
        }
    }

    fn nan(negative: bool, payload: u128) -> Self {
        let quiet_bit: u128 = 1_u128 << (BINARY128_FRACTION_BITS - 1);
        Self::from_parts(negative, 0x7fff, quiet_bit | (payload & (quiet_bit - 1)))
    }

    fn from_parts(negative: bool, exponent: u16, fraction: u128) -> Self {
        let fraction_bytes: [u8; 16] = fraction.to_le_bytes();
        let low: u64 = u64::from_le_bytes([
            fraction_bytes[0],
            fraction_bytes[1],
            fraction_bytes[2],
            fraction_bytes[3],
            fraction_bytes[4],
            fraction_bytes[5],
            fraction_bytes[6],
            fraction_bytes[7],
        ]);
        let high_fraction: u64 = u64::from_le_bytes([
            fraction_bytes[8],
            fraction_bytes[9],
            fraction_bytes[10],
            fraction_bytes[11],
            fraction_bytes[12],
            fraction_bytes[13],
            fraction_bytes[14],
            fraction_bytes[15],
        ]) & 0x0000_ffff_ffff_ffff;
        Self {
            low,
            high: (if negative { 1_u64 << 63 } else { 0 })
                | (u64::from(exponent) << 48)
                | high_fraction,
        }
    }

    #[cfg(test)]
    fn exponent(self) -> u16 {
        let bits: u64 = (self.high >> 48) & 0x7fff;
        u16::from_le_bytes([bits.to_le_bytes()[0], bits.to_le_bytes()[1]])
    }
}

/// A non-negative arbitrary-precision integer with 32-bit little-endian limbs.
#[derive(Clone, Debug, Eq, PartialEq)]
struct BigUint {
    limbs: Vec<u32>,
}

impl BigUint {
    fn zero() -> Self {
        Self { limbs: Vec::new() }
    }

    fn one() -> Self {
        Self { limbs: vec![1] }
    }

    fn is_zero(&self) -> bool {
        self.limbs.is_empty()
    }

    fn normalize(&mut self) {
        while self.limbs.last() == Some(&0) {
            self.limbs.pop();
        }
    }

    fn low_u32(value: u64) -> u32 {
        let bytes: [u8; 8] = value.to_le_bytes();
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    fn add_small(&mut self, value: u32) {
        let mut carry: u64 = u64::from(value);
        let mut i: usize = 0;
        while carry != 0 {
            if i == self.limbs.len() {
                self.limbs.push(0);
            }
            let sum: u64 = u64::from(self.limbs[i]) + carry;
            self.limbs[i] = Self::low_u32(sum);
            carry = sum >> 32;
            i += 1;
        }
    }

    fn mul_small(&mut self, value: u32) {
        if self.is_zero() || value == 1 {
            return;
        }
        if value == 0 {
            self.limbs.clear();
            return;
        }

        let mut carry: u64 = 0;
        for limb in &mut self.limbs {
            let product: u64 = u64::from(*limb) * u64::from(value) + carry;
            *limb = Self::low_u32(product);
            carry = product >> 32;
        }
        if carry != 0 {
            self.limbs.push(Self::low_u32(carry));
        }
    }

    fn mul_pow5(&mut self, count: usize) {
        for _ in 0..count {
            self.mul_small(5);
        }
    }

    fn shl_assign(&mut self, bits: usize) {
        if self.is_zero() || bits == 0 {
            return;
        }

        let word_shift: usize = bits / 32;
        let bit_shift: usize = bits % 32;
        if word_shift != 0 {
            let old_len: usize = self.limbs.len();
            self.limbs.resize(old_len + word_shift, 0);
            self.limbs.copy_within(0..old_len, word_shift);
            self.limbs[..word_shift].fill(0);
        }

        if bit_shift != 0 {
            let mut carry: u64 = 0;
            for limb in &mut self.limbs {
                let shifted: u64 = (u64::from(*limb) << bit_shift) | carry;
                *limb = Self::low_u32(shifted);
                carry = shifted >> 32;
            }
            if carry != 0 {
                self.limbs.push(Self::low_u32(carry));
            }
        }
    }

    fn shr_one_assign(&mut self) {
        let mut carry: u32 = 0;
        for limb in self.limbs.iter_mut().rev() {
            let next_carry: u32 = *limb & 1;
            *limb = (*limb >> 1) | (carry << 31);
            carry = next_carry;
        }
        self.normalize();
    }

    fn bit_len(&self) -> usize {
        match self.limbs.last() {
            Some(last) => {
                let leading: u8 = u8::try_from(last.leading_zeros()).unwrap_or(0);
                (self.limbs.len() - 1) * 32 + (32 - usize::from(leading))
            },
            None => 0,
        }
    }

    fn bit_at(&self, bit: usize) -> bool {
        let word: usize = bit / 32;
        let offset: usize = bit % 32;
        self.limbs
            .get(word)
            .is_some_and(|limb| ((limb >> offset) & 1) != 0)
    }

    fn is_odd(&self) -> bool {
        self.limbs.first().is_some_and(|limb| (limb & 1) != 0)
    }

    fn sub_assign(&mut self, other: &Self) {
        let mut borrow: bool = false;
        for i in 0..self.limbs.len() {
            let rhs: u32 = other.limbs.get(i).copied().unwrap_or(0);
            let (difference, first_borrow): (u32, bool) = self.limbs[i].overflowing_sub(rhs);
            let borrow_value: u32 = if borrow { 1 } else { 0 };
            let (difference, second_borrow): (u32, bool) = difference.overflowing_sub(borrow_value);
            self.limbs[i] = difference;
            borrow = first_borrow || second_borrow;
        }
        self.normalize();
    }

    fn div_rem(numerator: &Self, denominator: &Self) -> (Self, Self) {
        let mut quotient: Self = Self::zero();
        let mut remainder: Self = Self::zero();
        for bit in (0..numerator.bit_len()).rev() {
            quotient.shl_assign(1);
            remainder.shl_assign(1);
            if numerator.bit_at(bit) {
                remainder.add_small(1);
            }
            if remainder.cmp(denominator) != Ordering::Less {
                remainder.sub_assign(denominator);
                quotient.add_small(1);
            }
        }
        (quotient, remainder)
    }

    fn as_u128(&self) -> u128 {
        let mut result: u128 = 0;
        for (i, limb) in self.limbs.iter().take(4).enumerate() {
            let shift: usize = i * 32;
            result |= u128::from(*limb) << shift;
        }
        result
    }
}

impl Ord for BigUint {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.limbs.len().cmp(&other.limbs.len()) {
            Ordering::Equal => {
                for i in (0..self.limbs.len()).rev() {
                    match self.limbs[i].cmp(&other.limbs[i]) {
                        Ordering::Equal => continue,
                        order => return order,
                    }
                }
                Ordering::Equal
            },
            order => order,
        }
    }
}

impl PartialOrd for BigUint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The nonzero digits of a parsed significand, with trailing zero digits represented in the radix
/// exponent rather than materialized in the integer.
struct Significand {
    value: BigUint,
    pending_zeros: i64,
    stored_digits: i64,
}

impl Significand {
    fn new() -> Self {
        Self {
            value: BigUint::zero(),
            pending_zeros: 0,
            stored_digits: 0,
        }
    }

    fn is_zero(&self) -> bool {
        self.value.is_zero()
    }

    fn push(&mut self, digit: u8, radix: u32) {
        if self.value.is_zero() {
            if digit == 0 {
                return;
            }
            self.value.add_small(u32::from(digit));
            self.stored_digits = 1;
            return;
        }

        if digit == 0 {
            self.pending_zeros = self.pending_zeros.saturating_add(1);
            return;
        }

        let zeros: usize = usize::try_from(self.pending_zeros).unwrap_or(usize::MAX);
        for _ in 0..zeros {
            self.value.mul_small(radix);
        }
        self.stored_digits = self.stored_digits.saturating_add(self.pending_zeros);
        self.pending_zeros = 0;
        self.value.mul_small(radix);
        self.value.add_small(u32::from(digit));
        self.stored_digits = self.stored_digits.saturating_add(1);
    }
}

/// The parsed magnitude expressed as `numerator / denominator * 2^binary_scale`.
struct Rational {
    numerator: BigUint,
    denominator: BigUint,
    binary_scale: i64,
}

//==================================================================================================
// Helpers
//==================================================================================================

fn as_i64(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn as_usize(value: i64) -> Option<usize> {
    usize::try_from(value).ok()
}

fn parse_exponent(p: *const c_char) -> Option<(*const c_char, i64)> {
    let mut cursor: *const c_char = p;
    let negative: bool = crate::c_char_to_u8(unsafe { *cursor }) == b'-';
    if matches!(crate::c_char_to_u8(unsafe { *cursor }), b'+' | b'-') {
        cursor = unsafe { cursor.add(1) };
    }
    if !is_digit(unsafe { *cursor }) {
        return None;
    }

    let mut exponent: i64 = 0;
    while is_digit(unsafe { *cursor }) {
        exponent = exponent
            .saturating_mul(10)
            .saturating_add(i64::from(digit_val(unsafe { *cursor })));
        cursor = unsafe { cursor.add(1) };
    }
    Some((
        cursor,
        if negative {
            exponent.saturating_neg()
        } else {
            exponent
        },
    ))
}

fn round_div(numerator: &BigUint, denominator: &BigUint) -> BigUint {
    let (mut quotient, mut remainder): (BigUint, BigUint) =
        BigUint::div_rem(numerator, denominator);
    remainder.shl_assign(1);
    let comparison: Ordering = remainder.cmp(denominator);
    if comparison == Ordering::Greater || (comparison == Ordering::Equal && quotient.is_odd()) {
        quotient.add_small(1);
    }
    quotient
}

fn floor_log2_ratio(numerator: &BigUint, denominator: &BigUint) -> i64 {
    let numerator_bits: i64 = as_i64(numerator.bit_len());
    let denominator_bits: i64 = as_i64(denominator.bit_len());
    let candidate: i64 = numerator_bits.saturating_sub(denominator_bits);

    let comparison: Ordering = if candidate >= 0 {
        let mut shifted: BigUint = denominator.clone();
        shifted.shl_assign(as_usize(candidate).unwrap_or(usize::MAX));
        numerator.cmp(&shifted)
    } else {
        let mut shifted: BigUint = numerator.clone();
        shifted.shl_assign(as_usize(candidate.saturating_neg()).unwrap_or(usize::MAX));
        shifted.cmp(denominator)
    };

    if comparison == Ordering::Less {
        candidate.saturating_sub(1)
    } else {
        candidate
    }
}

fn rounded_scaled(rational: &Rational, shift: i64) -> BigUint {
    if shift >= 0 {
        let mut numerator: BigUint = rational.numerator.clone();
        numerator.shl_assign(as_usize(shift).unwrap_or(usize::MAX));
        round_div(&numerator, &rational.denominator)
    } else {
        let mut denominator: BigUint = rational.denominator.clone();
        denominator.shl_assign(as_usize(shift.saturating_neg()).unwrap_or(usize::MAX));
        round_div(&rational.numerator, &denominator)
    }
}

fn binary128_from_rational(rational: Rational, negative: bool) -> (Binary128, bool) {
    let exponent: i64 = floor_log2_ratio(&rational.numerator, &rational.denominator)
        .saturating_add(rational.binary_scale);

    if exponent >= BINARY128_MAX_EXPONENT.saturating_add(1) {
        return (Binary128::infinity(negative), true);
    }
    if exponent < BINARY128_MIN_SUBNORMAL_EXPONENT.saturating_sub(1) {
        return (Binary128::zero(negative), true);
    }

    if exponent >= BINARY128_MIN_NORMAL_EXPONENT {
        let shift: i64 = rational
            .binary_scale
            .saturating_add(as_i64(BINARY128_FRACTION_BITS))
            .saturating_sub(exponent);
        let mut significand: BigUint = rounded_scaled(&rational, shift);
        if significand.bit_len() > BINARY128_PRECISION {
            significand.shr_one_assign();
            let exponent: i64 = exponent.saturating_add(1);
            if exponent >= BINARY128_MAX_EXPONENT.saturating_add(1) {
                return (Binary128::infinity(negative), true);
            }
            let implicit: u128 = 1_u128 << BINARY128_FRACTION_BITS;
            return (
                Binary128::from_parts(
                    negative,
                    u16::try_from(exponent.saturating_add(16_383)).unwrap_or(0),
                    significand.as_u128().saturating_sub(implicit),
                ),
                false,
            );
        }

        let implicit: u128 = 1_u128 << BINARY128_FRACTION_BITS;
        return (
            Binary128::from_parts(
                negative,
                u16::try_from(exponent.saturating_add(16_383)).unwrap_or(0),
                significand.as_u128().saturating_sub(implicit),
            ),
            false,
        );
    }

    let shift: i64 = rational
        .binary_scale
        .saturating_add(BINARY128_MIN_SUBNORMAL_EXPONENT.saturating_neg());
    let significand: BigUint = rounded_scaled(&rational, shift);
    let minimum_normal: u128 = 1_u128 << BINARY128_FRACTION_BITS;
    let fraction: u128 = significand.as_u128();
    if fraction >= minimum_normal {
        return (Binary128::from_parts(negative, 1, 0), false);
    }

    (Binary128::from_parts(negative, 0, fraction), true)
}

fn decimal_rational(significand: Significand, exponent: i64) -> Option<Rational> {
    let exponent: i64 = exponent.saturating_add(significand.pending_zeros);
    if exponent >= 0 {
        let mut numerator: BigUint = significand.value;
        numerator.mul_pow5(as_usize(exponent)?);
        Some(Rational {
            numerator,
            denominator: BigUint::one(),
            binary_scale: exponent,
        })
    } else {
        let mut denominator: BigUint = BigUint::one();
        denominator.mul_pow5(as_usize(exponent.saturating_neg())?);
        Some(Rational {
            numerator: significand.value,
            denominator,
            binary_scale: exponent,
        })
    }
}

fn hexadecimal_rational(significand: Significand, exponent: i64) -> Rational {
    Rational {
        numerator: significand.value,
        denominator: BigUint::one(),
        binary_scale: exponent.saturating_add(significand.pending_zeros.saturating_mul(4)),
    }
}

fn decimal_order(significand: &Significand, exponent: i64) -> i64 {
    significand
        .stored_digits
        .saturating_sub(1)
        .saturating_add(significand.pending_zeros)
        .saturating_add(exponent)
}

fn nan_payload(start: *const c_char, end: *const c_char) -> u128 {
    let mut cursor: *const c_char = start;
    let hexadecimal: bool = unsafe { cursor.add(1) } < end
        && crate::c_char_to_u8(unsafe { *cursor }) == b'0'
        && matches!(crate::c_char_to_u8(unsafe { *cursor.add(1) }), b'x' | b'X');
    if hexadecimal {
        cursor = unsafe { cursor.add(2) };
    }

    let radix: u128 = if hexadecimal { 16 } else { 10 };
    let mut payload: u128 = 0;
    while cursor < end {
        let character: c_char = unsafe { *cursor };
        let digit: Option<u8> = if hexadecimal {
            if is_hex_digit(character) {
                Some(hex_digit_val(character))
            } else {
                None
            }
        } else if is_digit(character) {
            Some(digit_val(character))
        } else {
            None
        };
        let Some(digit) = digit else {
            return 0;
        };
        payload = payload.wrapping_mul(radix).wrapping_add(u128::from(digit))
            & ((1_u128 << (BINARY128_FRACTION_BITS - 1)) - 1);
        cursor = unsafe { cursor.add(1) };
    }
    payload
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Converts the initial portion of a narrow string directly to an IEEE-754 binary128 encoding.
///
/// This is shared by the AArch64 `strtold` and `wcstold` ABI shims. It deliberately returns bits
/// rather than a Rust floating-point type because Rust has no stable binary128 type.
///
/// # Safety
///
/// `nptr` must point to a valid, NUL-terminated string. `endptr`, if non-null, must be writable.
pub unsafe fn strto_binary128(nptr: *const c_char, endptr: *mut *mut c_char) -> Binary128 {
    if nptr.is_null() {
        if !endptr.is_null() {
            unsafe { *endptr = core::ptr::null_mut() };
        }
        return Binary128::zero(false);
    }

    let mut cursor: *const c_char = nptr;
    while is_whitespace(unsafe { *cursor }) {
        cursor = unsafe { cursor.add(1) };
    }

    let negative: bool = crate::c_char_to_u8(unsafe { *cursor }) == b'-';
    if matches!(crate::c_char_to_u8(unsafe { *cursor }), b'+' | b'-') {
        cursor = unsafe { cursor.add(1) };
    }

    if unsafe { match_keyword(cursor, b"inf") } {
        let length: usize = if unsafe { match_keyword(cursor, b"infinity") } {
            8
        } else {
            3
        };
        cursor = unsafe { cursor.add(length) };
        if !endptr.is_null() {
            unsafe { *endptr = cursor.cast_mut() };
        }
        return Binary128::infinity(negative);
    }

    if unsafe { match_keyword(cursor, b"nan") } {
        cursor = unsafe { cursor.add(3) };
        let mut payload: u128 = 0;
        if crate::c_char_to_u8(unsafe { *cursor }) == b'(' {
            let payload_start: *const c_char = unsafe { cursor.add(1) };
            let mut payload_end: *const c_char = payload_start;
            while {
                let character: u8 = crate::c_char_to_u8(unsafe { *payload_end });
                character == b'_' || character.is_ascii_alphanumeric()
            } {
                payload_end = unsafe { payload_end.add(1) };
            }
            if crate::c_char_to_u8(unsafe { *payload_end }) == b')' {
                payload = nan_payload(payload_start, payload_end);
                cursor = unsafe { payload_end.add(1) };
            }
        }
        if !endptr.is_null() {
            unsafe { *endptr = cursor.cast_mut() };
        }
        return Binary128::nan(negative, payload);
    }

    if crate::c_char_to_u8(unsafe { *cursor }) == b'0'
        && matches!(crate::c_char_to_u8(unsafe { *cursor.add(1) }), b'x' | b'X')
    {
        let mut hexadecimal_cursor: *const c_char = unsafe { cursor.add(2) };
        let mut significand: Significand = Significand::new();
        let mut parsed_digit: bool = false;
        while is_hex_digit(unsafe { *hexadecimal_cursor }) {
            significand.push(hex_digit_val(unsafe { *hexadecimal_cursor }), 16);
            parsed_digit = true;
            hexadecimal_cursor = unsafe { hexadecimal_cursor.add(1) };
        }
        let mut fractional_digits: i64 = 0;
        if crate::c_char_to_u8(unsafe { *hexadecimal_cursor }) == b'.' {
            hexadecimal_cursor = unsafe { hexadecimal_cursor.add(1) };
            while is_hex_digit(unsafe { *hexadecimal_cursor }) {
                significand.push(hex_digit_val(unsafe { *hexadecimal_cursor }), 16);
                parsed_digit = true;
                fractional_digits = fractional_digits.saturating_add(1);
                hexadecimal_cursor = unsafe { hexadecimal_cursor.add(1) };
            }
        }

        if parsed_digit {
            let mut exponent: i64 = 0;
            if matches!(crate::c_char_to_u8(unsafe { *hexadecimal_cursor }), b'p' | b'P') {
                let marker: *const c_char = hexadecimal_cursor;
                hexadecimal_cursor = unsafe { hexadecimal_cursor.add(1) };
                if let Some((end, parsed_exponent)) = parse_exponent(hexadecimal_cursor) {
                    hexadecimal_cursor = end;
                    exponent = parsed_exponent;
                } else {
                    hexadecimal_cursor = marker;
                }
            }
            exponent = exponent.saturating_sub(fractional_digits.saturating_mul(4));
            if !endptr.is_null() {
                unsafe { *endptr = hexadecimal_cursor.cast_mut() };
            }
            if significand.is_zero() {
                return Binary128::zero(negative);
            }
            let (value, range_error): (Binary128, bool) =
                binary128_from_rational(hexadecimal_rational(significand, exponent), negative);
            if range_error {
                set_errno(ERANGE);
            }
            return value;
        }
    }

    let mut significand: Significand = Significand::new();
    let mut parsed_digit: bool = false;
    while is_digit(unsafe { *cursor }) {
        significand.push(digit_val(unsafe { *cursor }), 10);
        parsed_digit = true;
        cursor = unsafe { cursor.add(1) };
    }

    let mut fractional_digits: i64 = 0;
    if crate::c_char_to_u8(unsafe { *cursor }) == b'.' {
        cursor = unsafe { cursor.add(1) };
        while is_digit(unsafe { *cursor }) {
            significand.push(digit_val(unsafe { *cursor }), 10);
            parsed_digit = true;
            fractional_digits = fractional_digits.saturating_add(1);
            cursor = unsafe { cursor.add(1) };
        }
    }

    if !parsed_digit {
        if !endptr.is_null() {
            unsafe { *endptr = nptr.cast_mut() };
        }
        return Binary128::zero(false);
    }

    let mut exponent: i64 = 0;
    if matches!(crate::c_char_to_u8(unsafe { *cursor }), b'e' | b'E') {
        let marker: *const c_char = cursor;
        cursor = unsafe { cursor.add(1) };
        if let Some((end, parsed_exponent)) = parse_exponent(cursor) {
            cursor = end;
            exponent = parsed_exponent;
        } else {
            cursor = marker;
        }
    }

    if !endptr.is_null() {
        unsafe { *endptr = cursor.cast_mut() };
    }
    if significand.is_zero() {
        return Binary128::zero(negative);
    }

    exponent = exponent.saturating_sub(fractional_digits);
    let order: i64 = decimal_order(&significand, exponent);
    if order > MAX_DIRECT_DECIMAL_ORDER {
        set_errno(ERANGE);
        return Binary128::infinity(negative);
    }
    if order < -MAX_DIRECT_DECIMAL_ORDER {
        set_errno(ERANGE);
        return Binary128::zero(negative);
    }

    let Some(rational) = decimal_rational(significand, exponent) else {
        set_errno(ERANGE);
        return Binary128::zero(negative);
    };
    let (value, range_error): (Binary128, bool) = binary128_from_rational(rational, negative);
    if range_error {
        set_errno(ERANGE);
    }
    value
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use ::sysapi::{
        errno::ERANGE,
        ffi::c_int,
    };

    fn get_errno() -> c_int {
        unsafe { *::sysapi::errno::__errno_location() }
    }

    fn parse(input: &[u8]) -> (Binary128, *mut c_char) {
        let mut end: *mut c_char = core::ptr::null_mut();
        let value: Binary128 =
            unsafe { strto_binary128(input.as_ptr().cast::<c_char>(), &mut end) };
        (value, end)
    }

    #[test]
    fn parses_direct_decimal_precision_beyond_f64() {
        let (value, end): (Binary128, *mut c_char) =
            parse(b"1.0000000000000000000000000000000002tail\0");
        assert_eq!(
            value,
            Binary128 {
                low: 1,
                high: 0x3fff_0000_0000_0000,
            }
        );
        assert_eq!(unsafe { *end }, c_char::from_ne_bytes(*b"t"));
    }

    #[test]
    fn rounds_hexadecimal_ties_to_even() {
        let (tie, _): (Binary128, *mut c_char) = parse(b"0x1.00000000000000000000000000008p0\0");
        let (above_tie, _): (Binary128, *mut c_char) =
            parse(b"0x1.000000000000000000000000000081p0\0");
        assert_eq!(
            tie,
            Binary128 {
                low: 0,
                high: 0x3fff_0000_0000_0000,
            }
        );
        assert_eq!(
            above_tie,
            Binary128 {
                low: 1,
                high: 0x3fff_0000_0000_0000,
            }
        );
    }

    #[test]
    fn parses_hexadecimal_exponents_and_signs() {
        let (value, end): (Binary128, *mut c_char) = parse(b"-0x1.8p+1rest\0");
        assert_eq!(
            value,
            Binary128 {
                low: 0,
                high: 0xc000_8000_0000_0000,
            }
        );
        assert_eq!(unsafe { *end }, c_char::from_ne_bytes(*b"r"));
    }

    #[test]
    fn handles_binary128_range_without_f64_intermediates() {
        crate::set_errno(0);
        let (large, _): (Binary128, *mut c_char) = parse(b"1e4932\0");
        assert!(large.exponent() < 0x7fff);
        assert_eq!(get_errno(), 0);

        crate::set_errno(0);
        let (small, _): (Binary128, *mut c_char) = parse(b"1e-4932\0");
        assert_eq!(small.exponent(), 0);
        assert_ne!(small.low, 0);
        assert_eq!(get_errno(), ERANGE);

        crate::set_errno(0);
        let (overflow, _): (Binary128, *mut c_char) = parse(b"-1e4933\0");
        assert_eq!(overflow.exponent(), 0x7fff);
        assert_ne!(overflow.high & (1_u64 << 63), 0);
        assert_eq!(get_errno(), ERANGE);

        crate::set_errno(0);
        let (underflow, _): (Binary128, *mut c_char) = parse(b"-1e-5000\0");
        assert_eq!(underflow, Binary128::zero(true));
        assert_eq!(get_errno(), ERANGE);
    }

    #[test]
    fn rounds_subnormal_halfway_cases_to_even() {
        crate::set_errno(0);
        let (halfway, _): (Binary128, *mut c_char) = parse(b"0x1p-16495\0");
        assert_eq!(halfway, Binary128::zero(false));
        assert_eq!(get_errno(), ERANGE);

        crate::set_errno(0);
        let (above_halfway, _): (Binary128, *mut c_char) = parse(b"0x1.8p-16495\0");
        assert_eq!(above_halfway, Binary128 { low: 1, high: 0 });
        assert_eq!(get_errno(), ERANGE);
    }

    #[test]
    fn parses_special_values_and_end_pointer() {
        let (infinity, infinity_end): (Binary128, *mut c_char) = parse(b"-INFINITYx\0");
        assert_eq!(infinity, Binary128::infinity(true));
        assert_eq!(unsafe { *infinity_end }, c_char::from_ne_bytes(*b"x"));

        let (nan, nan_end): (Binary128, *mut c_char) = parse(b"nan(0x123)!\0");
        assert_eq!(nan, Binary128::nan(false, 0x123));
        assert_eq!(unsafe { *nan_end }, c_char::from_ne_bytes(*b"!"));

        let input: &[u8] = b"+.\0";
        let (invalid, invalid_end): (Binary128, *mut c_char) = parse(input);
        assert_eq!(invalid, Binary128::zero(false));
        assert_eq!(invalid_end, input.as_ptr().cast_mut().cast::<c_char>());
    }
}
