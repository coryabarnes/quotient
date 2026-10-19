//! Arithmetic, powers, logarithms, rounding, trigonometry and complex numbers.

use std::f64::consts::PI;

use num_complex::Complex64 as C;

use super::{R, arity, each, flatten, num, real, reals};
use crate::big::{self, BigFloat, RM};
use crate::eval::{binary, nearly_equal, to_big};
use crate::parser::BinOp;
use crate::units::Quantity;
use crate::value::Value;

/// How a unary function treats quantities with units.
#[derive(Clone, Copy)]
enum Units {
    /// Not allowed.
    No,
    /// Applies to the value and keeps the unit (`abs`, `round`).
    Keep,
    /// Applies to the value and drops the unit (`sign`).
    Drop,
    /// The quantity must be an angle; it is converted to radians first.
    Angle,
    /// Raises the unit to this power (`sqrt(4 m^2)` = `2 m`).
    Power(f64),
}

struct Unary {
    real: fn(f64) -> f64,
    /// Inputs outside this domain are computed with `complex` instead.
    domain: fn(f64) -> bool,
    complex: Option<fn(C) -> C>,
    /// High-precision version; `None` from it means "use the ordinary path".
    big: Option<fn(&BigFloat) -> Option<BigFloat>>,
    units: Units,
}

fn all(_: f64) -> bool {
    true
}

fn p() -> usize {
    big::bits()
}

fn one() -> BigFloat {
    big::from_i64(1)
}

/// Compares a high-precision number with a small integer.
fn cmp_int(x: &BigFloat, n: i64) -> i64 {
    x.cmp(&big::from_i64(n)).unwrap_or(0).signum() as i64
}

fn recip(x: &BigFloat) -> BigFloat {
    one().div(x, p(), RM)
}

macro_rules! cc {
    ($x:ident, $f:ident) => {
        big::with_consts(|cc| $x.$f(p(), RM, cc))
    };
}

/// floor/ceil/fix snap to the nearest integer when within rounding error, as
/// mathjs does, so `floor((0.1 + 0.7) * 10)` is 8 rather than 7.
fn snap(x: f64, f: fn(f64) -> f64) -> f64 {
    if nearly_equal(x, x.round()) { x.round() } else { f(x) }
}

fn snap_big(x: &BigFloat, f: fn(&BigFloat) -> BigFloat) -> BigFloat {
    let rounded = big::round_decimal(x, 0);
    if big::nearly_equal(x, &rounded) { rounded } else { f(x) }
}

fn round_half_away(x: f64) -> f64 {
    crate::format::round_decimal(x, 0)
}

/// sin(pi) is 1.2e-16 in floating point only because pi is rounded; any result
/// within two ulps of the input's magnitude is indistinguishable from zero.
fn snap_zero(r: f64, x: f64) -> f64 {
    if r.abs() <= x.abs() * 4.5e-16 { 0.0 } else { r }
}

fn sin(x: f64) -> f64 {
    snap_zero(x.sin(), x)
}

fn cos(x: f64) -> f64 {
    snap_zero(x.cos(), x)
}

fn tan(x: f64) -> f64 {
    sin(x) / cos(x)
}

/// The high-precision version of `snap_zero`, at the working precision.
fn snap_zero_big(r: BigFloat, x: &BigFloat) -> BigFloat {
    let p = p();
    let tolerance = big::from_i64(2).powi(p - 8, p, RM);
    if r.abs().mul(&tolerance, p, RM).cmp(&x.abs()).is_some_and(|c| c <= 0) { big::from_i64(0) } else { r }
}

fn sin_big(x: &BigFloat) -> BigFloat {
    snap_zero_big(cc!(x, sin), x)
}

fn cos_big(x: &BigFloat) -> BigFloat {
    snap_zero_big(cc!(x, cos), x)
}

pub fn gamma(x: f64) -> f64 {
    if x.fract() == 0.0 {
        if x <= 0.0 {
            return f64::INFINITY;
        }
        if x > 171.0 {
            return f64::INFINITY;
        }
        return (1..x as u64).map(|i| i as f64).product();
    }
    if x < 0.5 {
        return PI / ((PI * x).sin() * gamma(1.0 - x));
    }
    // Lanczos approximation, g = 7, n = 9.
    const G: f64 = 7.0;
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    let x = x - 1.0;
    let t = x + G + 0.5;
    let sum = COEFFICIENTS[0] + (1..9).map(|i| COEFFICIENTS[i] / (x + i as f64)).sum::<f64>();
    (2.0 * PI).sqrt() * sum * ((x + 0.5) * t.ln() - t).exp()
}

