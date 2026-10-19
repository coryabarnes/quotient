//! Matrices and lists. Linear algebra uses nalgebra.

use nalgebra::DMatrix;
use num_complex::Complex64;

use super::{Apply, R, arity, count_arg, flatten, int, num, nums, real};
use crate::eval::{MAX_ELEMENTS, range};
use crate::functions::stats::sorted;
use crate::value::Value;

/// Rows of a rectangular numeric matrix; a list counts as one row if `allow_vector`.
fn grid(name: &str, v: &Value) -> Result<Vec<Vec<f64>>, String> {
    let Value::Matrix(rows) = v else {
        return Err(format!("{name} expects a matrix, got a {}", v.type_name()));
    };
    let parsed: Option<Vec<Vec<f64>>> = rows
        .iter()
        .map(|r| match r {
            Value::Matrix(cells) => cells.iter().map(Value::as_f64).collect(),
            _ => None,
        })
        .collect();
    let parsed = parsed.ok_or_else(|| format!("{name} expects a 2-dimensional matrix of numbers, like [1, 2; 3, 4]"))?;
    let width = parsed.first().map_or(0, Vec::len);
    if parsed.iter().any(|r| r.len() != width) {
        return Err(format!("{name}: all rows must have the same length"));
    }
    Ok(parsed)
}

fn vector(name: &str, v: &Value) -> Result<Vec<f64>, String> {
    match v {
        Value::Matrix(items) => {
            // Accept column or row matrices as vectors too.
            if items.iter().all(|i| matches!(i, Value::Matrix(_))) {
                let g = grid(name, v)?;
                if g.len() == 1 || g.iter().all(|r| r.len() == 1) {
                    return Ok(g.into_iter().flatten().collect());
                }
                return Err(format!("{name} expects a vector, got a matrix"));
            }
            items.iter().map(|i| i.as_f64().ok_or_else(|| format!("{name} expects numbers"))).collect()
        }
        other => Err(format!("{name} expects a list, got a {}", other.type_name())),
    }
}

fn to_na(rows: &[Vec<f64>]) -> DMatrix<f64> {
    let (r, c) = (rows.len(), rows.first().map_or(0, Vec::len));
    DMatrix::from_row_iterator(r, c, rows.iter().flatten().copied())
}

fn from_na(m: &DMatrix<f64>) -> Value {
    Value::Matrix((0..m.nrows()).map(|i| nums((0..m.ncols()).map(|j| m[(i, j)]))).collect())
}

fn square(name: &str, v: &Value) -> Result<DMatrix<f64>, String> {
    let g = grid(name, v)?;
    if g.len() != g.first().map_or(0, Vec::len) {
        return Err(format!("{name} needs a square matrix"));
    }
    Ok(to_na(&g))
}

/// Dimensions of a (possibly nested) list: `[2, 3]` for a 2×3 matrix.
fn shape(v: &Value) -> Vec<usize> {
    match v {
        Value::Matrix(items) => {
            let mut dims = vec![items.len()];
            if let Some(first) = items.first() {
                dims.extend(shape(first));
            }
            dims
        }
        _ => vec![],
    }
}

/// Builds a nested matrix of the given shape from a flat list.
fn build(dims: &[usize], items: &mut impl Iterator<Item = Value>) -> Value {
    match dims {
        [] => items.next().unwrap_or(Value::Num(0.0)),
        [n, rest @ ..] => Value::Matrix((0..*n).map(|_| build(rest, items)).collect()),
    }
}

fn dims_arg(name: &str, args: &[Value], i: usize) -> Result<Vec<usize>, String> {
    let list = match args.get(i) {
        Some(Value::Matrix(items)) => items.clone(),
        Some(v) => vec![v.clone()],
        None => return Err(format!("{name} needs dimensions")),
    };
    let dims: Vec<usize> = (0..list.len()).map(|j| count_arg(name, &list, j).map(|d| d as usize)).collect::<Result<_, _>>()?;
    if dims.iter().try_fold(1usize, |acc, d| acc.checked_mul(*d)).is_none_or(|n| n > MAX_ELEMENTS) {
        return Err(format!("{name}: at most {MAX_ELEMENTS} elements"));
    }
    Ok(dims)
}

