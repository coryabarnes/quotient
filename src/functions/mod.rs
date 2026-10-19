//! Built-in functions. Each submodule handles one area and returns `None` for
//! names it does not know, so `call` can try them in turn.

mod comb;
mod finance;
mod logic;
mod math;
mod matrix;
mod sets;
mod stats;
mod types;

use crate::eval::{CONSTANTS, map_values};
use crate::units::Quantity;
use crate::value::Value;

pub type R = Result<Value, String>;
/// Calls a function value; used by `map` and `filter`.
pub type Apply<'a> = &'a mut dyn FnMut(&Value, Vec<Value>) -> R;

pub fn call(name: &str, args: Vec<Value>, apply: Apply) -> Option<R> {
    if let Some(r) = matrix::call_with(name, &args, apply) {
        return Some(r);
    }
    math::call(name, &args)
        .or_else(|| stats::call(name, &args))
        .or_else(|| comb::call(name, &args))
        .or_else(|| matrix::call(name, &args))
        .or_else(|| sets::call(name, &args))
        .or_else(|| logic::call(name, &args))
        .or_else(|| types::call(name, &args))
        .or_else(|| finance::call(name, &args))
}

/// Names that cannot be assigned to: built-in functions, constants and keywords.
pub fn is_protected(name: &str) -> bool {
    crate::catalog::get(name).is_some() || CONSTANTS.contains(&name) || crate::parser::is_keyword(name)
}

pub(crate) fn arity(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), String> {
    if args.len() < min {
        return Err(format!("Too few arguments in function {name} (expected at least {min}, got {})", args.len()));
    }
    if args.len() > max {
        return Err(format!("Too many arguments in function {name} (expected at most {max}, got {})", args.len()));
    }
    Ok(())
}

pub(crate) fn real(name: &str, args: &[Value], i: usize) -> Result<f64, String> {
    let v = args.get(i).ok_or_else(|| format!("Too few arguments in function {name}"))?;
    v.as_f64().ok_or_else(|| {
        format!("Unexpected type of argument {} in function {name} (expected a number, got a {})", i + 1, v.type_name())
    })
}

pub(crate) fn reals(name: &str, args: &[Value], min: usize, max: usize) -> Result<Vec<f64>, String> {
    arity(name, args, min, max)?;
    (0..args.len()).map(|i| real(name, args, i)).collect()
}

/// A whole number argument.
pub(crate) fn int(name: &str, args: &[Value], i: usize) -> Result<i64, String> {
    let x = real(name, args, i)?;
    if x.fract() != 0.0 || x.abs() > 9_007_199_254_740_992.0 {
        return Err(format!("Argument {} of function {name} must be a whole number", i + 1));
    }
    Ok(x as i64)
}

/// A non-negative whole number argument.
pub(crate) fn count_arg(name: &str, args: &[Value], i: usize) -> Result<u64, String> {
    let n = int(name, args, i)?;
    u64::try_from(n).map_err(|_| format!("Argument {} of function {name} must not be negative", i + 1))
}

/// All values with matrices flattened, the way the original's `normalizeArgs`
/// accepts both `sum(1, 2, 3)` and `sum([1, 2, 3])`.
pub(crate) fn flatten(args: &[Value]) -> Vec<Value> {
    fn go(v: &Value, out: &mut Vec<Value>) {
        match v {
            Value::Matrix(items) => items.iter().for_each(|i| go(i, out)),
            other => out.push(other.clone()),
        }
    }
    let mut out = Vec::new();
    args.iter().for_each(|a| go(a, &mut out));
    out
}

/// Flattened real numbers.
pub(crate) fn flat_reals(name: &str, args: &[Value]) -> Result<Vec<f64>, String> {
    flatten(args)
        .iter()
        .map(|v| {
            v.as_f64().ok_or_else(|| format!("Function {name} expects numbers, got a {}", v.type_name()))
        })
        .collect()
}

/// Applies a function to a single argument or element-wise to a matrix.
pub(crate) fn each(name: &str, args: &[Value], f: &dyn Fn(Value) -> R) -> R {
    arity(name, args, 1, 1)?;
    map_values(args[0].clone(), f)
}

pub(crate) fn num(x: f64) -> R {
    Ok(Value::Num(x))
}

pub(crate) fn nums(v: impl IntoIterator<Item = f64>) -> Value {
    Value::Matrix(v.into_iter().map(Value::Num).collect())
}

pub(crate) fn quantity(value: f64, like: &Quantity) -> Value {
    Value::Quantity(Quantity { value, unit: like.unit.clone() })
}