pub fn erf(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    let a = x.abs();
    let result = if a < 2.5 {
        // Maclaurin series: 2/√π Σ (-1)^n x^(2n+1) / (n! (2n+1)).
        let mut term = a;
        let mut sum = a;
        let mut n = 0.0;
        while term.abs() > 1e-17 * sum.abs() {
            n += 1.0;
            term *= -a * a / n;
            sum += term / (2.0 * n + 1.0);
        }
        2.0 / PI.sqrt() * sum
    } else if a > 6.0 {
        1.0
    } else {
        // Continued fraction for erfc: e^(-x²)/√π / (x + ½/(x + 1/(x + 3/2/(x + ...)))).
        let mut f = a;
        for k in (1..=60).rev() {
            f = a + (k as f64 / 2.0) / f;
        }
        1.0 - (-a * a).exp() / PI.sqrt() / f
    };
    result.copysign(x)
}

fn unary(name: &str) -> Option<Unary> {
    macro_rules! u {
        ($real:expr, $domain:expr, $complex:expr, $big:expr, $units:expr) => {
            Unary { real: $real, domain: $domain, complex: $complex, big: $big, units: $units }
        };
    }
    Some(match name {
        "abs" => u!(f64::abs, all, Some(|z: C| C::new(z.norm(), 0.0)), Some(|x| Some(x.abs())), Units::Keep),
        "sign" => u!(
            |x| if x == 0.0 || x.is_nan() { x } else { x.signum() },
            all,
            Some(|z: C| if z.norm() == 0.0 { z } else { z / z.norm() }),
            Some(|x| Some(if x.is_zero() { x.clone() } else { x.signum() })),
            Units::Drop
        ),
        "unaryMinus" => u!(|x| -x, all, Some(|z: C| -z), Some(|x| Some(x.neg())), Units::Keep),
        "unaryPlus" => u!(|x| x, all, Some(|z: C| z), Some(|x| Some(x.clone())), Units::Keep),
        "sqrt" => u!(
            f64::sqrt,
            |x| x >= 0.0 || x.is_nan(),
            Some(|z: C| z.sqrt()),
            Some(|x| (cmp_int(x, 0) >= 0).then(|| x.sqrt(p(), RM))),
            Units::Power(0.5)
        ),
        "cbrt" => u!(f64::cbrt, all, Some(|z: C| z.cbrt()), Some(|x| Some(x.cbrt(p(), RM))), Units::Power(1.0 / 3.0)),
        "square" => u!(|x| x * x, all, Some(|z: C| z * z), Some(|x| Some(x.mul(x, p(), RM))), Units::Power(2.0)),
        "cube" => u!(
            |x| x * x * x,
            all,
            Some(|z: C| z * z * z),
            Some(|x| Some(x.mul(x, p(), RM).mul(x, p(), RM))),
            Units::Power(3.0)
        ),
        "exp" => u!(f64::exp, all, Some(|z: C| z.exp()), Some(|x| Some(cc!(x, exp))), Units::No),
        "expm1" => u!(
            f64::exp_m1,
            all,
            Some(|z: C| z.exp() - 1.0),
            Some(|x| Some(cc!(x, exp).sub(&one(), p(), RM))),
            Units::No
        ),
        "log" | "ln" => u!(
            f64::ln,
            |x| x >= 0.0 || x.is_nan(),
            Some(|z: C| z.ln()),
            Some(|x| (cmp_int(x, 0) > 0).then(|| cc!(x, ln))),
            Units::No
        ),
        "log10" => u!(
            f64::log10,
            |x| x >= 0.0 || x.is_nan(),
            Some(|z: C| z.ln() / std::f64::consts::LN_10),
            Some(|x| (cmp_int(x, 0) > 0).then(|| cc!(x, log10))),
            Units::No
        ),
        "log2" => u!(
            f64::log2,
            |x| x >= 0.0 || x.is_nan(),
            Some(|z: C| z.ln() / std::f64::consts::LN_2),
            Some(|x| (cmp_int(x, 0) > 0).then(|| cc!(x, log2))),
            Units::No
        ),
        "log1p" => u!(
            f64::ln_1p,
            |x| x >= -1.0 || x.is_nan(),
            Some(|z: C| (z + 1.0).ln()),
            Some(|x| {
                let y = x.add(&one(), p(), RM);
                (cmp_int(&y, 0) > 0).then(|| cc!(y, ln))
            }),
            Units::No
        ),
        "floor" => u!(
            |x| snap(x, f64::floor),
            all,
            Some(|z: C| C::new(snap(z.re, f64::floor), snap(z.im, f64::floor))),
            Some(|x| Some(snap_big(x, BigFloat::floor))),
            Units::Keep
        ),
        "ceil" => u!(
            |x| snap(x, f64::ceil),
            all,
            Some(|z: C| C::new(snap(z.re, f64::ceil), snap(z.im, f64::ceil))),
            Some(|x| Some(snap_big(x, BigFloat::ceil))),
            Units::Keep
        ),
        "fix" => u!(
            |x| snap(x, f64::trunc),
            all,
            Some(|z: C| C::new(snap(z.re, f64::trunc), snap(z.im, f64::trunc))),
            Some(|x| Some(snap_big(x, BigFloat::int))),
            Units::Keep
        ),
        "round" => u!(
            round_half_away,
            all,
            Some(|z: C| C::new(round_half_away(z.re), round_half_away(z.im))),
            Some(|x| Some(big::round_decimal(x, 0))),
            Units::Keep
        ),
        "sin" => u!(sin, all, Some(|z: C| z.sin()), Some(|x| Some(sin_big(x))), Units::Angle),
        "cos" => u!(cos, all, Some(|z: C| z.cos()), Some(|x| Some(cos_big(x))), Units::Angle),
        "tan" => u!(tan, all, Some(|z: C| z.tan()), Some(|x| Some(sin_big(x).div(&cos_big(x), p(), RM))), Units::Angle),
        "sec" => u!(|x| 1.0 / cos(x), all, Some(|z: C| z.cos().inv()), Some(|x| Some(recip(&cos_big(x)))), Units::Angle),
        "csc" => u!(|x| 1.0 / sin(x), all, Some(|z: C| z.sin().inv()), Some(|x| Some(recip(&sin_big(x)))), Units::Angle),
        "cot" => u!(|x| cos(x) / sin(x), all, Some(|z: C| z.tan().inv()), Some(|x| Some(cos_big(x).div(&sin_big(x), p(), RM))), Units::Angle),
        "asin" => u!(
            f64::asin,
            |x| (-1.0..=1.0).contains(&x) || x.is_nan(),
            Some(|z: C| z.asin()),
            Some(|x| (cmp_int(&x.abs(), 1) <= 0).then(|| cc!(x, asin))),
            Units::No
        ),
        "acos" => u!(
            f64::acos,
            |x| (-1.0..=1.0).contains(&x) || x.is_nan(),
            Some(|z: C| z.acos()),
            Some(|x| (cmp_int(&x.abs(), 1) <= 0).then(|| cc!(x, acos))),
            Units::No
        ),
        "atan" => u!(f64::atan, all, Some(|z: C| z.atan()), Some(|x| Some(cc!(x, atan))), Units::No),
        "asec" => u!(
            |x| (1.0 / x).acos(),
            |x| x.abs() >= 1.0 || x.is_nan(),
            Some(|z: C| z.inv().acos()),
            Some(|x| {
                let r = recip(x);
                (cmp_int(&x.abs(), 1) >= 0).then(|| cc!(r, acos))
            }),
            Units::No
        ),
        "acsc" => u!(
            |x| (1.0 / x).asin(),
            |x| x.abs() >= 1.0 || x.is_nan(),
            Some(|z: C| z.inv().asin()),
            Some(|x| {
                let r = recip(x);
                (cmp_int(&x.abs(), 1) >= 0).then(|| cc!(r, asin))
            }),
            Units::No
        ),
        "acot" => u!(
            |x| (1.0 / x).atan(),
            all,
            Some(|z: C| z.inv().atan()),
            Some(|x| {
                let r = recip(x);
                (!x.is_zero()).then(|| cc!(r, atan))
            }),
            Units::No
        ),
        "sinh" => u!(f64::sinh, all, Some(|z: C| z.sinh()), Some(|x| Some(cc!(x, sinh))), Units::Angle),
        "cosh" => u!(f64::cosh, all, Some(|z: C| z.cosh()), Some(|x| Some(cc!(x, cosh))), Units::Angle),
        "tanh" => u!(f64::tanh, all, Some(|z: C| z.tanh()), Some(|x| Some(cc!(x, tanh))), Units::Angle),
        "sech" => u!(|x| 1.0 / x.cosh(), all, Some(|z: C| z.cosh().inv()), Some(|x| Some(recip(&cc!(x, cosh)))), Units::Angle),
        "csch" => u!(|x| 1.0 / x.sinh(), all, Some(|z: C| z.sinh().inv()), Some(|x| Some(recip(&cc!(x, sinh)))), Units::Angle),
        "coth" => u!(|x| 1.0 / x.tanh(), all, Some(|z: C| z.tanh().inv()), Some(|x| Some(recip(&cc!(x, tanh)))), Units::Angle),
        "asinh" => u!(f64::asinh, all, Some(|z: C| z.asinh()), Some(|x| Some(cc!(x, asinh))), Units::No),
        "acosh" => u!(
            f64::acosh,
            |x| x >= 1.0 || x.is_nan(),
            Some(|z: C| z.acosh()),
            Some(|x| (cmp_int(x, 1) >= 0).then(|| cc!(x, acosh))),
            Units::No
        ),
        "atanh" => u!(
            f64::atanh,
            |x| (-1.0..=1.0).contains(&x) || x.is_nan(),
            Some(|z: C| z.atanh()),
            Some(|x| (cmp_int(&x.abs(), 1) < 0).then(|| cc!(x, atanh))),
            Units::No
        ),
        "asech" => u!(
            |x| (1.0 / x).acosh(),
            |x| (x > 0.0 && x <= 1.0) || x.is_nan(),
            Some(|z: C| z.inv().acosh()),
            None,
            Units::No
        ),
        "acsch" => u!(|x| (1.0 / x).asinh(), all, Some(|z: C| z.inv().asinh()), None, Units::No),
        "acoth" => u!(
            |x| (1.0 / x).atanh(),
            |x| x.abs() >= 1.0 || x.is_nan(),
            Some(|z: C| z.inv().atanh()),
            None,
            Units::No
        ),
        "gamma" => u!(gamma, all, None, None, Units::No),
        "erf" => u!(erf, all, None, None, Units::No),
        "toRadians" | "radians" | "rad" => u!(|d| d * (PI / 180.0), all, None, None, Units::No),
        "toDegrees" | "degrees" | "deg" => u!(|r| r * (180.0 / PI), all, None, None, Units::Angle),
        _ => return None,
    })
}

