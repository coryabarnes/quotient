//! Logic, comparisons and bitwise operations.

use super::{R, arity, int};
use crate::eval::binary;
use crate::functions::stats::compare;
use crate::parser::BinOp;
use crate::value::Value;

/// Exact integers up to 2^53, the range where floating point is exact.
const MAX_SAFE: i128 = 9_007_199_254_740_992;

fn bits(name: &str, args: &[Value], f: fn(i64, i64) -> Option<i128>) -> R {
    arity(name, args, 2, 2)?;
    let (a, b) = (int(name, args, 0)?, int(name, args, 1)?);
    let r = f(a, b).ok_or_else(|| format!("{name}: shift must be between 0 and 63"))?;
    if r.abs() > MAX_SAFE {
        return Err(format!("{name}: result is larger than 2^53 and cannot be represented exactly"));
    }
    Ok(Value::Num(r as f64))
}

fn shift(n: i64) -> Option<u32> {
    u32::try_from(n).ok().filter(|s| *s < 64)
}

fn deep_equal(a: &Value, b: &Value) -> Result<bool, String> {
    match (a, b) {
        (Value::Matrix(x), Value::Matrix(y)) => {
            if x.len() != y.len() {
                return Ok(false);
            }
            for (p, q) in x.iter().zip(y) {
                if !deep_equal(p, q)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Matrix(_), _) | (_, Value::Matrix(_)) => Ok(false),
        (Value::Str(x), Value::Str(y)) => Ok(x == y),
        (Value::Str(_), _) | (_, Value::Str(_)) => Ok(false),
        _ => match binary(BinOp::Eq, a.clone(), b.clone()) {
            Ok(v) => v.truthy(),
            // Values that cannot be compared (e.g. different units) are simply not equal.
            Err(_) => Ok(false),
        },
    }
}

fn text(name: &str, args: &[Value]) -> Result<(String, String), String> {
    arity(name, args, 2, 2)?;
    let s = |v: &Value| match v {
        Value::Str(s) => s.clone(),
        other => other.to_string(),
    };
    Ok((s(&args[0]), s(&args[1])))
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    let op = |op: BinOp| {
        arity(name, args, 2, 2)?;
        binary(op, args[0].clone(), args[1].clone())
    };
    Some(match name {
        "and" => op(BinOp::And),
        "or" => op(BinOp::Or),
        "xor" => op(BinOp::Xor),
        "not" => super::each(name, args, &|v| Ok(Value::Bool(!v.truthy()?))),
        "equal" => op(BinOp::Eq),
        "unequal" => op(BinOp::Ne),
        "smaller" => op(BinOp::Lt),
        "smallerEq" => op(BinOp::Le),
        "larger" => op(BinOp::Gt),
        "largerEq" => op(BinOp::Ge),
        "compare" => arity(name, args, 2, 2).and_then(|_| {
            if let (Value::Str(a), Value::Str(b)) = (&args[0], &args[1]) {
                return Ok(Value::Num(a.cmp(b) as i8 as f64));
            }
            Ok(Value::Num(compare(&args[0], &args[1])? as i8 as f64))
        }),
        "deepEqual" => arity(name, args, 2, 2).and_then(|_| deep_equal(&args[0], &args[1]).map(Value::Bool)),
        "compareText" => text(name, args).map(|(a, b)| Value::Num(a.cmp(&b) as i8 as f64)),
        "equalText" => text(name, args).map(|(a, b)| Value::Bool(a == b)),
        "bitAnd" => bits(name, args, |a, b| Some((a & b) as i128)),
        "bitOr" => bits(name, args, |a, b| Some((a | b) as i128)),
        "bitXor" => bits(name, args, |a, b| Some((a ^ b) as i128)),
        "bitNot" => arity(name, args, 1, 1).and_then(|_| Ok(Value::Num(!int(name, args, 0)? as f64))),
        "leftShift" => bits(name, args, |a, n| shift(n).and_then(|s| (a as i128).checked_mul(1i128 << s))),
        "rightArithShift" => bits(name, args, |a, n| shift(n).map(|s| (a >> s) as i128)),
        "rightLogShift" => bits(name, args, |a, n| shift(n).map(|s| ((a as u64) >> s) as i128)),
        _ => return None,
    })
}
