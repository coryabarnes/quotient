//! Number formatting shared by every numeric type.

/// How results are displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatOptions {
    /// Significant digits for ordinary numbers. `None` shows the shortest text that
    /// reads back as exactly the same number (like JavaScript), e.g. `0.30000000000000004`.
    pub digits: Option<usize>,
    /// Group the integer part in threes: `1,234,567.5`.
    pub thousands: bool,
}

impl FormatOptions {
    /// Exact round-trip formatting with no grouping.
    pub const FULL: Self = Self { digits: None, thousands: false };
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self { digits: Some(14), thousands: false }
    }
}

/// Lays out a number given its significant digits (no leading or trailing zeros;
/// empty means zero) and decimal exponent, so the value is `d.ddd × 10^exp`.
///
/// Uses plain notation for `1e-6 <= |x| < 10^max_exp` and `1.5e+21` otherwise.
/// Exact values use `max_exp` = 21 like JavaScript's `toString`; values rounded
/// to `n` digits use `n` like `toPrecision`, so rounding never shows as fake zeros
/// (`2.4329020081766e+18`, not `2432902008176600000`).
pub fn layout(negative: bool, digits: &str, exp: i64, max_exp: i64, thousands: bool) -> String {
    if digits.is_empty() {
        return "0".into();
    }
    let sign = if negative { "-" } else { "" };
    if !(-7 < exp && exp < max_exp) {
        let (first, rest) = digits.split_at(1);
        let frac = if rest.is_empty() { String::new() } else { format!(".{rest}") };
        let esign = if exp >= 0 { "+" } else { "-" };
        return format!("{sign}{first}{frac}e{esign}{}", exp.abs());
    }
    let (int, frac) = if exp >= 0 {
        let int_len = exp as usize + 1;
        if digits.len() <= int_len {
            (format!("{digits}{}", "0".repeat(int_len - digits.len())), String::new())
        } else {
            (digits[..int_len].to_string(), digits[int_len..].to_string())
        }
    } else {
        ("0".to_string(), format!("{}{digits}", "0".repeat((-exp - 1) as usize)))
    };
    let int = if thousands { group(&int) } else { int };
    if frac.is_empty() { format!("{sign}{int}") } else { format!("{sign}{int}.{frac}") }
}

fn group(int: &str) -> String {
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// Splits Rust's `{:e}` output (`"1.2345e-7"`) into significant digits and exponent.
fn split_scientific(s: &str) -> (String, i64) {
    let (mantissa, exp) = s.split_once('e').expect("scientific notation");
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0').to_string();
    (digits, exp.parse().expect("exponent"))
}

pub fn format_f64(x: f64, opts: FormatOptions) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if x == 0.0 {
        return "0".into();
    }
    // `{:e}` is the shortest decimal that reads back as the same number. Rounding
    // that (half away from zero) matches what was typed: 1.005 → 1.01.
    let (digits, exp) = split_scientific(&format!("{:e}", x.abs()));
    let (digits, exp) = match opts.digits {
        None => (digits, exp),
        Some(d) => round_digits(&digits, exp, d.max(1)),
    };
    let max_exp = opts.digits.map_or(21, |d| d as i64);
    layout(x < 0.0, &digits, exp, max_exp, opts.thousands)
}

/// JavaScript's `Number#toString`, which the original calctxt displayed.
pub fn js_number(x: f64) -> String {
    format_f64(x, FormatOptions::FULL)
}

/// Rounds a decimal digit string (as produced above) to `n` significant digits,
/// half away from zero. Returns the new digits and exponent.
pub fn round_digits(digits: &str, exp: i64, n: usize) -> (String, i64) {
    if digits.len() <= n {
        return (digits.trim_end_matches('0').to_string(), exp);
    }
    let mut kept: Vec<u8> = digits.as_bytes()[..n].iter().map(|d| d - b'0').collect();
    let mut exp = exp;
    if digits.as_bytes()[n] >= b'5' {
        let mut i = n;
        loop {
            if i == 0 {
                kept.insert(0, 1);
                kept.pop();
                exp += 1;
                break;
            }
            i -= 1;
            if kept[i] == 9 {
                kept[i] = 0;
            } else {
                kept[i] += 1;
                break;
            }
        }
    }
    let s: String = kept.iter().map(|d| (d + b'0') as char).collect();
    (s.trim_end_matches('0').to_string(), exp)
}

/// Rounds to `decimals` places after the point, half away from zero, using the
/// shortest decimal form of `x` so `round(1.005, 2)` is `1.01` as written.
pub fn round_decimal(x: f64, decimals: i64) -> f64 {
    if !x.is_finite() || x == 0.0 {
        return x;
    }
    let (digits, exp) = split_scientific(&format!("{:e}", x.abs()));
    let keep = exp + 1 + decimals;
    let rounded = if keep < 0 {
        0.0
    } else if keep == 0 {
        // Only the rounding digit remains: 0.5 → 1 at this position.
        if digits.as_bytes()[0] >= b'5' { 10f64.powi(-decimals as i32) } else { 0.0 }
    } else {
        let (d, e) = round_digits(&digits, exp, keep as usize);
        if d.is_empty() { 0.0 } else { format!("{d}e{}", e - d.len() as i64 + 1).parse().unwrap_or(f64::NAN) }
    };
    if x < 0.0 { -rounded } else { rounded }
}

/// Finds a fraction p/q with q ≤ 10⁶ equal to `x` up to floating-point rounding
/// (1e-14 relative), so `0.1 + 0.2` is 3/10 but pi has no fraction.
pub fn to_fraction(x: f64) -> Option<(f64, f64)> {
    if !x.is_finite() {
        return None;
    }
    let (mut h0, mut h1) = (0.0f64, 1.0f64);
    let (mut k0, mut k1) = (1.0f64, 0.0f64);
    let mut v = x.abs();
    for _ in 0..64 {
        let a = v.floor();
        let (h2, k2) = (a * h1 + h0, a * k1 + k0);
        if k2 > 1e6 {
            break;
        }
        (h0, h1, k0, k1) = (h1, h2, k1, k2);
        if (h1 / k1 - x.abs()).abs() <= 1e-14 * x.abs() {
            return Some((if x < 0.0 { -h1 } else { h1 }, k1));
        }
        let f = v - a;
        if f == 0.0 {
            break;
        }
        v = 1.0 / f;
    }
    None
}

/// `255` → `0xff`, with `prefix` `0x`, `0b` or `0o`.
pub fn to_radix(x: f64, radix: u32, prefix: &str) -> Result<String, String> {
    if x.fract() != 0.0 || !x.is_finite() || x.abs() > 9_007_199_254_740_992.0 {
        return Err(format!("Only whole numbers up to 2^53 can be shown in base {radix}"));
    }
    let mut n = x.abs() as u64;
    let mut digits = Vec::new();
    loop {
        digits.push(std::char::from_digit((n % radix as u64) as u32, radix).expect("digit"));
        n /= radix as u64;
        if n == 0 {
            break;
        }
    }
    let body: String = digits.iter().rev().collect();
    Ok(format!("{}{prefix}{body}", if x < 0.0 { "-" } else { "" }))
}