fn apply_unary(name: &str, spec: &Unary, v: Value) -> R {
    let real_path = |x: f64| -> R {
        if (spec.domain)(x) {
            Ok(Value::Num((spec.real)(x)))
        } else if let Some(f) = spec.complex {
            Ok(Value::from_complex(f(C::new(x, 0.0))))
        } else {
            Err(format!("{name} is undefined for {x}"))
        }
    };
    match v {
        Value::Quantity(q) => apply_to_quantity(name, spec, q, &real_path),
        Value::Complex(z) => match spec.complex {
            Some(f) => Ok(Value::from_complex(f(z))),
            None => Err(format!("{name} does not support complex numbers")),
        },
        Value::Big(b) => {
            if let Some(r) = spec.big.and_then(|f| f(&b)) {
                return Ok(Value::Big(r));
            }
            real_path(big::to_f64(&b))
        }
        other => match other.as_f64() {
            Some(x) => real_path(x),
            None => Err(format!("{name} expects a number, got a {}", other.type_name())),
        },
    }
}

fn apply_to_quantity(name: &str, spec: &Unary, q: Quantity, real_path: &dyn Fn(f64) -> R) -> R {
    match spec.units {
        Units::No => Err(format!("{name} does not accept units ({})", q.unit)),
        Units::Keep => Ok(Value::Quantity(Quantity { value: (spec.real)(q.value), ..q })),
        Units::Drop => Ok(Value::Num((spec.real)(q.value))),
        // degrees() and friends take angle quantities too: toDegrees(1 rad) = 57.29...
        Units::Angle => {
            if !q.unit.is_angle() {
                return Err(format!("{name} expects an angle, got {}", q.unit));
            }
            real_path(q.to_si())
        }
        Units::Power(power) => {
            let unit = q.unit.powf(power).ok_or_else(|| format!("{name} of {} does not have whole-number units", q.unit))?;
            if (spec.domain)(q.value) {
                Ok(Value::Quantity(Quantity { value: (spec.real)(q.value), unit }))
            } else {
                Err(format!("{name} of a negative quantity is not supported"))
            }
        }
    }
}

