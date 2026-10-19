//! Set operations on lists of numbers. Results are sorted and distinct.

use super::{R, arity, flat_reals, num, nums};
use crate::value::Value;

fn set(name: &str, v: &Value) -> Result<Vec<f64>, String> {
    let mut xs = flat_reals(name, std::slice::from_ref(v))?;
    if xs.iter().any(|x| x.is_nan()) {
        return Err(format!("{name}: sets cannot contain NaN"));
    }
    xs.sort_by(f64::total_cmp);
    xs.dedup();
    Ok(xs)
}

fn pair(name: &str, args: &[Value]) -> Result<(Vec<f64>, Vec<f64>), String> {
    arity(name, args, 2, 2)?;
    Ok((set(name, &args[0])?, set(name, &args[1])?))
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    Some(match name {
        "setUnion" => pair(name, args).map(|(a, b)| {
            let mut u = [a, b].concat();
            u.sort_by(f64::total_cmp);
            u.dedup();
            nums(u)
        }),
        "setIntersect" => pair(name, args).map(|(a, b)| nums(a.into_iter().filter(|x| b.contains(x)))),
        "setDifference" => pair(name, args).map(|(a, b)| nums(a.into_iter().filter(|x| !b.contains(x)))),
        "setSymDifference" => pair(name, args).map(|(a, b)| {
            let mut d: Vec<f64> = a.iter().filter(|x| !b.contains(x)).chain(b.iter().filter(|x| !a.contains(x))).copied().collect();
            d.sort_by(f64::total_cmp);
            nums(d)
        }),
        "setDistinct" => arity(name, args, 1, 1).and_then(|_| set(name, &args[0]).map(nums)),
        "setIsSubset" => pair(name, args).map(|(a, b)| Value::Bool(a.iter().all(|x| b.contains(x)))),
        "setSize" => arity(name, args, 1, 2).and_then(|_| {
            let distinct = match args.get(1) {
                Some(v) => v.truthy()?,
                None => false,
            };
            if distinct {
                num(set(name, &args[0])?.len() as f64)
            } else {
                num(flat_reals(name, &args[..1])?.len() as f64)
            }
        }),
        "setMultiplicity" => arity(name, args, 2, 2).and_then(|_| {
            let x = super::real(name, args, 0)?;
            num(flat_reals(name, &args[1..])?.iter().filter(|v| **v == x).count() as f64)
        }),
        "setCartesian" => pair(name, args).and_then(|(a, b)| {
            if a.len().saturating_mul(b.len()) > crate::eval::MAX_ELEMENTS {
                return Err("setCartesian: result too large".into());
            }
            Ok(Value::Matrix(a.iter().flat_map(|x| b.iter().map(move |y| nums([*x, *y]))).collect()))
        }),
        "setPowerset" => arity(name, args, 1, 1).and_then(|_| {
            let a = set(name, &args[0])?;
            if a.len() > 16 {
                return Err("setPowerset: at most 16 distinct values (65536 subsets)".into());
            }
            let mut subsets: Vec<Vec<f64>> = (0u32..1 << a.len())
                .map(|mask| a.iter().enumerate().filter(|(i, _)| mask & (1 << i) != 0).map(|(_, x)| *x).collect())
                .collect();
            subsets.sort_by(|p, q| p.len().cmp(&q.len()).then_with(|| p.iter().zip(q).map(|(x, y)| x.total_cmp(y)).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal)));
            Ok(Value::Matrix(subsets.into_iter().map(nums).collect()))
        }),
        _ => return None,
    })
}
