//! Evaluates parsed expressions.

use std::collections::HashMap;
use std::rc::Rc;

use num_complex::Complex64;

use crate::big::{self, BigFloat, RM};
use crate::format::{FormatOptions, format_f64, to_fraction, to_radix};
use crate::functions;
use crate::parser::{self, BinOp, Expr, PercentOp, Stmt};
use crate::units::{self, Combined, Quantity, Unit};
pub use crate::value::{UserFn, Value};

pub type Scope = HashMap<String, Value>;
type R = Result<Value, String>;

/// User-defined functions calling themselves deeper than this produce an error.
const MAX_CALL_DEPTH: usize = 100;
/// Largest matrix a range or function may create.
pub const MAX_ELEMENTS: usize = 1_000_000;

/// The result of evaluating one statement.
#[derive(Debug)]
pub struct Evaluated {
    pub value: Value,
    /// For assignments, what to show left of `=`: `x` or `f(x)`.
    pub assigned: Option<String>,
}

/// Parses and evaluates one expression, updating `scope` on assignment.
pub fn evaluate(src: &str, scope: &mut Scope) -> Result<Evaluated, String> {
    match parser::parse(src)? {
        Stmt::Expr(e) => Ok(Evaluated { value: eval(&e, scope, 0)?, assigned: None }),
        Stmt::Assign(name, e) => {
            if functions::is_protected(&name) {
                return Err(format!("Cannot assign to '{name}': it is a protected function or constant name"));
            }
            let value = eval(&e, scope, 0)?;
            scope.insert(name.clone(), value.clone());
            Ok(Evaluated { value, assigned: Some(name) })
        }
        Stmt::FuncDef { name, params, body, body_src } => {
            if functions::is_protected(&name) {
                return Err(format!("Cannot assign to '{name}': it is a protected function or constant name"));
            }
            let label = format!("{name}({})", params.join(", "));
            let value = Value::Func(Rc::new(UserFn { name: name.clone(), params, body, body_src }));
            scope.insert(name, value.clone());
            Ok(Evaluated { value, assigned: Some(label) })
        }
    }
}

fn eval(e: &Expr, scope: &Scope, depth: usize) -> R {
    let ev = |e: &Expr| eval(e, scope, depth);
    match e {
        Expr::Num { value, text } => Ok(match big::digits() {
            Some(_) => Value::Big(big::parse(text)),
            None => Value::Num(*value),
        }),
        Expr::Str(s) => Ok(Value::Str(s.clone())),
        Expr::Var(name) => lookup(name, scope),
        Expr::Matrix(items) => items.iter().map(ev).collect::<Result<_, _>>().map(Value::Matrix),
        Expr::Range(start, step, end) => {
            let step = match step {
                Some(s) => Some(ev(s)?),
                None => None,
            };
            range(ev(start)?, step, ev(end)?, true)
        }
        Expr::Neg(inner) => negate(ev(inner)?),
        Expr::Not(inner) => map_values(ev(inner)?, &|v| Ok(Value::Bool(!v.truthy()?))),
        Expr::Binary(op @ (BinOp::Add | BinOp::Sub), a, b) if matches!(**b, Expr::Percent(_)) => {
            // `100 + 10%` is 110, not 100.1.
            let Expr::Percent(pct) = &**b else { unreachable!() };
            let base = ev(a)?;
            let delta = binary(BinOp::Mul, base.clone(), percent(ev(pct)?)?)?;
            binary(*op, base, delta)
        }
        Expr::Binary(BinOp::And, a, b) => {
            if !ev(a)?.truthy()? {
                return Ok(Value::Bool(false));
            }
            Ok(Value::Bool(ev(b)?.truthy()?))
        }
        Expr::Binary(BinOp::Or, a, b) => {
            if ev(a)?.truthy()? {
                return Ok(Value::Bool(true));
            }
            Ok(Value::Bool(ev(b)?.truthy()?))
        }
        Expr::Binary(op, a, b) => binary(*op, ev(a)?, ev(b)?),
        Expr::Factorial(inner) => call("factorial", vec![ev(inner)?], scope, depth),
        Expr::Percent(inner) => percent(ev(inner)?),
        Expr::PercentOf(op, pct, base) => {
            let Expr::Percent(inner) = &**pct else {
                let word = match op {
                    PercentOp::Of => "of",
                    PercentOp::Off => "off",
                    PercentOp::On => "on",
                };
                return Err(format!("A percentage is expected before \"{word}\", like 20% {word} 150"));
            };
            let fraction = percent(ev(inner)?)?;
            let base = ev(base)?;
            let part = binary(BinOp::Mul, base.clone(), fraction)?;
            match op {
                PercentOp::Of => Ok(part),
                PercentOp::Off => binary(BinOp::Sub, base, part),
                PercentOp::On => binary(BinOp::Add, base, part),
            }
        }
        Expr::Index(base, idx) => {
            let mut v = ev(base)?;
            for i in idx {
                v = index(v, ev(i)?)?;
            }
            Ok(v)
        }
        Expr::Call(name, args) => {
            let args = args.iter().map(ev).collect::<Result<_, _>>()?;
            call(name, args, scope, depth)
        }
        Expr::Convert(value, target) => convert(ev(value)?, target),
        Expr::Ternary(cond, yes, no) => {
            if ev(cond)?.truthy()? {
                ev(yes)
            } else {
                ev(no)
            }
        }
    }
}

