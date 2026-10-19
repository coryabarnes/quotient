#![allow(dead_code)]

use quotient::{FormatOptions, Kind, Settings, calculate};

/// The result of the last line of `doc` as displayed, or `Error: message`.
pub fn run(doc: &str, settings: Settings) -> String {
    let out = calculate(doc, &settings).outputs.pop().expect("an output");
    match out.kind {
        Kind::Error => format!("Error: {}", out.value.unwrap_or_default()),
        _ => out.value.unwrap_or(out.text),
    }
}

/// With the default display (14 significant digits).
pub fn calc(doc: &str) -> String {
    run(doc, Settings::default())
}

/// With every digit shown, as JavaScript would.
pub fn full(doc: &str) -> String {
    run(doc, Settings { format: FormatOptions::FULL, precision: None })
}

/// In high precision with `digits` significant digits.
pub fn precise(doc: &str, digits: usize) -> String {
    run(doc, Settings { format: FormatOptions::default(), precision: Some(digits) })
}

pub fn num(doc: &str) -> f64 {
    let s = full(doc);
    s.parse().unwrap_or_else(|_| panic!("{doc}: expected a number, got {s}"))
}

/// Asserts a numeric result within a relative tolerance of 1e-12.
pub fn close(doc: &str, expected: f64) {
    let got = num(doc);
    let ok = (got - expected).abs() <= 1e-12 * expected.abs().max(1e-300) || got == expected;
    assert!(ok, "{doc}: expected {expected}, got {got}");
}

pub fn error(doc: &str) -> String {
    let s = calc(doc);
    assert!(s.starts_with("Error: "), "{doc}: expected an error, got {s}");
    s["Error: ".len()..].to_string()
}
