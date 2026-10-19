//! Factorials, combinatorics, number theory and random numbers.

use std::cell::Cell;

use super::{R, arity, count_arg, each, flatten, int, num, real};
use crate::big::{self, BigFloat, RM};
use crate::functions::math::gamma;
use crate::value::Value;

/// An exact integer result: full digits in high precision, else the nearest f64.
fn exact(n: u128) -> Value {
    match big::digits() {
        Some(_) => Value::Big(big::parse(&n.to_string())),
        None => Value::Num(n as f64),
    }
}

fn factorial(v: Value) -> R {
    match v {
        Value::Big(b) if b.is_int() => {
            let n = big::to_i64(&b).ok_or("factorial: number too large")?;
            if n < 0 {
                return Err("Factorial is undefined for negative integers".into());
            }
            if n > 10_000 {
                return Err("factorial: n must be at most 10000 in high precision".into());
            }
            let p = big::bits();
            let mut acc = BigFloat::from_i64(1, p);
            for i in 2..=n {
                acc = acc.mul(&BigFloat::from_i64(i, p), p, RM);
            }
            Ok(Value::Big(acc))
        }
        other => {
            let n = other.as_f64().ok_or_else(|| format!("factorial expects a number, got a {}", other.type_name()))?;
            if n.fract() == 0.0 {
                if n < 0.0 {
                    return Err("Factorial is undefined for negative integers".into());
                }
                if n > 170.0 {
                    return num(f64::INFINITY);
                }
                return num((2..=n as u64).map(|i| i as f64).product());
            }
            num(gamma(n + 1.0))
        }
    }
}

/// n choose k, exactly when it fits in 128 bits.
fn choose(n: u64, k: u64) -> Option<u128> {
    if k > n {
        return Some(0);
    }
    let k = k.min(n - k);
    let mut r: u128 = 1;
    for i in 1..=k as u128 {
        // r is C(n-k+i-1, i-1), so r * (n-k+i) is divisible by i.
        r = r.checked_mul(n as u128 - k as u128 + i)? / i;
    }
    Some(r)
}

fn choose_f64(n: u64, k: u64) -> f64 {
    let k = k.min(n - k);
    (1..=k).fold(1.0, |acc, i| acc * (n - k + i) as f64 / i as f64).round()
}

fn binomial(n: u64, k: u64) -> Value {
    match choose(n, k) {
        Some(r) => exact(r),
        None => Value::Num(choose_f64(n, k)),
    }
}

/// Row n of Stirling numbers of the second kind, S(n, 0..=n): exact while it
/// fits in 128 bits, and always as f64.
fn stirling_row(n: u64) -> Result<(Option<Vec<u128>>, Vec<f64>), String> {
    if n > 1000 {
        return Err("n must be at most 1000".into());
    }
    // Recurrence S(n, k) = k S(n-1, k) + S(n-1, k-1).
    let mut exact_row: Option<Vec<u128>> = Some(vec![1]);
    let mut float_row = vec![1.0f64];
    for i in 1..=n as usize {
        let mut next_f = vec![0.0; i + 1];
        for j in 1..=i {
            next_f[j] = j as f64 * float_row.get(j).copied().unwrap_or(0.0) + float_row[j - 1];
        }
        exact_row = exact_row.and_then(|row| {
            let mut next = vec![0u128; i + 1];
            for j in 1..=i {
                let prev = row.get(j).copied().unwrap_or(0);
                next[j] = (j as u128).checked_mul(prev)?.checked_add(row[j - 1])?;
            }
            Some(next)
        });
        float_row = next_f;
    }
    Ok((exact_row, float_row))
}

fn stirling2(n: u64, k: u64) -> R {
    let (exact_row, float_row) = stirling_row(n)?;
    let k = k as usize;
    Ok(match exact_row {
        Some(row) => exact(row.get(k).copied().unwrap_or(0)),
        None => Value::Num(float_row.get(k).copied().unwrap_or(0.0)),
    })
}