fn percent(v: Value) -> R {
    binary(BinOp::Div, v, Value::Num(100.0))
}

pub fn constant(name: &str) -> Option<Value> {
    let high = big::digits().is_some();
    let big_or = |b: fn() -> BigFloat, x: f64| if high { Value::Big(b()) } else { Value::Num(x) };
    Some(match name {
        "pi" | "PI" => big_or(big::pi, std::f64::consts::PI),
        "e" | "E" => big_or(big::e, std::f64::consts::E),
        "tau" => big_or(|| big::pi().mul(&big::from_i64(2), big::bits(), RM), std::f64::consts::TAU),
        "phi" => big_or(
            || {
                let p = big::bits();
                let five = big::from_i64(5).sqrt(p, RM);
                five.add(&big::from_i64(1), p, RM).div(&big::from_i64(2), p, RM)
            },
            1.618_033_988_749_895,
        ),
        "i" => Value::Complex(Complex64::new(0.0, 1.0)),
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        "Infinity" => Value::Num(f64::INFINITY),
        "NaN" => Value::Num(f64::NAN),
        "LN2" => Value::Num(std::f64::consts::LN_2),
        "LN10" => Value::Num(std::f64::consts::LN_10),
        "LOG2E" => Value::Num(std::f64::consts::LOG2_E),
        "LOG10E" => Value::Num(std::f64::consts::LOG10_E),
        "SQRT2" => Value::Num(std::f64::consts::SQRT_2),
        "SQRT1_2" => Value::Num(std::f64::consts::FRAC_1_SQRT_2),
        _ => return None,
    })
}

pub const CONSTANTS: &[&str] = &[
    "pi", "PI", "e", "E", "tau", "phi", "i", "true", "false", "Infinity", "NaN", "LN2", "LN10", "LOG2E",
    "LOG10E", "SQRT2", "SQRT1_2",
];

/// Variables first, then constants, units and finally built-in functions.
fn lookup(name: &str, scope: &Scope) -> R {
    if let Some(v) = scope.get(name) {
        return Ok(v.clone());
    }
    if let Some(v) = constant(name) {
        return Ok(v);
    }
    if let Some((prefix, def)) = units::lookup(name) {
        return Ok(Value::Quantity(Quantity { value: 1.0, unit: Unit::single(prefix, def) }));
    }
    if let Some(entry) = crate::catalog::get(name) {
        return Ok(Value::Builtin(entry.name));
    }
    Err(format!("Undefined symbol {name}"))
}

fn call(name: &str, args: Vec<Value>, scope: &Scope, depth: usize) -> R {
    match scope.get(name) {
        Some(f @ (Value::Func(_) | Value::Builtin(_))) => apply(f, args, scope, depth),
        Some(_) => Err(format!("{name} is a variable, not a function")),
        None => {
            let mut apply_fn = |f: &Value, args: Vec<Value>| apply(f, args, scope, depth + 1);
            functions::call(name, args, &mut apply_fn).unwrap_or_else(|| Err(format!("Undefined function {name}")))
        }
    }
}

