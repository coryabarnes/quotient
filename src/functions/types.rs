//! Type checks, conversions, text and unit helpers.

use super::{R, arity, each, int, real};
use crate::big;
use crate::eval::{Scope, evaluate, unit_expr};
use crate::format::{FormatOptions, to_radix};
use crate::parser::{self, Stmt};
use crate::units::{Quantity, Unit};
use crate::value::Value;

/// A unit given as text (`"km/h"`) or as a unit value (`km`, which evaluates to 1 km).
fn unit_arg(name: &str, v: &Value) -> Result<Unit, String> {
    match v {
        Value::Str(s) => match parser::parse(s)? {
            Stmt::Expr(e) => unit_expr(&e),
            _ => Err(format!("{name}: \"{s}\" is not a unit")),
        },
        Value::Quantity(q) if q.value == 1.0 => Ok(q.unit.clone()),
        other => Err(format!("{name}: expected a unit, got {}", other)),
    }
}

fn number(v: &Value) -> R {
    Ok(Value::Num(match v {
        Value::Str(s) => s.trim().parse::<f64>().map_err(|_| format!("Cannot convert \"{s}\" to a number"))?,
        Value::Quantity(q) => q.value,
        Value::Complex(z) if z.im == 0.0 => z.re,
        other => other.as_f64().ok_or_else(|| format!("Cannot convert a {} to a number", other.type_name()))?,
    }))
}

fn bignumber(v: &Value) -> R {
    match v {
        Value::Big(b) => Ok(Value::Big(b.clone())),
        Value::Str(s) => {
            let t = s.trim().replace('_', "");
            t.parse::<f64>().map_err(|_| format!("Cannot convert \"{s}\" to a number"))?;
            Ok(Value::Big(big::parse(&t)))
        }
        // Use the shortest decimal form, so bignumber(0.1) is exactly 0.1.
        other => match other.as_f64() {
            Some(x) if x.is_finite() => Ok(Value::Big(big::parse(&format!("{x:e}")))),
            Some(x) => Ok(Value::Num(x)),
            None => Err(format!("Cannot convert a {} to a bignumber", other.type_name())),
        },
    }
}

fn split_unit(args: &[Value]) -> R {
    arity("splitUnit", args, 2, 2)?;
    let Value::Quantity(q) = &args[0] else {
        return Err("splitUnit expects a quantity with units first".into());
    };
    let units: Vec<Unit> = match &args[1] {
        Value::Matrix(items) => items.iter().map(|u| unit_arg("splitUnit", u)).collect::<Result<_, _>>()?,
        other => vec![unit_arg("splitUnit", other)?],
    };
    if units.is_empty() {
        return Err("splitUnit needs at least one unit".into());
    }
    let mut remaining = q.clone();
    let mut parts = Vec::new();
    for (i, unit) in units.iter().enumerate() {
        let converted = remaining.convert(unit)?;
        if i + 1 == units.len() {
            parts.push(Value::Quantity(converted));
            break;
        }
        // Treat values within rounding error of the next whole number as whole.
        let rounded = converted.value.round();
        let whole = if (converted.value - rounded).abs() < 1e-9 * converted.value.abs().max(1.0) { rounded } else { converted.value.trunc() };
        parts.push(Value::Quantity(Quantity { value: whole, unit: unit.clone() }));
        let rest = Quantity { value: converted.value - whole, unit: unit.clone() };
        remaining = rest;
    }
    Ok(Value::Matrix(parts))
}