fn bell(n: u64) -> R {
    let (exact_row, float_row) = stirling_row(n)?;
    let exact_sum = exact_row.and_then(|row| row.into_iter().try_fold(0u128, u128::checked_add));
    Ok(match exact_sum {
        Some(t) => exact(t),
        None => Value::Num(float_row.iter().sum()),
    })
}

/// Deterministic Miller-Rabin for 64-bit integers.
pub fn is_prime_u64(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    const SMALL: [u64; 12] = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];
    for p in SMALL {
        if n % p == 0 {
            return n == p;
        }
    }
    let mul = |a: u64, b: u64| ((a as u128 * b as u128) % n as u128) as u64;
    let pow = |mut b: u64, mut e: u64| {
        let mut r = 1u64;
        b %= n;
        while e > 0 {
            if e & 1 == 1 {
                r = mul(r, b);
            }
            b = mul(b, b);
            e >>= 1;
        }
        r
    };
    let mut d = n - 1;
    let mut s = 0;
    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }
    'witness: for a in SMALL {
        let mut x = pow(a, d);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 1..s {
            x = mul(x, x);
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false;
    }
    true
}

fn gcd_i(a: i128, b: i128) -> i128 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Extended Euclid: (g, x, y) with a x + b y = g ≥ 0.
fn xgcd_i(a: i128, b: i128) -> (i128, i128, i128) {
    let (mut old_r, mut r) = (a, b);
    let (mut old_s, mut s) = (1i128, 0i128);
    let (mut old_t, mut t) = (0i128, 1i128);
    while r != 0 {
        let q = old_r / r;
        (old_r, r) = (r, old_r - q * r);
        (old_s, s) = (s, old_s - q * s);
        (old_t, t) = (t, old_t - q * t);
    }
    if old_r < 0 { (-old_r, -old_s, -old_t) } else { (old_r, old_s, old_t) }
}

fn integers(name: &str, args: &[Value]) -> Result<Vec<i128>, String> {
    let values = flatten(args);
    if values.is_empty() {
        return Err(format!("Too few arguments in function {name}"));
    }
    (0..values.len()).map(|i| int(name, &values, i).map(i128::from)).collect()
}

thread_local! {
    static RNG: Cell<u64> = Cell::new({
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
        (t ^ 0x9E37_79B9_7F4A_7C15) | 1
    });
}