/// Calls a function value (user-defined or built-in) with arguments.
fn apply(f: &Value, args: Vec<Value>, scope: &Scope, depth: usize) -> R {
    if depth >= MAX_CALL_DEPTH {
        return Err("Maximum call depth exceeded (is a function calling itself forever?)".into());
    }
    match f {
        Value::Func(f) => {
            if args.len() != f.params.len() {
                return Err(format!(
                    "Wrong number of arguments in function {} ({} provided, {} expected)",
                    f.name,
                    args.len(),
                    f.params.len()
                ));
            }
            let mut local = scope.clone();
            for (p, v) in f.params.iter().zip(args) {
                local.insert(p.clone(), v);
            }
            eval(&f.body, &local, depth + 1)
        }
        Value::Builtin(name) => {
            let mut apply_fn = |f: &Value, args: Vec<Value>| apply(f, args, scope, depth + 1);
            functions::call(name, args, &mut apply_fn).unwrap_or_else(|| Err(format!("Undefined function {name}")))
        }
        other => Err(format!("A {} is not a function", other.type_name())),
    }
}

/// Applies `f` to a value, or to every element of a matrix.
pub fn map_values(v: Value, f: &dyn Fn(Value) -> R) -> R {
    match v {
        Value::Matrix(items) => items.into_iter().map(|i| map_values(i, f)).collect::<Result<_, _>>().map(Value::Matrix),
        other => f(other),
    }
}

pub fn negate(v: Value) -> R {
    map_values(v, &|v| match v {
        Value::Num(x) => Ok(Value::Num(-x)),
        Value::Big(b) => Ok(Value::Big(b.neg())),
        Value::Complex(c) => Ok(Value::Complex(-c)),
        Value::Quantity(q) => Ok(Value::Quantity(Quantity { value: -q.value, ..q })),
        Value::Bool(b) => Ok(Value::Num(if b { -1.0 } else { 0.0 })),
        other => Err(format!("Cannot negate a {}", other.type_name())),
    })
}

/// Inclusive (`1:5`) or exclusive (`range(1, 5)`) sequences.
pub fn range(start: Value, step: Option<Value>, end: Value, inclusive: bool) -> R {
    let num = |v: &Value, what: &str| v.as_f64().ok_or_else(|| format!("Range {what} must be a number"));
    let start = num(&start, "start")?;
    let end = num(&end, "end")?;
    let step = match &step {
        Some(s) => num(s, "step")?,
        None => 1.0,
    };
    if step == 0.0 || !step.is_finite() || !start.is_finite() || !end.is_finite() {
        return Err("Range needs finite numbers and a non-zero step".into());
    }
    let count = ((end - start) / step + 1e-9).floor();
    if count < 0.0 {
        return Ok(Value::Matrix(vec![]));
    }
    if count >= MAX_ELEMENTS as f64 {
        return Err(format!("Range would have more than {MAX_ELEMENTS} elements"));
    }
    let mut items = Vec::new();
    for k in 0..=count as usize {
        let x = start + k as f64 * step;
        if !inclusive && (if step > 0.0 { x >= end } else { x <= end }) {
            break;
        }
        items.push(Value::Num(x));
    }
    Ok(Value::Matrix(items))
}

/// mathjs compares numbers with a relative tolerance, so `0.1 + 0.2 == 0.3` is true.
pub fn nearly_equal(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    if !a.is_finite() || !b.is_finite() {
        return false;
    }
    let diff = (a - b).abs();
    diff <= 1e-12 * a.abs().max(b.abs()) || diff < f64::MIN_POSITIVE
}

fn compare_f64(op: BinOp, a: f64, b: f64) -> bool {
    let eq = nearly_equal(a, b);
    match op {
        BinOp::Eq => eq,
        BinOp::Ne => !eq,
        BinOp::Lt => a < b && !eq,
        BinOp::Le => a < b || eq,
        BinOp::Gt => a > b && !eq,
        BinOp::Ge => a > b || eq,
        _ => unreachable!("not a comparison"),
    }
}

/// mathjs `mod`: the result takes the sign of the divisor; `x mod 0` is `x`.
pub fn modulo(x: f64, y: f64) -> f64 {
    if y == 0.0 { x } else { x - y * (x / y).floor() }
}

