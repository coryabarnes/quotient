//! The values expressions evaluate to.

use std::fmt;
use std::rc::Rc;

use num_complex::Complex64;

use crate::big::{self, BigFloat};
use crate::format::{FormatOptions, format_f64};
use crate::parser::Expr;
use crate::units::Quantity;

#[derive(Debug, Clone)]
pub enum Value {
    Num(f64),
    /// High-precision number (only produced while high precision is on).
    Big(BigFloat),
    Complex(Complex64),
    /// A number with units, e.g. `5 km`.
    Quantity(Quantity),
    Bool(bool),
    Str(String),
    Matrix(Vec<Value>),
    /// A user-defined function, `f(x) = x^2`.
    Func(Rc<UserFn>),
    /// A built-in function used as a value, e.g. `map(data, sqrt)`.
    Builtin(&'static str),
}

#[derive(Debug)]
pub struct UserFn {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
    pub body_src: String,
}

impl Value {
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) | Value::Big(_) => "number",
            Value::Complex(_) => "complex number",
            Value::Quantity(_) => "quantity with units",
            Value::Bool(_) => "boolean",
            Value::Str(_) => "text",
            Value::Matrix(_) => "matrix",
            Value::Func(_) | Value::Builtin(_) => "function",
        }
    }

    /// A real number, if this is one (booleans count as 0/1, like mathjs).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            Value::Big(b) => Some(big::to_f64(b)),
            Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    pub fn as_complex(&self) -> Option<Complex64> {
        match self {
            Value::Complex(c) => Some(*c),
            other => other.as_f64().map(|x| Complex64::new(x, 0.0)),
        }
    }

    /// Whether this value counts as true in `and`, `or`, `not` and `? :`.
    pub fn truthy(&self) -> Result<bool, String> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Num(n) => Ok(*n != 0.0 && !n.is_nan()),
            Value::Big(b) => Ok(!b.is_zero() && !b.is_nan()),
            Value::Complex(c) => Ok(c.re != 0.0 || c.im != 0.0),
            Value::Quantity(q) => Ok(q.value != 0.0),
            Value::Str(s) => Ok(!s.is_empty()),
            other => Err(format!("A {} cannot be used as true or false", other.type_name())),
        }
    }

    /// Complex results with no imaginary part become plain numbers.
    pub fn from_complex(c: Complex64) -> Value {
        if c.im == 0.0 { Value::Num(c.re) } else { Value::Complex(c) }
    }

    pub fn format(&self, opts: FormatOptions) -> String {
        match self {
            Value::Num(n) => format_f64(*n, opts),
            Value::Big(b) => big::format(b, big::digits().unwrap_or(64), opts.thousands),
            Value::Complex(c) => format_complex(*c, opts),
            Value::Quantity(q) => q.format(opts),
            Value::Bool(b) => b.to_string(),
            Value::Str(s) => s.clone(),
            Value::Matrix(items) => {
                let parts: Vec<String> = items.iter().map(|v| v.format(opts)).collect();
                format!("[{}]", parts.join(", "))
            }
            Value::Func(f) => f.body_src.clone(),
            Value::Builtin(name) => format!("{name}(...)"),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.format(FormatOptions::FULL))
    }
}

/// `3 + 4i`, `2 - i`, `-3i`.
pub fn format_complex(c: Complex64, opts: FormatOptions) -> String {
    let im_part = |im: f64| match format_f64(im, opts).as_str() {
        "1" => "i".to_string(),
        "-1" => "-i".to_string(),
        s => format!("{s}i"),
    };
    // When rounding for display, a part smaller than the shown precision is noise,
    // e.g. the 1.2e-16 in exp(i pi) = -1 + 1.2e-16i.
    let negligible = |part: f64| match opts.digits {
        Some(d) => part.abs() < c.re.abs().max(c.im.abs()) * 10f64.powi(-(d as i32)),
        None => false,
    };
    if c.im == 0.0 || negligible(c.im) {
        return format_f64(c.re, opts);
    }
    if c.re == 0.0 || negligible(c.re) {
        return im_part(c.im);
    }
    let sign = if c.im < 0.0 { "-" } else { "+" };
    format!("{} {sign} {}", format_f64(c.re, opts), im_part(c.im.abs()))
}