fn filled(name: &str, args: &[Value], value: f64) -> R {
    arity(name, args, 1, 2)?;
    let dims = if args.len() == 2 {
        vec![count_arg(name, args, 0)? as usize, count_arg(name, args, 1)? as usize]
    } else {
        dims_arg(name, args, 0)?
    };
    if dims.iter().product::<usize>() > MAX_ELEMENTS {
        return Err(format!("{name}: at most {MAX_ELEMENTS} elements"));
    }
    Ok(build(&dims, &mut std::iter::repeat(Value::Num(value))))
}

fn transpose(v: &Value) -> R {
    let Value::Matrix(rows) = v else {
        return Ok(v.clone());
    };
    if !rows.iter().all(|r| matches!(r, Value::Matrix(_))) || rows.is_empty() {
        return Ok(v.clone()); // A list is its own transpose, as in mathjs.
    }
    let rows: Vec<&Vec<Value>> = rows.iter().map(|r| if let Value::Matrix(c) = r { c } else { unreachable!() }).collect();
    let width = rows[0].len();
    if rows.iter().any(|r| r.len() != width) {
        return Err("transpose: all rows must have the same length".into());
    }
    Ok(Value::Matrix((0..width).map(|j| Value::Matrix(rows.iter().map(|r| r[j].clone()).collect())).collect()))
}

/// Removes dimensions of size 1: `[[1, 2, 3]]` → `[1, 2, 3]`, `[[5]]` → `5`.
fn squeeze(v: Value) -> Value {
    match v {
        Value::Matrix(mut items) if items.len() == 1 => squeeze(items.remove(0)),
        Value::Matrix(items) if items.iter().all(|i| matches!(i, Value::Matrix(c) if c.len() == 1)) => {
            Value::Matrix(items.into_iter().map(squeeze).collect())
        }
        other => other,
    }
}

/// Pads with `fill` or truncates to the given dimensions.
fn resize(v: &Value, dims: &[usize], fill: &Value) -> Value {
    match dims {
        [] => match v {
            Value::Matrix(_) => fill.clone(),
            other => other.clone(),
        },
        [n, rest @ ..] => {
            let items: Vec<Value> = match v {
                Value::Matrix(items) => items.clone(),
                scalar => vec![scalar.clone()],
            };
            Value::Matrix(
                (0..*n)
                    .map(|i| match items.get(i) {
                        Some(item) => resize(item, rest, fill),
                        None => build(rest, &mut std::iter::repeat(fill.clone())),
                    })
                    .collect(),
            )
        }
    }
}

fn concat(args: &[Value]) -> R {
    let (lists, dim) = match args.last() {
        Some(v) if args.len() > 2 && !matches!(v, Value::Matrix(_)) => (&args[..args.len() - 1], Some(int("concat", args, args.len() - 1)?)),
        _ => (args, None),
    };
    if lists.len() < 2 {
        return Err("concat needs at least two lists".into());
    }
    // Stacking rows: a plain list next to matrices counts as one row.
    let max_depth = lists.iter().map(|l| shape(l).len()).max().unwrap_or(0);
    let promoted: Vec<Value>;
    let lists = if dim == Some(1) && max_depth == 2 {
        promoted = lists.iter().map(|l| if shape(l).len() == 1 { Value::Matrix(vec![l.clone()]) } else { l.clone() }).collect();
        &promoted[..]
    } else {
        lists
    };
    let depth = shape(&lists[0]).len();
    if lists.iter().any(|l| shape(l).len() != depth) || depth == 0 {
        return Err("concat: all arguments must be lists with the same number of dimensions".into());
    }
    let dim = dim.unwrap_or(depth as i64);
    if dim < 1 || dim as usize > depth {
        return Err(format!("concat: dimension must be between 1 and {depth}"));
    }
    if dim == 1 {
        let mut out = Vec::new();
        for l in lists {
            if let Value::Matrix(items) = l {
                out.extend(items.iter().cloned());
            }
        }
        if depth == 2 {
            let width = shape(&out[0]).first().copied().unwrap_or(0);
            if out.iter().any(|r| shape(r).first().copied().unwrap_or(0) != width) {
                return Err("concat: rows must have the same length".into());
            }
        }
        return Ok(Value::Matrix(out));
    }
    // Concatenate along columns: join row by row.
    let rows: Vec<&Vec<Value>> = lists.iter().map(|l| if let Value::Matrix(r) = l { r } else { unreachable!() }).collect();
    let n = rows[0].len();
    if rows.iter().any(|r| r.len() != n) {
        return Err("concat: matrices must have the same number of rows".into());
    }
    (0..n)
        .map(|i| {
            let parts: Vec<Value> = rows.iter().map(|r| r[i].clone()).collect();
            concat(&parts)
        })
        .collect::<Result<_, _>>()
        .map(Value::Matrix)
}