fn real_op(op: BinOp, a: f64, b: f64) -> Value {
    match op {
        BinOp::Add => Value::Num(a + b),
        BinOp::Sub => Value::Num(a - b),
        BinOp::Mul => Value::Num(a * b),
        BinOp::Div => Value::Num(a / b),
        BinOp::Mod => Value::Num(modulo(a, b)),
        // A negative base with a fractional exponent has a complex result, as in mathjs.
        BinOp::Pow if a < 0.0 && b.fract() != 0.0 && b.is_finite() => {
            complex_pow(Complex64::new(a, 0.0), Complex64::new(b, 0.0))
        }
        BinOp::Pow => Value::Num(a.powf(b)),
        BinOp::And => Value::Bool(a != 0.0 && b != 0.0),
        BinOp::Or => Value::Bool(a != 0.0 || b != 0.0),
        BinOp::Xor => Value::Bool((a != 0.0) != (b != 0.0)),
        cmp => Value::Bool(compare_f64(cmp, a, b)),
    }
}

fn complex_pow(x: Complex64, y: Complex64) -> Value {
    if y.im == 0.0 && y.re.fract() == 0.0 && y.re.abs() <= 1024.0 {
        return Value::from_complex(x.powi(y.re as i32));
    }
    if x == Complex64::new(0.0, 0.0) {
        return Value::Num(if y.re > 0.0 { 0.0 } else { f64::INFINITY });
    }
    Value::from_complex(x.powc(y))
}

fn complex_op(op: BinOp, x: Complex64, y: Complex64) -> R {
    Ok(match op {
        BinOp::Add => Value::from_complex(x + y),
        BinOp::Sub => Value::from_complex(x - y),
        BinOp::Mul => Value::from_complex(x * y),
        BinOp::Div => Value::from_complex(x / y),
        BinOp::Pow => complex_pow(x, y),
        BinOp::Eq => Value::Bool(nearly_equal(x.re, y.re) && nearly_equal(x.im, y.im)),
        BinOp::Ne => Value::Bool(!(nearly_equal(x.re, y.re) && nearly_equal(x.im, y.im))),
        BinOp::Mod => return Err("mod is not defined for complex numbers".into()),
        _ => return Err("Complex numbers cannot be compared with < or >".into()),
    })
}

/// A value as a high-precision number, if that loses nothing.
pub fn to_big(v: &Value) -> Option<BigFloat> {
    match v {
        Value::Big(b) => Some(b.clone()),
        // Whole numbers convert exactly; anything else already lost precision as f64.
        Value::Num(x) if x.fract() == 0.0 && x.abs() <= 9_007_199_254_740_992.0 => Some(big::from_f64(*x)),
        Value::Bool(b) => Some(big::from_i64(*b as i64)),
        _ => None,
    }
}

fn big_op(op: BinOp, x: &BigFloat, y: &BigFloat) -> Value {
    let p = big::bits();
    match op {
        BinOp::Add => Value::Big(x.add(y, p, RM)),
        BinOp::Sub => Value::Big(x.sub(y, p, RM)),
        BinOp::Mul => Value::Big(x.mul(y, p, RM)),
        BinOp::Div => Value::Big(x.div(y, p, RM)),
        BinOp::Mod if y.is_zero() => Value::Big(x.clone()),
        BinOp::Mod => Value::Big(x.sub(&y.mul(&x.div(y, p, RM).floor(), p, RM), p, RM)),
        BinOp::Pow => big_pow(x, y),
        BinOp::And | BinOp::Or | BinOp::Xor => real_op(op, big::to_f64(x), big::to_f64(y)),
        cmp => {
            let eq = big::nearly_equal(x, y);
            let ord = x.cmp(y).unwrap_or(0);
            Value::Bool(match cmp {
                BinOp::Eq => eq,
                BinOp::Ne => !eq,
                BinOp::Lt => ord < 0 && !eq,
                BinOp::Le => ord < 0 || eq,
                BinOp::Gt => ord > 0 && !eq,
                _ => ord > 0 || eq,
            })
        }
    }
}

pub fn big_pow(x: &BigFloat, y: &BigFloat) -> Value {
    let p = big::bits();
    if let Some(n) = big::to_i64(y).filter(|n| n.unsigned_abs() <= 1_000_000) {
        let r = x.powi(n.unsigned_abs() as usize, p, RM);
        return Value::Big(if n < 0 { big::from_i64(1).div(&r, p, RM) } else { r });
    }
    if x.is_negative() {
        return complex_pow(Complex64::new(big::to_f64(x), 0.0), Complex64::new(big::to_f64(y), 0.0));
    }
    Value::Big(big::with_consts(|cc| x.pow(y, p, RM, cc)))
}

