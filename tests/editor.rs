//! Syntax highlighting and autocomplete.

use quotient::highlight::{Class, complete, highlight, word_before};

fn classes(text: &str, vars: &[&str]) -> Vec<(&'static str, Class)> {
    let vars: Vec<String> = vars.iter().map(|s| s.to_string()).collect();
    let spans = highlight(text, &vars);
    // Spans must cover the text exactly, in order.
    let mut pos = 0;
    for (r, _) in &spans {
        assert_eq!(r.start, pos, "gap or overlap in {text:?}");
        pos = r.end;
    }
    assert_eq!(pos, text.len());
    spans
        .into_iter()
        .filter(|(r, c)| *c != Class::Plain && !text[r.clone()].trim().is_empty())
        .map(|(r, c)| (Box::leak(text[r].to_string().into_boxed_str()) as &str, c))
        .collect()
}

#[test]
fn highlights_each_kind_of_token() {
    assert_eq!(
        classes("x = sqrt(2) * 5 km to mi // note", &["x"]),
        vec![
            ("x", Class::Variable),
            ("=", Class::Operator),
            ("sqrt", Class::Function),
            ("(", Class::Operator),
            ("2", Class::Number),
            (")", Class::Operator),
            ("*", Class::Operator),
            ("5", Class::Number),
            ("km", Class::Unit),
            ("to", Class::Keyword),
            ("mi", Class::Unit),
            ("// note", Class::Comment),
        ]
    );
}

#[test]
fn numbers_constants_and_text() {
    assert_eq!(classes("2e", &[]), vec![("2", Class::Number), ("e", Class::Constant)]);
    assert_eq!(classes("1.5e-3", &[]), vec![("1.5e-3", Class::Number)]);
    assert_eq!(classes("0xff", &[]), vec![("0xff", Class::Number)]);
    assert_eq!(classes("5km", &[]), vec![("5", Class::Number), ("km", Class::Unit)]);
    assert_eq!(classes("\"a // b\"", &[]), vec![("\"a // b\"", Class::Text)]);
    assert_eq!(classes("ans + line3", &[]), vec![("ans", Class::Variable), ("+", Class::Operator), ("line3", Class::Variable)]);
    assert_eq!(classes("pi", &[]), vec![("pi", Class::Constant)]);
    assert_eq!(classes("255 to hex", &[]), vec![("255", Class::Number), ("to", Class::Keyword), ("hex", Class::Keyword)]);
}

#[test]
fn handles_unicode_and_odd_input() {
    for text in ["π × 2", "72 °F", "√16 ÷ 2", "", "\n\n", "\"unterminated", "((((", "µs"] {
        classes(text, &[]);
    }
}

#[test]
fn word_before_cursor() {
    assert_eq!(word_before("x = sq", 6), (4, "sq"));
    assert_eq!(word_before("sqrt(", 5), (5, ""));
    assert_eq!(word_before("12", 2), (2, ""));
    assert_eq!(word_before("aπb", "aπb".len()), (0, "aπb"));
}

#[test]
fn completions() {
    let vars = vec!["sales".to_string(), "salary".to_string()];
    let c = complete("sa", &vars, 10);
    assert_eq!(c[0].label, "sales");
    assert_eq!(c[1].label, "salary");
    let sq = complete("sq", &[], 10);
    assert_eq!(sq[0].label, "sqrt");
    assert_eq!(sq[0].insert, "sqrt(");
    assert!(sq[0].detail.contains("Square root"));
    // Case-insensitive matches work, after exact-case ones; shorter names first.
    let is: Vec<String> = complete("isp", &[], 10).into_iter().map(|c| c.label).collect();
    assert_eq!(is, ["isPrime", "isPositive"]);
    let ta: Vec<String> = complete("ta", &["Tab".to_string()], 10).into_iter().map(|c| c.label).collect();
    assert_eq!(ta, ["tan", "tau", "tanh", "Tab"]);
    assert!(complete("", &[], 10).is_empty());
    assert!(complete("sqrt", &[], 10).iter().all(|c| c.label != "sqrt"));
    assert_eq!(complete("s", &[], 3).len(), 3);
}