/// Folds `add(a, b, c)` into `(a + b) + c`.
fn fold(name: &str, op: BinOp, args: &[Value]) -> R {
    arity(name, args, 2, usize::MAX)?;
    let mut acc = args[0].clone();
    for a in &args[1..] {
        acc = binary(op, acc, a.clone())?;
    }
    Ok(acc)
}

fn two(name: &str, op: BinOp, args: &[Value]) -> R {
    arity(name, args, 2, 2)?;
    binary(op, args[0].clone(), args[1].clone())
}

/// Real n-th root: `nthRoot(-27, 3)` = -3.
fn nth_root(x: f64, n: f64) -> Result<f64, String> {
    if n == 0.0 {
        return Err("nthRoot: the root cannot be zero".into());
    }
    if x < 0.0 {
        if n.fract() == 0.0 && (n as i64) % 2 != 0 {
            return Ok(-(-x).powf(1.0 / n));
        }
        return Err("nthRoot of a negative number needs an odd root; use nthRoots for complex roots".into());
    }
    // Correct the last bit so exact roots come out exact: nthRoot(125, 3) = 5.
    let r = x.powf(1.0 / n);
    let rounded = r.round();
    Ok(if n.fract() == 0.0 && rounded.powf(n) == x { rounded } else { r })
}

