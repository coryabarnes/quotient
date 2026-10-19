//! High-precision arithmetic on top of `astro-float`.
//!
//! When high precision is on, number literals and constants become `BigFloat`s
//! carrying binary precision well beyond the displayed digits; results are
//! rounded to the requested number of decimal digits for display, which hides the
//! binary representation error (so `0.1 + 0.2` shows `0.3`).

use std::cell::{Cell, RefCell};

pub use astro_float::BigFloat;
use astro_float::{Consts, Radix, RoundingMode};

use crate::format::{layout, round_digits};

pub const RM: RoundingMode = RoundingMode::ToEven;

thread_local! {
    static CONSTS: RefCell<Consts> = RefCell::new(Consts::new().expect("astro-float constants cache"));
    /// Significant decimal digits while high precision is on; 0 means off.
    static DIGITS: Cell<usize> = const { Cell::new(0) };
}

/// Runs `f` with high precision set to `digits` (or off for `None`).
pub fn with_precision<R>(digits: Option<usize>, f: impl FnOnce() -> R) -> R {
    let previous = DIGITS.with(|d| d.replace(digits.unwrap_or(0)));
    let result = f();
    DIGITS.with(|d| d.set(previous));
    result
}

/// The current high-precision digit count, if enabled.
pub fn digits() -> Option<usize> {
    DIGITS.with(|d| match d.get() {
        0 => None,
        n => Some(n),
    })
}

/// Binary precision used for calculations: the decimal digits plus ~20 guard digits.
pub fn bits() -> usize {
    let digits = digits().unwrap_or(64);
    let bits = (digits as f64 * std::f64::consts::LOG2_10).ceil() as usize + 64;
    bits.div_ceil(64) * 64
}

pub fn with_consts<R>(f: impl FnOnce(&mut Consts) -> R) -> R {
    CONSTS.with(|c| f(&mut c.borrow_mut()))
}

/// Parses decimal text such as `"0.1"` or `"1.5e-3"` exactly (to the working precision).
pub fn parse(text: &str) -> BigFloat {
    let text: String = text.chars().filter(|c| *c != '_').collect();
    with_consts(|cc| BigFloat::parse(&text, Radix::Dec, bits(), RM, cc))
}

pub fn from_f64(x: f64) -> BigFloat {
    BigFloat::from_f64(x, bits())
}

pub fn from_i64(x: i64) -> BigFloat {
    BigFloat::from_i64(x, bits())
}

pub fn pi() -> BigFloat {
    with_consts(|cc| cc.pi(bits(), RM))
}

pub fn e() -> BigFloat {
    with_consts(|cc| cc.e(bits(), RM))
}

/// Splits astro-float's decimal output (`-1.2345e-3`, `0.0`, `Inf`, `NaN`) into
/// sign, significant digits and exponent.
fn decompose(x: &BigFloat) -> Result<(bool, String, i64), &'static str> {
    if x.is_nan() {
        return Err("NaN");
    }
    if x.is_inf_pos() {
        return Err("Infinity");
    }
    if x.is_inf_neg() {
        return Err("-Infinity");
    }
    if x.is_zero() {
        return Ok((false, String::new(), 0));
    }
    let s = with_consts(|cc| x.format(Radix::Dec, RM, cc)).map_err(|_| "NaN")?;
    let negative = s.starts_with('-');
    let s = s.trim_start_matches('-');
    let (mantissa, exp) = s.split_once('e').unwrap_or((s, "0"));
    let mut exp: i64 = exp.parse().map_err(|_| "NaN")?;
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits = format!("{int}{frac}");
    // Normalise so the first digit is non-zero.
    let leading = digits.len() - digits.trim_start_matches('0').len();
    digits.drain(..leading);
    exp += int.len() as i64 - 1 - leading as i64;
    Ok((negative, digits.trim_end_matches('0').to_string(), exp))
}

/// Formats to `digits` significant digits.
pub fn format(x: &BigFloat, digits: usize, thousands: bool) -> String {
    match decompose(x) {
        Err(special) => special.into(),
        Ok((_, d, _)) if d.is_empty() => "0".into(),
        Ok((negative, d, exp)) => {
            let (d, exp) = round_digits(&d, exp, digits);
            layout(negative, &d, exp, digits as i64, thousands)
        }
    }
}

pub fn to_f64(x: &BigFloat) -> f64 {
    match decompose(x) {
        Err("Infinity") => f64::INFINITY,
        Err("-Infinity") => f64::NEG_INFINITY,
        Err(_) => f64::NAN,
        Ok((_, d, _)) if d.is_empty() => 0.0,
        Ok((negative, d, exp)) => {
            let (d, exp) = round_digits(&d, exp, 20);
            let v: f64 = format!("{d}e{}", exp - d.len() as i64 + 1).parse().unwrap_or(f64::NAN);
            if negative { -v } else { v }
        }
    }
}

/// Relative tolerance comparison at the display precision.
pub fn nearly_equal(a: &BigFloat, b: &BigFloat) -> bool {
    let p = bits();
    let diff = a.sub(b, p, RM).abs();
    if diff.is_zero() {
        return true;
    }
    let scale = a.abs().max(&b.abs());
    let digits = digits().unwrap_or(64) as i64;
    let tolerance = with_consts(|cc| {
        BigFloat::from_i64(10, p).pow(&BigFloat::from_i64(-digits, p), p, RM, cc)
    });
    diff.cmp(&scale.mul(&tolerance, p, RM)).is_some_and(|c| c <= 0)
}

pub fn is_integer(x: &BigFloat) -> bool {
    x.is_int()
}

/// Exact conversion to i64 for integers that fit.
pub fn to_i64(x: &BigFloat) -> Option<i64> {
    if !x.is_int() {
        return None;
    }
    let f = to_f64(x);
    (f.abs() <= 9_007_199_254_740_992.0).then_some(f as i64)
}

/// Rounds to `decimals` places, half away from zero, on the decimal digits (after
/// dropping binary noise beyond the working precision), so 1.005 rounds to 1.01.
pub fn round_decimal(x: &BigFloat, decimals: i64) -> BigFloat {
    let Ok((negative, d, exp)) = decompose(x) else {
        return x.clone();
    };
    if d.is_empty() {
        return x.clone();
    }
    let (d, exp) = round_digits(&d, exp, digits().unwrap_or(64));
    let keep = exp + 1 + decimals;
    let text = if keep < 0 {
        "0".to_string()
    } else if keep == 0 {
        if d.as_bytes()[0] >= b'5' { format!("1e{}", -decimals) } else { "0".into() }
    } else {
        let (r, e) = round_digits(&d, exp, keep as usize);
        if r.is_empty() { "0".into() } else { format!("{r}e{}", e - r.len() as i64 + 1) }
    };
    let v = parse(&text);
    if negative { v.neg() } else { v }
}
