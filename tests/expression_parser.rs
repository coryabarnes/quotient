//! Ported from the original app's `expression-parser.test.js`.

use quotient::document::{Expression, parse_expressions};

fn exprs(items: &[(&str, usize, usize)]) -> Vec<Expression> {
    items
        .iter()
        .map(|(e, s, end)| Expression { expression: e.to_string(), start_line: *s, end_line: *end })
        .collect()
}

#[test]
fn single_line_expressions() {
    assert_eq!(parse_expressions("2 + 3"), exprs(&[("2 + 3", 0, 0)]));
    assert_eq!(
        parse_expressions("x = 5\ny = 10\nx + y"),
        exprs(&[("x = 5", 0, 0), ("y = 10", 1, 1), ("x + y", 2, 2)])
    );
    assert_eq!(parse_expressions("  x = 5  \n  y = 10  "), exprs(&[("x = 5", 0, 0), ("y = 10", 1, 1)]));
}

#[test]
fn empty_lines() {
    assert_eq!(
        parse_expressions("x = 5\n\ny = 10"),
        exprs(&[("x = 5", 0, 0), ("", 1, 1), ("y = 10", 2, 2)])
    );
    assert_eq!(
        parse_expressions("x = 5\n\n\ny = 10"),
        exprs(&[("x = 5", 0, 0), ("", 1, 1), ("", 2, 2), ("y = 10", 3, 3)])
    );
    assert_eq!(parse_expressions("   \n   "), exprs(&[("", 0, 0), ("", 1, 1)]));
    assert_eq!(parse_expressions("\n\n\n"), exprs(&[("", 0, 0), ("", 1, 1), ("", 2, 2), ("", 3, 3)]));
}

#[test]
fn comments() {
    assert_eq!(parse_expressions("// This is a comment"), exprs(&[("// This is a comment", 0, 0)]));
    assert_eq!(
        parse_expressions("// Comment\nx = 5\n// Another comment\ny = 10"),
        exprs(&[("// Comment", 0, 0), ("x = 5", 1, 1), ("// Another comment", 2, 2), ("y = 10", 3, 3)])
    );
}

#[test]
fn inline_comments_are_stripped() {
    // Changed from the original, which passed "x = 5 // note" to mathjs and failed.
    assert_eq!(parse_expressions("x = 5 // note"), exprs(&[("x = 5", 0, 0)]));
    assert_eq!(parse_expressions("\"a // b\" // c"), exprs(&[("\"a // b\"", 0, 0)]));
    // The original kept the comment text inside the joined expression.
    assert_eq!(
        parse_expressions("func(\n  // comment inside\n  1, 2\n)"),
        exprs(&[("func(  1, 2 )", 0, 3)])
    );
}

#[test]
fn multi_line_expressions() {
    assert_eq!(parse_expressions("trend(\n  1, 2, 3\n)"), exprs(&[("trend( 1, 2, 3 )", 0, 2)]));
    assert_eq!(
        parse_expressions("func(\n  inner(\n    1, 2\n  ),\n  3\n)"),
        exprs(&[("func( inner( 1, 2 ), 3 )", 0, 5)])
    );
    assert_eq!(
        parse_expressions("a = trend(\n  1, 2\n)\nb = func(\n  3, 4\n)"),
        exprs(&[("a = trend( 1, 2 )", 0, 2), ("b = func( 3, 4 )", 3, 5)])
    );
    assert_eq!(parse_expressions("func(\n  1,\n\n  2\n)"), exprs(&[("func( 1,  2 )", 0, 4)]));
    assert_eq!(parse_expressions("[1,\n 2]"), exprs(&[("[1, 2]", 0, 1)]));
}

#[test]
fn unbalanced_brackets() {
    // The original swallowed every following line into an unclosed bracket; now
    // only the line with the unclosed bracket fails and later lines still work.
    assert_eq!(parse_expressions("func(\n  1, 2"), exprs(&[("func(", 0, 0), ("1, 2", 1, 1)]));
    assert_eq!(parse_expressions("(1 + 2\nx = 5\n\ny"), exprs(&[("(1 + 2", 0, 0), ("x = 5", 1, 1), ("", 2, 2), ("y", 3, 3)]));
    assert_eq!(parse_expressions("1 + 2)"), exprs(&[("1 + 2)", 0, 0)]));
    // The original swallowed every following line after an extra `)`.
    assert_eq!(parse_expressions("1 + 2)\nx = 5"), exprs(&[("1 + 2)", 0, 0), ("x = 5", 1, 1)]));
}

#[test]
fn stock_trend_example() {
    let input = "stock_trend = trend(\n  100, 105, 98,\n  110, 115, 108,\n  112, 120\n)";
    assert_eq!(
        parse_expressions(input),
        exprs(&[("stock_trend = trend( 100, 105, 98, 110, 115, 108, 112, 120 )", 0, 4)])
    );
}

#[test]
fn mixed_content() {
    let input = "// Welcome to calctxt\nx = 5\ny = 10\n\n// Calculate sum\nz = x + y\n\nresult = trend(\n  x, y, z\n)";
    assert_eq!(
        parse_expressions(input),
        exprs(&[
            ("// Welcome to calctxt", 0, 0),
            ("x = 5", 1, 1),
            ("y = 10", 2, 2),
            ("", 3, 3),
            ("// Calculate sum", 4, 4),
            ("z = x + y", 5, 5),
            ("", 6, 6),
            ("result = trend( x, y, z )", 7, 9),
        ])
    );
}