fn round_to(name: &str, args: &[Value]) -> R {
    let digits = super::int(name, args, 1)?;
    if !(-15..=100).contains(&digits) {
        return Err("round: the number of decimals must be between -15 and 100".into());
    }
    crate::eval::map_values(args[0].clone(), &|v| match v {
        Value::Quantity(q) => Ok(Value::Quantity(Quantity { value: crate::format::round_decimal(q.value, digits), ..q })),
        Value::Complex(z) => Ok(Value::from_complex(C::new(
            crate::format::round_decimal(z.re, digits),
            crate::format::round_decimal(z.im, digits),
        ))),
        Value::Big(b) => Ok(Value::Big(big::round_decimal(&b, digits))),
        other => match other.as_f64() {
            Some(x) => num(crate::format::round_decimal(x, digits)),
            None => Err(format!("round expects a number, got a {}", other.type_name())),
        },
    })
}

fn atan2(args: &[Value]) -> R {
    arity("atan2", args, 2, 2)?;
    if let (Some(y), Some(x)) = (to_big(&args[0]), to_big(&args[1]))
        && (matches!(args[0], Value::Big(_)) || matches!(args[1], Value::Big(_)))
    {
        let p = p();
        let pi = big::pi();
        let half_pi = pi.div(&big::from_i64(2), p, RM);
        let r = if x.is_zero() {
            match cmp_int(&y, 0) {
                1.. => half_pi,
                0 => big::from_i64(0),
                _ => half_pi.neg(),
            }
        } else {
            let q = y.div(&x, p, RM);
            let a = cc!(q, atan);
            match (x.is_negative(), y.is_negative()) {
                (false, _) => a,
                (true, false) => a.add(&pi, p, RM),
                (true, true) => a.sub(&pi, p, RM),
            }
        };
        return Ok(Value::Big(r));
    }
    let (y, x) = match (&args[0], &args[1]) {
        (Value::Quantity(a), Value::Quantity(b)) => (a.value, a.value_of(b)?),
        _ => (real("atan2", args, 0)?, real("atan2", args, 1)?),
    };
    num(y.atan2(x))
}

fn log(args: &[Value]) -> R {
    arity("log", args, 1, 2)?;
    if args.len() == 1 {
        return each("log", args, &|v| apply_unary("log", &unary("log").expect("log"), v));
    }
    let ln = |v: &Value| apply_unary("log", &unary("log").expect("log"), v.clone());
    binary(BinOp::Div, ln(&args[0])?, ln(&args[1])?)
}