fn quantity_op(op: BinOp, a: Value, b: Value) -> R {
    match (a, b) {
        (Value::Quantity(x), Value::Quantity(y)) => match op {
            BinOp::Add | BinOp::Sub => {
                let v = x.value_of(&y)?;
                let value = if op == BinOp::Add { x.value + v } else { x.value - v };
                Ok(Value::Quantity(Quantity { value, unit: x.unit }))
            }
            BinOp::Mul | BinOp::Div => Ok(match x.combine(&y, if op == BinOp::Mul { 1 } else { -1 }) {
                Combined::Quantity(q) => Value::Quantity(q),
                Combined::Number(n) => Value::Num(n),
            }),
            BinOp::Mod => {
                let v = x.value_of(&y)?;
                Ok(Value::Quantity(Quantity { value: modulo(x.value, v), unit: x.unit }))
            }
            BinOp::Pow => Err("An exponent cannot have units".into()),
            cmp => Ok(Value::Bool(compare_f64(cmp, x.value, x.value_of(&y)?))),
        },
        (Value::Quantity(q), n) => {
            let Some(n) = n.as_f64() else {
                return Err(format!("Cannot combine {} with a {}", q.unit, n.type_name()));
            };
            match op {
                BinOp::Mul => Ok(Value::Quantity(Quantity { value: q.value * n, ..q })),
                BinOp::Div => Ok(Value::Quantity(Quantity { value: q.value / n, ..q })),
                BinOp::Pow => {
                    let unit = q.unit.powf(n).ok_or_else(|| {
                        format!("Cannot raise {} to the power {}", q.unit, format_f64(n, FormatOptions::FULL))
                    })?;
                    if unit.terms.is_empty() {
                        return Ok(Value::Num(1.0));
                    }
                    Ok(Value::Quantity(Quantity { value: q.value.powf(n), unit }))
                }
                _ => Err(format!("Cannot {} {} and a number without units", verb(op), q.unit)),
            }
        }
        (n, Value::Quantity(q)) => {
            let Some(n) = n.as_f64() else {
                return Err(format!("Cannot combine a {} with {}", n.type_name(), q.unit));
            };
            match op {
                BinOp::Mul => Ok(Value::Quantity(Quantity { value: n * q.value, ..q })),
                BinOp::Div => Ok(Value::Quantity(Quantity { value: n / q.value, unit: q.unit.powi(-1) })),
                _ => Err(format!("Cannot {} a number without units and {}", verb(op), q.unit)),
            }
        }
        _ => unreachable!("quantity_op needs a quantity"),
    }
}

fn verb(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "add",
        BinOp::Sub => "subtract",
        BinOp::Mod => "take the remainder of",
        BinOp::Pow => "raise",
        _ => "compare",
    }
}

/// Applies a binary operator to any two values.
pub fn binary(op: BinOp, a: Value, b: Value) -> R {
    match (a, b) {
        (Value::Matrix(x), Value::Matrix(y)) => {
            if op == BinOp::Mul {
                return matmul(&x, &y);
            }
            if x.len() != y.len() {
                return Err(format!("Dimension mismatch ({} != {})", x.len(), y.len()));
            }
            x.into_iter().zip(y).map(|(p, q)| binary(op, p, q)).collect::<Result<_, _>>().map(Value::Matrix)
        }
        (Value::Matrix(x), s) => x.into_iter().map(|p| binary(op, p, s.clone())).collect::<Result<_, _>>().map(Value::Matrix),
        (s, Value::Matrix(y)) => y.into_iter().map(|q| binary(op, s.clone(), q)).collect::<Result<_, _>>().map(Value::Matrix),
        (a, b) if matches!(op, BinOp::And | BinOp::Or | BinOp::Xor) => {
            let (x, y) = (a.truthy()?, b.truthy()?);
            Ok(Value::Bool(match op {
                BinOp::And => x && y,
                BinOp::Or => x || y,
                _ => x != y,
            }))
        }
        (Value::Str(x), Value::Str(y)) if op == BinOp::Eq => Ok(Value::Bool(x == y)),
        (Value::Str(x), Value::Str(y)) if op == BinOp::Ne => Ok(Value::Bool(x != y)),
        (Value::Str(x), Value::Str(y)) if op == BinOp::Add => Ok(Value::Str(x + &y)),
        (a, b) if matches!(a, Value::Quantity(_)) || matches!(b, Value::Quantity(_)) => {
            if matches!(a, Value::Complex(_)) || matches!(b, Value::Complex(_)) {
                return Err("Complex numbers with units are not supported".into());
            }
            quantity_op(op, a, b)
        }
        (a, b) if matches!(a, Value::Complex(_)) || matches!(b, Value::Complex(_)) => {
            match (a.as_complex(), b.as_complex()) {
                (Some(x), Some(y)) => complex_op(op, x, y),
                _ => Err(type_error(op, &a, &b)),
            }
        }
        (a, b) if matches!(a, Value::Big(_)) || matches!(b, Value::Big(_)) => match (to_big(&a), to_big(&b)) {
            (Some(x), Some(y)) => Ok(big_op(op, &x, &y)),
            _ => match (a.as_f64(), b.as_f64()) {
                (Some(x), Some(y)) => Ok(real_op(op, x, y)),
                _ => Err(type_error(op, &a, &b)),
            },
        },
        (a, b) => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => Ok(real_op(op, x, y)),
            _ => Err(type_error(op, &a, &b)),
        },
    }
}

