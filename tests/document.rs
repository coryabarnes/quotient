//! Whole documents: grouping, output text, settings and the welcome example.

use quotient::{FormatOptions, Kind, Settings, WELCOME, calculate};

#[test]
fn output_lines() {
    let doc = calculate("// quick math\nprice = 149.99\ntax = 8% of price\n\nprice + tax\nbogus\ntrend(1, 3)", &Settings::default());
    let lines: Vec<(Kind, &str)> = doc.outputs.iter().map(|o| (o.kind, o.text.as_str())).collect();
    assert_eq!(
        lines,
        vec![
            (Kind::Info, "// quick math"),
            (Kind::Assignment, "price = 149.99"),
            (Kind::Assignment, "tax = 11.9992"),
            (Kind::Info, ""),
            (Kind::Result, "price + tax = 161.9892"),
            (Kind::Error, "bogus → Error: Undefined symbol bogus"),
            (Kind::Result, "trend(1, 3) = Values: [1, 3]\n  1 → 3 (+2)\n  Average change: 2.000\n  Max change: 2\n  Min change: 2"),
        ]
    );
    assert_eq!(doc.outputs[1].value.as_deref(), Some("149.99"));
    assert_eq!(doc.outputs[5].value.as_deref(), Some("Undefined symbol bogus"));
    assert_eq!(doc.variables, vec!["price", "tax"]);
}

#[test]
fn line_numbers_follow_the_text() {
    let doc = calculate("\n\ny\ny = 2\nsum(\n 1,\n 2\n)", &Settings::default());
    let spans: Vec<(usize, usize)> = doc.outputs.iter().map(|o| (o.start_line, o.end_line)).collect();
    assert_eq!(spans, vec![(2, 2), (3, 3), (4, 7)]);
    assert_eq!(doc.outputs[0].kind, Kind::Error, "variables are not visible before they are defined");
}

#[test]
fn settings_change_the_display() {
    let digits = |d| Settings { format: FormatOptions { digits: d, thousands: false }, precision: None };
    let one = |doc: &str, s: Settings| calculate(doc, &s).outputs[0].value.clone().unwrap();
    assert_eq!(one("2/3", digits(Some(4))), "0.6667");
    assert_eq!(one("2/3", digits(None)), "0.6666666666666666");
    let grouped = Settings { format: FormatOptions { digits: Some(14), thousands: true }, precision: None };
    assert_eq!(one("1234567.5", grouped), "1,234,567.5");
    assert_eq!(one("2^40 m", grouped), "1,099,511,627,776 m");
    let precise = Settings { format: FormatOptions::default(), precision: Some(20) };
    assert_eq!(one("1/3", precise), "0.33333333333333333333");
}

#[test]
fn empty_document() {
    let doc = calculate("  \n ", &Settings::default());
    assert_eq!(doc.outputs.len(), 1);
    assert_eq!(doc.outputs[0].kind, Kind::Info);
}

#[test]
fn welcome_document_has_no_errors() {
    for o in calculate(WELCOME, &Settings::default()).outputs {
        assert_ne!(o.kind, Kind::Error, "{}", o.text);
    }
}

#[test]
fn a_huge_document_is_fast() {
    let doc: String = (0..2000).map(|i| format!("x{i} = {i} * 2 km\nsqrt(x{i} * 3 m) to m\n")).collect();
    let start = std::time::Instant::now();
    let out = calculate(&doc, &Settings::default());
    assert_eq!(out.outputs.len(), 4000);
    assert!(start.elapsed().as_secs_f64() < 2.0, "took {:?}", start.elapsed());
}
