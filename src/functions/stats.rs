//! Statistics. Sums, means, extremes and medians use the general operators, so
//! they work with units, complex numbers and high precision alike.

use std::cmp::Ordering;

use super::{R, arity, flat_reals, flatten, num, real};
use crate::eval::binary;
use crate::format::{FormatOptions, format_f64};
use crate::parser::BinOp;
use crate::units::Quantity;
use crate::value::Value;

/// Orders two values with `<`, so quantities in different units compare correctly.
pub(crate) fn compare(a: &Value, b: &Value) -> Result<Ordering, String> {
    if binary(BinOp::Lt, a.clone(), b.clone())?.truthy()? {
        Ok(Ordering::Less)
    } else if binary(BinOp::Gt, a.clone(), b.clone())?.truthy()? {
        Ok(Ordering::Greater)
    } else {
        Ok(Ordering::Equal)
    }
}

/// Sorts values, reporting the first comparison that fails.
pub(crate) fn sorted(mut values: Vec<Value>) -> Result<Vec<Value>, String> {
    let mut error = None;
    values.sort_by(|a, b| {
        compare(a, b).unwrap_or_else(|e| {
            error.get_or_insert(e);
            Ordering::Equal
        })
    });
    match error {
        Some(e) => Err(e),
        None => Ok(values),
    }
}

fn nonempty(name: &str, args: &[Value]) -> Result<Vec<Value>, String> {
    let values = flatten(args);
    if values.is_empty() {
        return Err(format!("Cannot calculate {name} of an empty list"));
    }
    Ok(values)
}

fn sum(values: Vec<Value>) -> R {
    let mut iter = values.into_iter();
    let Some(mut acc) = iter.next() else {
        return num(0.0);
    };
    for v in iter {
        acc = binary(BinOp::Add, acc, v)?;
    }
    Ok(acc)
}

fn mean(name: &str, args: &[Value]) -> R {
    let values = nonempty(name, args)?;
    let n = values.len() as f64;
    binary(BinOp::Div, sum(values)?, Value::Num(n))
}

fn extreme(name: &str, args: &[Value], want: Ordering) -> R {
    let mut values = nonempty(name, args)?.into_iter();
    let mut best = values.next().expect("nonempty");
    for v in values {
        if v.as_f64().is_some_and(f64::is_nan) {
            return num(f64::NAN);
        }
        if compare(&v, &best)? == want {
            best = v;
        }
    }
    Ok(best)
}

fn median_of(values: Vec<Value>) -> R {
    let s = sorted(values)?;
    let mid = s.len() / 2;
    if s.len() % 2 == 1 {
        return Ok(s[mid].clone());
    }
    binary(BinOp::Div, binary(BinOp::Add, s[mid - 1].clone(), s[mid].clone())?, Value::Num(2.0))
}

/// Numbers, or quantities converted to the first one's unit (returned for re-wrapping).
fn reals_or_quantity(name: &str, args: &[Value]) -> Result<(Vec<f64>, Option<Quantity>), String> {
    let values = flatten(args);
    if let Some(Value::Quantity(first)) = values.first() {
        let first = first.clone();
        let xs = values
            .iter()
            .map(|v| match v {
                Value::Quantity(q) => first.value_of(q),
                _ => Err(format!("{name}: either all or none of the values must have units")),
            })
            .collect::<Result<_, _>>()?;
        return Ok((xs, Some(first)));
    }
    Ok((flat_reals(name, args)?, None))
}

fn wrap(x: f64, like: &Option<Quantity>) -> Value {
    match like {
        Some(q) => super::quantity(x, q),
        None => Value::Num(x),
    }
}

fn mean_f64(v: &[f64]) -> f64 {
    v.iter().sum::<f64>() / v.len() as f64
}

/// Sum of squared deviations divided by n + `offset` (-1 sample, 0 population).
fn variance_f64(v: &[f64], offset: f64) -> f64 {
    let m = mean_f64(v);
    v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() as f64 + offset)
}

/// Splits off a trailing normalization word: "unbiased" (n - 1), "uncorrected" (n), "biased" (n + 1).
fn normalization(name: &str, args: &[Value], default: f64) -> Result<(Vec<Value>, f64), String> {
    if let Some(Value::Str(s)) = args.last() {
        let offset = match s.as_str() {
            "unbiased" => -1.0,
            "uncorrected" => 0.0,
            "biased" => 1.0,
            other => return Err(format!("{name}: unknown normalization \"{other}\" (use unbiased, uncorrected or biased)")),
        };
        return Ok((args[..args.len() - 1].to_vec(), offset));
    }
    Ok((args.to_vec(), default))
}