fn hypot(args: &[Value]) -> R {
    let values = flatten(args);
    if let Some(Value::Quantity(first)) = values.first() {
        let mut sum = 0.0;
        for v in &values {
            let Value::Quantity(q) = v else {
                return Err("hypot: either all or none of the values must have units".into());
            };
            sum += first.value_of(q)?.powi(2);
        }
        return Ok(super::quantity(sum.sqrt(), first));
    }
    let xs = super::flat_reals("hypot", args)?;
    num(xs.iter().map(|x| x * x).sum::<f64>().sqrt())
}

fn complex_fn(name: &str, args: &[Value]) -> Option<R> {
    let part = |f: fn(C) -> Value| each(name, args, &move |v| match v.as_complex() {
        Some(z) => Ok(f(z)),
        None => Err(format!("{name} expects a number, got a {}", v.type_name())),
    });
    Some(match name {
        "complex" => reals(name, args, 1, 2).map(|a| Value::from_complex(C::new(a[0], *a.get(1).unwrap_or(&0.0)))),
        "re" => part(|z| Value::Num(z.re)),
        "im" => part(|z| Value::Num(z.im)),
        "arg" => part(|z| Value::Num(z.arg())),
        "conj" => part(|z| Value::from_complex(z.conj())),
        "nthRoots" => (|| {
            arity(name, args, 1, 2)?;
            let z = args[0].as_complex().ok_or("nthRoots expects a number")?;
            let n = if args.len() == 2 { super::count_arg(name, args, 1)? } else { 2 };
            if n == 0 || n > 10_000 {
                return Err("nthRoots: the root must be between 1 and 10000".into());
            }
            let (r, theta) = z.to_polar();
            let r = r.powf(1.0 / n as f64);
            Ok(Value::Matrix(
                (0..n)
                    .map(|k| {
                        let root = C::from_polar(r, (theta + 2.0 * PI * k as f64) / n as f64);
                        // Clean up components that are zero up to rounding error.
                        let clean = |x: f64| if x.abs() < 1e-14 * r.max(1.0) { 0.0 } else { x };
                        Value::from_complex(C::new(clean(root.re), clean(root.im)))
                    })
                    .collect(),
            ))
        })(),
        _ => return None,
    })
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    if let Some(spec) = unary(name) {
        // log(x, base) and round(x, digits) take a second argument.
        if args.len() == 2 && (name == "log" || name == "ln") {
            return Some(log(args));
        }
        if args.len() == 2 && name == "round" {
            return Some(round_to(name, args));
        }
        return Some(each(name, args, &|v| apply_unary(name, &spec, v)));
    }
    if let Some(r) = complex_fn(name, args) {
        return Some(r);
    }
    Some(match name {
        "add" => fold(name, BinOp::Add, args),
        "multiply" => fold(name, BinOp::Mul, args),
        "subtract" => two(name, BinOp::Sub, args),
        "divide" => two(name, BinOp::Div, args),
        "mod" => two(name, BinOp::Mod, args),
        "pow" => two(name, BinOp::Pow, args),
        "atan2" => atan2(args),
        "hypot" => hypot(args),
        "nthRoot" => (|| {
            arity(name, args, 1, 2)?;
            let n = if args.len() == 2 { real(name, args, 1)? } else { 2.0 };
            match &args[0] {
                Value::Quantity(q) => {
                    let unit = q.unit.powf(1.0 / n).ok_or("nthRoot: units would not be whole")?;
                    Ok(Value::Quantity(Quantity { value: nth_root(q.value, n)?, unit }))
                }
                _ => crate::eval::map_values(args[0].clone(), &|v| {
                    let x = v.as_f64().ok_or("nthRoot expects a number")?;
                    num(nth_root(x, n)?)
                }),
            }
        })(),
        "dist2d" => reals(name, args, 4, 4).and_then(|a| num(f64::hypot(a[2] - a[0], a[3] - a[1]))),
        "distance" => (|| {
            arity(name, args, 2, 2)?;
            let a = super::flat_reals(name, &args[..1])?;
            let b = super::flat_reals(name, &args[1..])?;
            if a.len() != b.len() || !(2..=3).contains(&a.len()) {
                return Err("distance expects two points with 2 or 3 coordinates each".into());
            }
            num(a.iter().zip(&b).map(|(p, q)| (p - q).powi(2)).sum::<f64>().sqrt())
        })(),
        _ => return None,
    })
}