fn type_error(op: BinOp, a: &Value, b: &Value) -> String {
    format!("Cannot apply {} to {} and {}", op.symbol(), a.type_name(), b.type_name())
}

fn as_vector(items: &[Value]) -> Option<Vec<f64>> {
    items.iter().map(Value::as_f64).collect()
}

fn as_rows(items: &[Value]) -> Option<Vec<Vec<f64>>> {
    items
        .iter()
        .map(|r| match r {
            Value::Matrix(row) => as_vector(row),
            _ => None,
        })
        .collect()
}

fn nums(v: Vec<f64>) -> Value {
    Value::Matrix(v.into_iter().map(Value::Num).collect())
}

/// mathjs `multiply` for matrices: dot product of vectors, otherwise matrix product.
fn matmul(x: &[Value], y: &[Value]) -> R {
    let mismatch = || Err("Dimension mismatch in matrix multiplication".to_string());
    let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(p, q)| p * q).sum::<f64>();
    let column = |m: &[Vec<f64>], j: usize| m.iter().map(|r| r[j]).collect::<Vec<f64>>();
    match (as_vector(x), as_vector(y), as_rows(x), as_rows(y)) {
        (Some(a), Some(b), _, _) => {
            if a.len() != b.len() {
                return mismatch();
            }
            Ok(Value::Num(dot(&a, &b)))
        }
        (_, _, Some(a), Some(b)) => {
            let cols = b.first().map_or(0, Vec::len);
            if a.iter().any(|r| r.len() != b.len()) || b.iter().any(|r| r.len() != cols) {
                return mismatch();
            }
            Ok(Value::Matrix(a.iter().map(|r| nums((0..cols).map(|j| dot(r, &column(&b, j))).collect())).collect()))
        }
        (_, Some(b), Some(a), _) => {
            if a.iter().any(|r| r.len() != b.len()) {
                return mismatch();
            }
            Ok(nums(a.iter().map(|r| dot(r, &b)).collect()))
        }
        (Some(a), _, _, Some(b)) => {
            let cols = b.first().map_or(0, Vec::len);
            if a.len() != b.len() || b.iter().any(|r| r.len() != cols) {
                return mismatch();
            }
            Ok(nums((0..cols).map(|j| dot(&a, &column(&b, j))).collect()))
        }
        _ => Err("Only matrices of real numbers can be multiplied".into()),
    }
}

/// 1-based indexing, as in mathjs: `data[1]` is the first element.
fn index(base: Value, idx: Value) -> R {
    let i = idx.as_f64().ok_or("Index must be a number")?;
    if i.fract() != 0.0 || i < 1.0 {
        return Err(format!("Index must be a whole number of 1 or more, got {}", format_f64(i, FormatOptions::FULL)));
    }
    let i = i as usize;
    match base {
        Value::Matrix(items) => {
            let len = items.len();
            items.into_iter().nth(i - 1).ok_or_else(|| format!("Index out of range ({i} > {len})"))
        }
        Value::Str(s) => {
            let len = s.chars().count();
            s.chars().nth(i - 1).map(|c| Value::Str(c.to_string())).ok_or_else(|| format!("Index out of range ({i} > {len})"))
        }
        other => Err(format!("Cannot index a {}", other.type_name())),
    }
}