fn norm(args: &[Value]) -> R {
    arity("norm", args, 1, 2)?;
    let p = match args.get(1) {
        None => None,
        Some(Value::Str(s)) if s == "fro" => None,
        Some(_) => Some(real("norm", args, 1)?),
    };
    let x = &args[0];
    if let Value::Matrix(items) = x
        && items.iter().all(|i| matches!(i, Value::Matrix(_)))
        && !items.is_empty()
    {
        let g = grid("norm", x)?;
        return num(match p {
            None => g.iter().flatten().map(|v| v * v).sum::<f64>().sqrt(),
            Some(1.0) => (0..g[0].len()).map(|j| g.iter().map(|r| r[j].abs()).sum::<f64>()).fold(0.0, f64::max),
            Some(p) if p == f64::INFINITY => g.iter().map(|r| r.iter().map(|v| v.abs()).sum::<f64>()).fold(0.0, f64::max),
            Some(2.0) => {
                let svd = to_na(&g).svd(false, false);
                svd.singular_values.iter().copied().fold(0.0, f64::max)
            }
            Some(_) => return Err("norm of a matrix supports p = 1, 2, Infinity or \"fro\"".into()),
        });
    }
    if let Some(z) = x.as_complex().filter(|_| !matches!(x, Value::Matrix(_))) {
        return num(z.norm());
    }
    let v = vector("norm", x)?;
    num(match p.unwrap_or(2.0) {
        p if p == f64::INFINITY => v.iter().map(|a| a.abs()).fold(0.0, f64::max),
        p if p == f64::NEG_INFINITY => v.iter().map(|a| a.abs()).fold(f64::INFINITY, f64::min),
        p if p > 0.0 => v.iter().map(|a| a.abs().powf(p)).sum::<f64>().powf(1.0 / p),
        _ => return Err("norm: p must be positive or ±Infinity".into()),
    })
}

fn eigs(v: &Value) -> R {
    let m = square("eigs", v)?;
    if m.nrows() == 0 {
        return Ok(Value::Matrix(vec![]));
    }
    let scale = m.iter().fold(0.0f64, |a, x| a.max(x.abs())).max(1.0);
    let symmetric = (0..m.nrows()).all(|i| (0..i).all(|j| (m[(i, j)] - m[(j, i)]).abs() <= 1e-12 * scale));
    if symmetric {
        let mut values: Vec<f64> = m.symmetric_eigen().eigenvalues.iter().copied().collect();
        values.sort_by(f64::total_cmp);
        return Ok(nums(values));
    }
    let mut values: Vec<Complex64> = m.complex_eigenvalues().iter().copied().collect();
    let clean = |x: f64| if x.abs() < 1e-12 * scale { 0.0 } else { x };
    values.iter_mut().for_each(|z| *z = Complex64::new(clean(z.re), clean(z.im)));
    values.sort_by(|a, b| a.re.total_cmp(&b.re).then(a.im.total_cmp(&b.im)));
    Ok(Value::Matrix(values.into_iter().map(Value::from_complex).collect()))
}

/// Solves A x = b with `solve`, returning a list when b is a list.
fn solve(name: &str, args: &[Value], solve: fn(&DMatrix<f64>, &DMatrix<f64>) -> Option<DMatrix<f64>>) -> R {
    arity(name, args, 2, 2)?;
    let a = square(name, &args[0])?;
    let b_is_list = matches!(&args[1], Value::Matrix(items) if items.iter().all(|i| !matches!(i, Value::Matrix(_))));
    let b = if b_is_list {
        let v = vector(name, &args[1])?;
        DMatrix::from_column_slice(v.len(), 1, &v)
    } else {
        to_na(&grid(name, &args[1])?)
    };
    if b.nrows() != a.nrows() {
        return Err(format!("{name}: b must have {} rows", a.nrows()));
    }
    let x = solve(&a, &b).ok_or_else(|| format!("{name}: the system has no unique solution (the matrix is singular)"))?;
    if x.iter().any(|v| !v.is_finite()) {
        return Err(format!("{name}: the system has no unique solution (the matrix is singular)"));
    }
    Ok(if b_is_list { nums(x.column(0).iter().copied()) } else { from_na(&x) })
}