fn spread(name: &str, args: &[Value], default: f64, sqrt: bool) -> R {
    let (args, offset) = normalization(name, args, default)?;
    let (xs, unit) = reals_or_quantity(name, &args)?;
    if xs.is_empty() {
        // The original returned 0 for stddev() and variance() of nothing.
        return num(0.0);
    }
    if xs.len() as f64 + offset <= 0.0 {
        return Err(format!("{name} needs at least 2 values for the sample (n - 1) version"));
    }
    let var = variance_f64(&xs, offset);
    if sqrt {
        return Ok(wrap(var.sqrt(), &unit));
    }
    match unit {
        Some(q) => Ok(Value::Quantity(Quantity { value: var, unit: q.unit.powi(2) })),
        None => num(var),
    }
}

fn quantile(name: &str, args: &[Value], scale: f64) -> R {
    arity(name, args, 2, 2)?;
    let (xs, unit) = reals_or_quantity(name, &args[..1])?;
    if xs.is_empty() {
        return Err(format!("Cannot calculate {name} of an empty list"));
    }
    let mut s = xs;
    s.sort_by(f64::total_cmp);
    let at = |p: f64| -> Result<Value, String> {
        let p = p / scale;
        if !(0.0..=1.0).contains(&p) {
            return Err(format!("{name}: probability must be between 0 and {scale}"));
        }
        let pos = p * (s.len() - 1) as f64;
        let lo = pos.floor() as usize;
        let hi = pos.ceil() as usize;
        Ok(wrap(s[lo] + (pos - lo as f64) * (s[hi] - s[lo]), &unit))
    };
    match &args[1] {
        Value::Matrix(ps) => ps
            .iter()
            .map(|p| at(p.as_f64().ok_or_else(|| format!("{name}: probabilities must be numbers"))?))
            .collect::<Result<_, _>>()
            .map(Value::Matrix),
        _ => at(real(name, args, 1)?),
    }
}

fn mode(name: &str, args: &[Value]) -> R {
    let mut xs = flat_reals(name, args)?;
    if xs.is_empty() {
        return Err(format!("Cannot calculate {name} of an empty list"));
    }
    xs.sort_by(f64::total_cmp);
    let mut best: Vec<f64> = Vec::new();
    let mut best_count = 0;
    for run in xs.chunk_by(|a, b| a == b) {
        if run.len() > best_count {
            best_count = run.len();
            best.clear();
        }
        if run.len() == best_count {
            best.push(run[0]);
        }
    }
    Ok(if best.len() == 1 { Value::Num(best[0]) } else { super::nums(best) })
}

fn trend(args: &[Value]) -> R {
    let v = flat_reals("trend", args)?;
    if v.len() < 2 {
        return Err("trend needs at least 2 values".into());
    }
    let f = |x: f64| format_f64(x, FormatOptions::default());
    let joined: Vec<String> = v.iter().map(|x| f(*x)).collect();
    let mut lines = vec![format!("Values: [{}]", joined.join(", "))];
    let diffs: Vec<f64> = v.windows(2).map(|w| w[1] - w[0]).collect();
    for (w, d) in v.windows(2).zip(&diffs) {
        let sign = if *d >= 0.0 { "+" } else { "" };
        lines.push(format!("{} → {} ({sign}{})", f(w[0]), f(w[1]), f(*d)));
    }
    let max = diffs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let min = diffs.iter().copied().fold(f64::INFINITY, f64::min);
    lines.push(format!("Average change: {:.3}", mean_f64(&diffs)));
    lines.push(format!("Max change: {}", f(max)));
    lines.push(format!("Min change: {}", f(min)));
    Ok(Value::Str(lines.join("\n")))
}