/// Display formats usable after `to`.
pub const FORMATS: &[&str] = &["hex", "bin", "oct", "fraction", "sci", "percent"];

fn convert(v: Value, target: &Expr) -> R {
    if let Expr::Var(name) = target
        && FORMATS.contains(&name.as_str())
    {
        return map_values(v, &|v| to_format(v, name));
    }
    let unit = unit_expr(target)?;
    map_values(v, &|v| match v {
        Value::Quantity(q) => Ok(Value::Quantity(q.convert(&unit)?)),
        other => match other.as_f64() {
            Some(_) if unit.is_dimensionless() => Ok(other),
            Some(_) => Err(format!("Cannot convert a number without units to {unit}")),
            None => Err(format!("Cannot convert a {} to {unit}", other.type_name())),
        },
    })
}

fn to_format(v: Value, format: &str) -> R {
    let (x, suffix) = match &v {
        Value::Quantity(q) => (q.value, format!(" {}", q.unit)),
        other => (other.as_f64().ok_or_else(|| format!("Cannot show a {} as {format}", other.type_name()))?, String::new()),
    };
    let text = match format {
        "hex" => to_radix(x, 16, "0x")?,
        "bin" => to_radix(x, 2, "0b")?,
        "oct" => to_radix(x, 8, "0o")?,
        "sci" => {
            if x == 0.0 || !x.is_finite() {
                format_f64(x, FormatOptions::default())
            } else {
                let exp = x.abs().log10().floor() as i32;
                let mantissa = crate::format::round_decimal(x / 10f64.powi(exp), 13);
                // Rounding can carry the mantissa to 10 (9.99999… → 10).
                let (mantissa, exp) = if mantissa.abs() >= 10.0 { (mantissa / 10.0, exp + 1) } else { (mantissa, exp) };
                format!("{} × 10^{exp}", format_f64(mantissa, FormatOptions::default()))
            }
        }
        "percent" => format!("{}%", format_f64(x * 100.0, FormatOptions::default())),
        _ => match to_fraction(x) {
            Some((p, q)) if q == 1.0 => format_f64(p, FormatOptions::FULL),
            Some((p, q)) => format!("{}/{}", format_f64(p, FormatOptions::FULL), format_f64(q, FormatOptions::FULL)),
            None => return Err(format!("{} has no simple fraction", format_f64(x, FormatOptions::default()))),
        },
    };
    Ok(Value::Str(text + &suffix))
}

/// Evaluates the target of `to`: unit names combined with `*`, `/`, `^` and spaces.
pub fn unit_expr(e: &Expr) -> Result<Unit, String> {
    match e {
        Expr::Var(name) => match units::lookup(name) {
            Some((prefix, def)) => Ok(Unit::single(prefix, def)),
            None => Err(format!("Unknown unit \"{name}\"")),
        },
        Expr::Num { value, .. } if *value == 1.0 => Ok(Unit::default()),
        Expr::Binary(op @ (BinOp::Mul | BinOp::Div), a, b) => {
            let a = unit_expr(a)?;
            let b = unit_expr(b)?;
            let b = if *op == BinOp::Div { b.powi(-1) } else { b };
            let mut terms = a.terms;
            for t in b.terms {
                match terms.iter_mut().find(|e| std::ptr::eq(e.def, t.def) && std::ptr::eq(e.prefix, t.prefix)) {
                    Some(existing) => existing.power += t.power,
                    None => terms.push(t),
                }
            }
            terms.retain(|t| t.power != 0);
            Ok(Unit { terms })
        }
        Expr::Binary(BinOp::Pow, a, b) => {
            let power = match &**b {
                Expr::Num { value, .. } => Some(*value),
                Expr::Neg(inner) => match &**inner {
                    Expr::Num { value, .. } => Some(-value),
                    _ => None,
                },
                _ => None,
            };
            match power {
                Some(p) if p.fract() == 0.0 => Ok(unit_expr(a)?.powi(p as i32)),
                _ => Err("Unit powers must be whole numbers".into()),
            }
        }
        _ => Err("Expected a unit after \"to\", like km/h, or a format: hex, bin, oct, fraction, sci, percent".into()),
    }
}
