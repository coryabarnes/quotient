//! Financial functions. `pmt`, `fv`, `pv`, `nper`, `npv` and `irr` follow
//! Excel's conventions: rates are per period (5%/12 for monthly), money paid
//! out is negative, and `type` is 1 for payments at the start of each period.

use super::{R, flat_reals, num, reals};
use crate::value::Value;

/// (1 + r)^n, and the annuity factor ((1 + r)^n - 1) / r with its r = 0 limit.
fn growth(rate: f64, n: f64) -> (f64, f64) {
    let g = (1.0 + rate).powf(n);
    let annuity = if rate == 0.0 { n } else { (g - 1.0) / rate };
    (g, annuity)
}

fn npv(rate: f64, flows: &[f64], first_period: i32) -> f64 {
    flows.iter().enumerate().map(|(i, cf)| cf / (1.0 + rate).powi(i as i32 + first_period)).sum()
}

fn irr(flows: &[f64]) -> R {
    if !(flows.iter().any(|c| *c > 0.0) && flows.iter().any(|c| *c < 0.0)) {
        return Err("irr needs at least one positive and one negative cash flow".into());
    }
    let f = |r: f64| npv(r, flows, 0);
    let df = |r: f64| flows.iter().enumerate().map(|(i, cf)| -(i as f64) * cf / (1.0 + r).powi(i as i32 + 1)).sum::<f64>();
    // Newton's method from 10%, like Excel's default guess.
    let mut r = 0.1;
    for _ in 0..100 {
        let (y, dy) = (f(r), df(r));
        if dy == 0.0 || !dy.is_finite() {
            break;
        }
        let next = r - y / dy;
        if next <= -1.0 || !next.is_finite() {
            break;
        }
        if (next - r).abs() < 1e-14 * next.abs().max(1.0) {
            return num(next);
        }
        r = next;
    }
    // Fall back to bisection over a bracket where the NPV changes sign.
    let mut lo = -0.999_999;
    let mut hi = lo;
    let mut step = 0.01;
    while hi < 1e6 {
        let next = hi + step;
        if f(hi).signum() != f(next).signum() {
            lo = hi;
            hi = next;
            for _ in 0..200 {
                let mid = (lo + hi) / 2.0;
                if f(lo).signum() == f(mid).signum() {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            return num((lo + hi) / 2.0);
        }
        hi = next;
        step *= 1.2;
    }
    Err("irr: no rate found".into())
}

pub fn call(name: &str, args: &[Value]) -> Option<R> {
    Some(match name {
        "pctchange" => reals(name, args, 2, 2).and_then(|a| {
            if a[0] == 0.0 {
                return Err("Cannot calculate percentage change from zero".into());
            }
            num((a[1] - a[0]) / a[0] * 100.0)
        }),
        "cagr" => reals(name, args, 3, 3).and_then(|a| {
            if a.iter().any(|x| *x <= 0.0) {
                return Err("CAGR requires positive values".into());
            }
            num(((a[1] / a[0]).powf(1.0 / a[2]) - 1.0) * 100.0)
        }),
        "compound" => reals(name, args, 3, 4).and_then(|a| {
            let freq = a.get(3).copied().unwrap_or(1.0);
            if freq <= 0.0 {
                return Err("compound: compounding frequency must be positive".into());
            }
            num(a[0] * (1.0 + (a[1] / 100.0) / freq).powf(freq * a[2]))
        }),
        "pmt" => reals(name, args, 3, 5).and_then(|a| {
            let (rate, n, pv) = (a[0], a[1], a[2]);
            let (fv, kind) = (a.get(3).copied().unwrap_or(0.0), a.get(4).copied().unwrap_or(0.0));
            if n == 0.0 {
                return Err("pmt: the number of periods cannot be zero".into());
            }
            let (g, annuity) = growth(rate, n);
            num(-(pv * g + fv) / ((1.0 + rate * kind) * annuity))
        }),
        "fv" => reals(name, args, 3, 5).and_then(|a| {
            let (rate, n, pmt) = (a[0], a[1], a[2]);
            let (pv, kind) = (a.get(3).copied().unwrap_or(0.0), a.get(4).copied().unwrap_or(0.0));
            let (g, annuity) = growth(rate, n);
            num(-(pv * g + pmt * (1.0 + rate * kind) * annuity))
        }),
        "pv" => reals(name, args, 3, 5).and_then(|a| {
            let (rate, n, pmt) = (a[0], a[1], a[2]);
            let (fv, kind) = (a.get(3).copied().unwrap_or(0.0), a.get(4).copied().unwrap_or(0.0));
            let (g, annuity) = growth(rate, n);
            num(-(fv + pmt * (1.0 + rate * kind) * annuity) / g)
        }),
        "nper" => reals(name, args, 3, 5).and_then(|a| {
            let (rate, pmt, pv) = (a[0], a[1], a[2]);
            let (fv, kind) = (a.get(3).copied().unwrap_or(0.0), a.get(4).copied().unwrap_or(0.0));
            if rate == 0.0 {
                if pmt == 0.0 {
                    return Err("nper: payment cannot be zero when the rate is zero".into());
                }
                return num(-(pv + fv) / pmt);
            }
            let adjusted = pmt * (1.0 + rate * kind);
            let n = ((adjusted - fv * rate) / (adjusted + pv * rate)).ln() / (1.0 + rate).ln();
            if !n.is_finite() {
                return Err("nper: these payments never reach the target".into());
            }
            num(n)
        }),
        "npv" => (|| {
            let rate = super::real(name, args, 0)?;
            let flows = flat_reals(name, &args[1..])?;
            num(npv(rate, &flows, 1))
        })(),
        "irr" => flat_reals(name, args).and_then(|f| irr(&f)),
        _ => return None,
    })
}