/// Uniform in [0, 1), from xorshift64*.
fn random_unit() -> f64 {
    RNG.with(|s| {
        let mut x = s.get();
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        s.set(x);
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    })
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    Some(match name {
        "factorial" => each(name, args, &factorial),
        "combinations" => (|| {
            arity(name, args, 2, 2)?;
            let (n, k) = (count_arg(name, args, 0)?, count_arg(name, args, 1)?);
            if k > n {
                return Err("combinations: k must not be larger than n".into());
            }
            Ok(binomial(n, k))
        })(),
        "permutations" => (|| {
            arity(name, args, 1, 2)?;
            let n = count_arg(name, args, 0)?;
            let k = if args.len() == 2 { count_arg(name, args, 1)? } else { n };
            if k > n {
                return Err("permutations: k must not be larger than n".into());
            }
            let exact_value = (0..k).try_fold(1u128, |acc, i| acc.checked_mul((n - i) as u128));
            Ok(match exact_value {
                Some(r) => exact(r),
                None => Value::Num((0..k).fold(1.0, |acc, i| acc * (n - i) as f64)),
            })
        })(),
        "multinomial" => (|| {
            let parts = integers(name, args)?;
            if parts.iter().any(|p| *p < 0) {
                return Err("multinomial: values must not be negative".into());
            }
            let mut total = 0u64;
            let mut exact_value: Option<u128> = Some(1);
            let mut float_value = 1.0;
            for p in parts {
                total += p as u64;
                exact_value = exact_value.and_then(|e| e.checked_mul(choose(total, p as u64)?));
                float_value *= choose_f64(total, p as u64);
            }
            Ok(exact_value.map_or(Value::Num(float_value), exact))
        })(),
        "catalan" => (|| {
            arity(name, args, 1, 1)?;
            let n = count_arg(name, args, 0)?;
            Ok(match choose(2 * n, n) {
                Some(c) => exact(c / (n as u128 + 1)),
                None => Value::Num(choose_f64(2 * n, n) / (n as f64 + 1.0)),
            })
        })(),
        "stirlingS2" => (|| {
            arity(name, args, 2, 2)?;
            stirling2(count_arg(name, args, 0)?, count_arg(name, args, 1)?)
        })(),
        "bellNumbers" => (|| {
            arity(name, args, 1, 1)?;
            bell(count_arg(name, args, 0)?)
        })(),
        "composition" => (|| {
            arity(name, args, 2, 2)?;
            let (n, k) = (count_arg(name, args, 0)?, count_arg(name, args, 1)?);
            if k == 0 || k > n {
                return Err("composition: need 1 ≤ k ≤ n".into());
            }
            Ok(binomial(n - 1, k - 1))
        })(),
        "isPrime" => each(name, args, &|v| {
            let x = v.as_f64().ok_or("isPrime expects a number")?;
            if x.abs() > 9_007_199_254_740_992.0 {
                return Err("isPrime: numbers above 2^53 cannot be represented exactly".into());
            }
            Ok(Value::Bool(x.fract() == 0.0 && x >= 2.0 && is_prime_u64(x as u64)))
        }),
        "gcd" => integers(name, args).map(|v| Value::Num(v.into_iter().fold(0, gcd_i) as f64)),
        "lcm" => (|| {
            let v = integers(name, args)?;
            let mut acc: i128 = 1;
            for x in v {
                if x == 0 {
                    return num(0.0);
                }
                acc = (acc / gcd_i(acc, x)).checked_mul(x.abs()).ok_or("lcm: result too large")?;
            }
            num(acc as f64)
        })(),
        "xgcd" => (|| {
            arity(name, args, 2, 2)?;
            let (g, x, y) = xgcd_i(int(name, args, 0)? as i128, int(name, args, 1)? as i128);
            Ok(super::nums([g as f64, x as f64, y as f64]))
        })(),
        "invmod" => (|| {
            arity(name, args, 2, 2)?;
            let (a, m) = (int(name, args, 0)? as i128, int(name, args, 1)? as i128);
            if m <= 0 {
                return Err("invmod: the modulus must be positive".into());
            }
            let (g, x, _) = xgcd_i(a.rem_euclid(m), m);
            if g != 1 {
                return Err(format!("invmod: {a} has no inverse modulo {m}"));
            }
            num(x.rem_euclid(m) as f64)
        })(),
        "random" => (|| {
            arity(name, args, 0, 2)?;
            let (lo, hi) = match args.len() {
                0 => (0.0, 1.0),
                1 => (0.0, real(name, args, 0)?),
                _ => (real(name, args, 0)?, real(name, args, 1)?),
            };
            num(lo + random_unit() * (hi - lo))
        })(),
        "randomInt" => (|| {
            arity(name, args, 1, 2)?;
            let (lo, hi) = if args.len() == 1 { (0, int(name, args, 0)?) } else { (int(name, args, 0)?, int(name, args, 1)?) };
            if hi <= lo {
                return Err("randomInt: max must be greater than min".into());
            }
            num((lo as f64 + (random_unit() * (hi - lo) as f64).floor()).min(hi as f64 - 1.0))
        })(),
        "pickRandom" => (|| {
            arity(name, args, 1, 1)?;
            let items = flatten(args);
            if items.is_empty() {
                return Err("pickRandom: the list is empty".into());
            }
            let i = ((random_unit() * items.len() as f64) as usize).min(items.len() - 1);
            Ok(items[i].clone())
        })(),
        _ => return None,
    })
}