fn check(name: &str, args: &[Value], f: fn(f64) -> bool) -> R {
    each(name, args, &|v| {
        let x = match &v {
            Value::Quantity(q) => q.value,
            other => other.as_f64().ok_or_else(|| format!("{name} expects a number, got a {}", other.type_name()))?,
        };
        Ok(Value::Bool(f(x)))
    })
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    Some(match name {
        "number" => (|| {
            arity(name, args, 1, 2)?;
            if args.len() == 2 {
                let Value::Quantity(q) = &args[0] else {
                    return Err("number(x, unit) needs a quantity with units".into());
                };
                return Ok(Value::Num(q.convert(&unit_arg(name, &args[1])?)?.value));
            }
            crate::eval::map_values(args[0].clone(), &|v| number(&v))
        })(),
        "string" => arity(name, args, 1, 1).map(|_| Value::Str(args[0].to_string())),
        "format" => (|| {
            arity(name, args, 1, 2)?;
            let digits = if args.len() == 2 { int(name, args, 1)? } else { 14 };
            if !(1..=100).contains(&digits) {
                return Err("format: digits must be between 1 and 100".into());
            }
            let opts = FormatOptions { digits: Some(digits as usize), thousands: false };
            Ok(Value::Str(match &args[0] {
                Value::Big(b) => big::format(b, digits as usize, false),
                other => other.format(opts),
            }))
        })(),
        "boolean" => each(name, args, &|v| match v {
            Value::Str(s) if s == "true" => Ok(Value::Bool(true)),
            Value::Str(s) if s == "false" => Ok(Value::Bool(false)),
            Value::Str(s) => s.trim().parse::<f64>().map(|x| Value::Bool(x != 0.0)).map_err(|_| format!("Cannot convert \"{s}\" to boolean")),
            other => other.truthy().map(Value::Bool),
        }),
        "bignumber" => each(name, args, &|v| bignumber(&v)),
        "unit" => (|| {
            arity(name, args, 1, 2)?;
            if args.len() == 2 {
                let value = real(name, args, 0)?;
                return Ok(Value::Quantity(Quantity { value, unit: unit_arg(name, &args[1])? }));
            }
            match &args[0] {
                q @ Value::Quantity(_) => Ok(q.clone()),
                Value::Str(s) => match evaluate(s, &mut Scope::new())?.value {
                    q @ Value::Quantity(_) => Ok(q),
                    _ => Err(format!("unit: \"{s}\" is not a quantity with units")),
                },
                other => Err(format!("unit expects text like \"5 cm\", got a {}", other.type_name())),
            }
        })(),
        "splitUnit" => split_unit(args),
        "hex" | "bin" | "oct" => {
            let (radix, prefix) = match name {
                "hex" => (16, "0x"),
                "bin" => (2, "0b"),
                _ => (8, "0o"),
            };
            each(name, args, &|v| {
                let x = v.as_f64().ok_or_else(|| format!("{name} expects a whole number"))?;
                Ok(Value::Str(to_radix(x, radix, prefix)?))
            })
        }
        "typeOf" => arity(name, args, 1, 1).map(|_| {
            Value::Str(
                match &args[0] {
                    Value::Num(_) => "number",
                    Value::Big(_) => "BigNumber",
                    Value::Complex(_) => "Complex",
                    Value::Quantity(_) => "Unit",
                    Value::Bool(_) => "boolean",
                    Value::Str(_) => "string",
                    Value::Matrix(_) => "Matrix",
                    Value::Func(_) | Value::Builtin(_) => "function",
                }
                .into(),
            )
        }),
        "isInteger" => check(name, args, |x| x.fract() == 0.0 && x.is_finite()),
        "isNaN" => check(name, args, f64::is_nan),
        "isNegative" => check(name, args, |x| x < 0.0),
        "isPositive" => check(name, args, |x| x > 0.0),
        "isZero" => check(name, args, |x| x == 0.0),
        "isNumeric" => arity(name, args, 1, 1).map(|_| Value::Bool(matches!(args[0], Value::Num(_) | Value::Big(_) | Value::Bool(_)))),
        "hasNumericValue" => arity(name, args, 1, 1).map(|_| {
            Value::Bool(match &args[0] {
                Value::Str(s) => s.trim().parse::<f64>().is_ok(),
                v => matches!(v, Value::Num(_) | Value::Big(_) | Value::Bool(_)),
            })
        }),
        _ => return None,
    })
}
