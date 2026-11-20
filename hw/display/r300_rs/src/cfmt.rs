//! C `printf` conversions that Rust's formatter has no equivalent for, kept
//! byte-compatible so the register dump reads like the C one.

use core::fmt::{self, Write};

/// Writes `value` like C's `printf("%g", value)`: six significant digits, the
/// shorter of fixed and exponent notation, trailing zeros removed.
pub fn write_g(out: &mut impl Write, value: f32) -> fmt::Result {
    const PRECISION: i32 = 6;
    if value.is_nan() {
        return out.write_str(if value.is_sign_negative() { "-nan" } else { "nan" });
    }
    if value.is_infinite() {
        return out.write_str(if value < 0.0 { "-inf" } else { "inf" });
    }
    let value = f64::from(value);
    if value == 0.0 {
        return out.write_str(if value.is_sign_negative() { "-0" } else { "0" });
    }
    // Rust rounds the exact decimal expansion half to even, like the C library, so the
    // exponent here is the one after rounding to six digits.
    let sci = format!("{:.*e}", (PRECISION - 1) as usize, value);
    let (mantissa, exp) = sci.split_once('e').expect("{:e} always has an exponent");
    let exp: i32 = exp.parse().expect("{:e} exponent is an integer");
    if !(-4..PRECISION).contains(&exp) {
        let mantissa = strip_zeros(mantissa);
        let sign = if exp < 0 { '-' } else { '+' };
        write!(out, "{mantissa}e{sign}{:02}", exp.abs())
    } else {
        let decimals = (PRECISION - 1 - exp) as usize;
        let fixed = format!("{value:.decimals$}");
        out.write_str(strip_zeros(&fixed))
    }
}

/// Drops trailing zeros after a decimal point, and the point itself if
/// nothing follows it.
fn strip_zeros(s: &str) -> &str {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    }
}