/// `map` and `filter` call back into user functions.
pub fn call_with(name: &str, args: &[Value], apply: Apply) -> Option<R> {
    Some(match name {
        "map" => (|| {
            arity(name, args, 2, 2)?;
            fn go(v: &Value, f: &Value, apply: &mut dyn FnMut(&Value, Vec<Value>) -> R) -> R {
                match v {
                    Value::Matrix(items) => items.iter().map(|i| go(i, f, apply)).collect::<Result<_, _>>().map(Value::Matrix),
                    other => apply(f, vec![other.clone()]),
                }
            }
            go(&args[0], &args[1], apply)
        })(),
        "filter" => (|| {
            arity(name, args, 2, 2)?;
            let Value::Matrix(items) = &args[0] else {
                return Err("filter expects a list".into());
            };
            let mut kept = Vec::new();
            for item in items {
                if apply(&args[1], vec![item.clone()])?.truthy()? {
                    kept.push(item.clone());
                }
            }
            Ok(Value::Matrix(kept))
        })(),
        _ => return None,
    })
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    Some(match name {
        "matrix" => arity(name, args, 0, 1).map(|_| match args.first() {
            None => Value::Matrix(vec![]),
            Some(m @ Value::Matrix(_)) => m.clone(),
            Some(other) => Value::Matrix(vec![other.clone()]),
        }),
        "zeros" => filled(name, args, 0.0),
        "ones" => filled(name, args, 1.0),
        "identity" => (|| {
            arity(name, args, 1, 2)?;
            let r = count_arg(name, args, 0)? as usize;
            let c = if args.len() == 2 { count_arg(name, args, 1)? as usize } else { r };
            if r.saturating_mul(c) > MAX_ELEMENTS {
                return Err(format!("{name}: at most {MAX_ELEMENTS} elements"));
            }
            Ok(Value::Matrix((0..r).map(|i| nums((0..c).map(|j| if i == j { 1.0 } else { 0.0 }))).collect()))
        })(),
        "range" => (|| {
            arity(name, args, 2, 3)?;
            range(args[0].clone(), args.get(2).cloned(), args[1].clone(), false)
        })(),
        "size" => arity(name, args, 1, 1).map(|_| match &args[0] {
            Value::Str(s) => nums([s.chars().count() as f64]),
            v => nums(shape(v).into_iter().map(|d| d as f64)),
        }),
        "transpose" => arity(name, args, 1, 1).and_then(|_| transpose(&args[0])),
        "det" => arity(name, args, 1, 1).and_then(|_| {
            if let Some(x) = args[0].as_f64() {
                return num(x);
            }
            let m = square(name, &args[0])?;
            num(if m.nrows() == 0 { 1.0 } else { m.determinant() })
        }),
        "inv" => arity(name, args, 1, 1).and_then(|_| {
            if let Some(x) = args[0].as_f64() {
                return num(1.0 / x);
            }
            let m = square(name, &args[0])?;
            let inv = m.try_inverse().ok_or("Cannot calculate inverse, the determinant is zero")?;
            Ok(from_na(&inv))
        }),
        "pinv" => arity(name, args, 1, 1).and_then(|_| {
            let m = to_na(&grid(name, &args[0])?);
            let p = m.pseudo_inverse(1e-12).map_err(|e| format!("pinv: {e}"))?;
            Ok(from_na(&p))
        }),
        "trace" => arity(name, args, 1, 1).and_then(|_| num(square(name, &args[0])?.trace())),
        "diag" => arity(name, args, 1, 1).and_then(|_| match &args[0] {
            Value::Matrix(items) if items.iter().all(|i| matches!(i, Value::Matrix(_))) && !items.is_empty() => {
                let g = grid(name, &args[0])?;
                let n = g.len().min(g[0].len());
                Ok(nums((0..n).map(|i| g[i][i])))
            }
            v => {
                let d = vector(name, v)?;
                Ok(Value::Matrix((0..d.len()).map(|i| nums((0..d.len()).map(|j| if i == j { d[i] } else { 0.0 }))).collect()))
            }
        }),
        "dot" => arity(name, args, 2, 2).and_then(|_| {
            let (a, b) = (vector(name, &args[0])?, vector(name, &args[1])?);
            if a.len() != b.len() {
                return Err(format!("dot: vectors have different lengths ({} and {})", a.len(), b.len()));
            }
            num(a.iter().zip(&b).map(|(x, y)| x * y).sum())
        }),
        "cross" => arity(name, args, 2, 2).and_then(|_| {
            let (a, b) = (vector(name, &args[0])?, vector(name, &args[1])?);
            if a.len() != 3 || b.len() != 3 {
                return Err("cross needs two vectors of length 3".into());
            }
            Ok(nums([a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]))
        }),
        "kron" => arity(name, args, 2, 2).and_then(|_| {
            let as_grid = |v: &Value| match shape(v).len() {
                1 => vector(name, v).map(|r| vec![r]),
                _ => grid(name, v),
            };
            let both_lists = shape(&args[0]).len() == 1 && shape(&args[1]).len() == 1;
            let k = to_na(&as_grid(&args[0])?).kronecker(&to_na(&as_grid(&args[1])?));
            Ok(if both_lists { nums(k.row(0).iter().copied()) } else { from_na(&k) })
        }),
        "eigs" => arity(name, args, 1, 1).and_then(|_| eigs(&args[0])),
        "lusolve" => solve(name, args, |a, b| a.clone().lu().solve(b)),
        "lsolve" => solve(name, args, |a, b| a.solve_lower_triangular(b)),
        "usolve" => solve(name, args, |a, b| a.solve_upper_triangular(b)),
        "flatten" => arity(name, args, 1, 1).map(|_| Value::Matrix(flatten(args))),
        "reshape" => arity(name, args, 2, 2).and_then(|_| {
            let items = flatten(&args[..1]);
            let dims = dims_arg(name, args, 1)?;
            if dims.iter().product::<usize>() != items.len() {
                return Err(format!("reshape: cannot fit {} elements into shape {:?}", items.len(), dims));
            }
            Ok(build(&dims, &mut items.into_iter()))
        }),
        "resize" => arity(name, args, 2, 3).and_then(|_| {
            let dims = dims_arg(name, args, 1)?;
            let fill = args.get(2).cloned().unwrap_or(Value::Num(0.0));
            Ok(resize(&args[0], &dims, &fill))
        }),
        "squeeze" => arity(name, args, 1, 1).map(|_| squeeze(args[0].clone())),
        "concat" => concat(args),
        "row" | "column" => arity(name, args, 2, 2).and_then(|_| {
            let g = grid(name, &args[0])?;
            let i = count_arg(name, args, 1)? as usize;
            let len = if name == "row" { g.len() } else { g.first().map_or(0, Vec::len) };
            if i < 1 || i > len {
                return Err(format!("{name}: index {i} is out of range (1 to {len})"));
            }
            Ok(if name == "row" { nums(g[i - 1].clone()) } else { nums(g.iter().map(|r| r[i - 1])) })
        }),
        "sort" => arity(name, args, 1, 2).and_then(|_| {
            let descending = match args.get(1) {
                None => false,
                Some(Value::Str(s)) if s == "asc" => false,
                Some(Value::Str(s)) if s == "desc" => true,
                Some(_) => return Err("sort: order must be \"asc\" or \"desc\"".into()),
            };
            let mut s = sorted(flatten(&args[..1]))?;
            if descending {
                s.reverse();
            }
            Ok(Value::Matrix(s))
        }),
        "reverse" => arity(name, args, 1, 1).and_then(|_| match &args[0] {
            Value::Matrix(items) => Ok(Value::Matrix(items.iter().rev().cloned().collect())),
            Value::Str(s) => Ok(Value::Str(s.chars().rev().collect())),
            other => Err(format!("reverse expects a list, got a {}", other.type_name())),
        }),
        "partitionSelect" => arity(name, args, 2, 2).and_then(|_| {
            let s = sorted(flatten(&args[..1]))?;
            let k = count_arg(name, args, 1)? as usize;
            s.get(k).cloned().ok_or_else(|| format!("partitionSelect: k must be less than {}", s.len()))
        }),
        "norm" => norm(args),
        _ => return None,
    })
}