fn sma(args: &[Value]) -> R {
    arity("sma", args, 1, usize::MAX)?;
    let periods = super::count_arg("sma", args, 0)?;
    if periods == 0 {
        return Err("sma: periods must be at least 1".into());
    }
    let v = flat_reals("sma", &args[1..])?;
    let periods = periods as usize;
    if v.len() < periods {
        return Err(format!("sma: need at least {periods} values for a {periods}-period moving average"));
    }
    let lines: Vec<String> =
        v.windows(periods).enumerate().map(|(i, w)| format!("Period {}: {:.3}", i + periods, mean_f64(w))).collect();
    Ok(Value::Str(lines.join("\n")))
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    Some(match name {
        "sum" => sum(flatten(args)),
        "prod" => flatten(args).into_iter().try_fold(Value::Num(1.0), |acc, v| binary(BinOp::Mul, acc, v)),
        "count" => num(flatten(args).len() as f64),
        "mean" => mean(name, args),
        "median" => nonempty(name, args).and_then(median_of),
        "min" => extreme(name, args, Ordering::Less),
        "max" => extreme(name, args, Ordering::Greater),
        "mode" => mode(name, args),
        "std" => (|| {
            let (rest, _) = normalization(name, args, -1.0)?;
            if flatten(&rest).is_empty() {
                return Err("Cannot calculate std of an empty list".into());
            }
            spread(name, args, -1.0, true)
        })(),
        "stddev" => spread(name, args, 0.0, true),
        "variance" => spread(name, args, 0.0, false),
        "mad" => (|| {
            let (xs, unit) = reals_or_quantity(name, args)?;
            if xs.is_empty() {
                return Err("Cannot calculate mad of an empty list".into());
            }
            let med = |mut v: Vec<f64>| {
                v.sort_by(f64::total_cmp);
                let mid = v.len() / 2;
                if v.len() % 2 == 1 { v[mid] } else { (v[mid - 1] + v[mid]) / 2.0 }
            };
            let m = med(xs.clone());
            Ok(wrap(med(xs.iter().map(|x| (x - m).abs()).collect()), &unit))
        })(),
        "quantileSeq" => quantile(name, args, 1.0),
        "percentile" => quantile(name, args, 100.0),
        "cumsum" => (|| {
            let mut total: Option<Value> = None;
            let mut out = Vec::new();
            for v in flatten(args) {
                let next = match total.take() {
                    Some(t) => binary(BinOp::Add, t, v)?,
                    None => v,
                };
                out.push(next.clone());
                total = Some(next);
            }
            Ok(Value::Matrix(out))
        })(),
        "rangeCalc" => (|| {
            let (xs, unit) = reals_or_quantity(name, args)?;
            if xs.is_empty() {
                return num(0.0);
            }
            let max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let min = xs.iter().copied().fold(f64::INFINITY, f64::min);
            Ok(wrap(max - min, &unit))
        })(),
        "geomean" => (|| {
            let xs = flat_reals(name, args)?;
            if xs.is_empty() {
                return num(0.0);
            }
            if xs.iter().any(|x| *x <= 0.0) {
                return Err("Geometric mean requires positive values".into());
            }
            // Sum logarithms rather than multiplying, which can overflow.
            num((xs.iter().map(|x| x.ln()).sum::<f64>() / xs.len() as f64).exp())
        })(),
        "harmean" => (|| {
            let xs = flat_reals(name, args)?;
            if xs.is_empty() {
                return num(0.0);
            }
            if xs.contains(&0.0) {
                return Err("Harmonic mean is undefined when a value is zero".into());
            }
            num(xs.len() as f64 / xs.iter().map(|x| 1.0 / x).sum::<f64>())
        })(),
        "rms" => (|| {
            let (xs, unit) = reals_or_quantity(name, args)?;
            if xs.is_empty() {
                return num(0.0);
            }
            Ok(wrap((xs.iter().map(|x| x * x).sum::<f64>() / xs.len() as f64).sqrt(), &unit))
        })(),
        "trend" => trend(args),
        "sma" => sma(args),
        "kldivergence" => (|| {
            arity(name, args, 2, 2)?;
            let p = flat_reals(name, &args[..1])?;
            let q = flat_reals(name, &args[1..])?;
            if p.len() != q.len() || p.is_empty() {
                return Err("kldivergence needs two lists of the same length".into());
            }
            let (sp, sq) = (p.iter().sum::<f64>(), q.iter().sum::<f64>());
            let mut total = 0.0;
            for (a, b) in p.iter().zip(&q) {
                let (a, b) = (a / sp, b / sq);
                if a > 0.0 {
                    if b <= 0.0 {
                        return Err("kldivergence: q is zero where p is not".into());
                    }
                    total += a * (a / b).ln();
                }
            }
            num(total)
        })(),
        _ => return None,
    })
}
