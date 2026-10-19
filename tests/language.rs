//! The expression language: operators, literals, variables and errors.

mod common;
use common::{calc, error, full};

#[test]
fn precedence() {
    assert_eq!(calc("2 + 3 * 4"), "14");
    assert_eq!(calc("(2 + 3) * 4"), "20");
    assert_eq!(calc("2 ^ 3 ^ 2"), "512");
    assert_eq!(calc("-2 ^ 2"), "-4");
    assert_eq!(calc("2 ^ -2"), "0.25");
    assert_eq!(calc("2 ^ 3!"), "64");
    assert_eq!(calc("3!^2"), "36");
    assert_eq!(calc("-3!"), "-6");
    assert_eq!(calc("10 - 2 - 3"), "5");
    assert_eq!(calc("100 / 10 / 5"), "2");
}

#[test]
fn implicit_multiplication() {
    assert_eq!(calc("2pi"), calc("2 * pi"));
    assert_eq!(calc("r = 3\n2 pi r"), calc("2 * pi * 3"));
    assert_eq!(calc("3(1 + 1)"), "6");
    assert_eq!(calc("(1 + 1)(2 + 2)"), "8");
    assert_eq!(calc("2e"), calc("2 * e"));
    assert_eq!(calc("2√9"), "6");
}

#[test]
fn literals() {
    assert_eq!(calc("1e3"), "1000");
    assert_eq!(calc("1.5e-3"), "0.0015");
    assert_eq!(calc(".5"), "0.5");
    assert_eq!(calc("0xff + 0b101 + 0o17"), "275");
    assert_eq!(calc("1_000_000"), "1000000");
    assert_eq!(calc("\"text\""), "text");
}

#[test]
fn typographic_symbols() {
    assert_eq!(calc("3 × 4 ÷ 2"), "6");
    assert_eq!(calc("5 − 2"), "3");
    assert_eq!(calc("π"), calc("pi"));
    assert_eq!(calc("2² + 2³"), "12");
    assert_eq!(calc("√16"), "4");
    assert_eq!(calc("5 ≠ 4"), "true");
    assert_eq!(calc("4 ≥ 4"), "true");
}

#[test]
fn percentages() {
    assert_eq!(calc("50%"), "0.5");
    assert_eq!(calc("20% of 150"), "30");
    assert_eq!(calc("15% off 80"), "68");
    assert_eq!(calc("5% on 200"), "210");
    assert_eq!(calc("100 + 10%"), "110");
    assert_eq!(calc("100 - 10%"), "90");
    assert_eq!(calc("200 * 10%"), "20");
    assert_eq!(calc("7 % 3"), "1");
    assert!(error("0.2 of 150").contains("percentage is expected"));
}

#[test]
fn modulo_follows_the_divisor_sign() {
    assert_eq!(calc("-7 mod 3"), "2");
    assert_eq!(calc("7 mod -3"), "-2");
    assert_eq!(calc("mod(-7, 3)"), "2");
    assert_eq!(calc("5 mod 0"), "5");
}

#[test]
fn comparisons_and_logic() {
    assert_eq!(calc("0.1 + 0.2 == 0.3"), "true");
    assert_eq!(calc("1 == 1.0000000000001"), "true");
    assert_eq!(calc("1 == 1.000000000001"), "false");
    assert_eq!(calc("1 < 2 and 2 < 3"), "true");
    assert_eq!(calc("true xor true"), "false");
    assert_eq!(calc("not true or true"), "true");
    assert_eq!(calc("and(true, false)"), "false");
    assert_eq!(calc("x = 150\nx > 100 ? \"big\" : \"small\""), "big");
    assert_eq!(calc("1 ? 2 : 3 ? 4 : 5"), "2");
}

#[test]
fn ranges_and_indexing() {
    assert_eq!(calc("1:5"), "[1, 2, 3, 4, 5]");
    assert_eq!(calc("0:0.5:2"), "[0, 0.5, 1, 1.5, 2]");
    assert_eq!(calc("10:-3:1"), "[10, 7, 4, 1]");
    assert_eq!(calc("5:1"), "[]");
    assert_eq!(calc("sum(1:100)"), "5050");
    assert_eq!(calc("[10, 20, 30][2]"), "20");
    assert_eq!(calc("[1, 2; 3, 4][2, 1]"), "3");
    assert_eq!(calc("\"hello\"[2]"), "e");
    assert_eq!(error("[1, 2][3]"), "Index out of range (3 > 2)");
    assert!(error("1:2000000").contains("more than"));
}

#[test]
fn matrices_and_broadcasting() {
    assert_eq!(calc("[1, 2, 3] * 2"), "[2, 4, 6]");
    assert_eq!(calc("[1, 2] + [3, 4]"), "[4, 6]");
    assert_eq!(calc("[1, 2, 3] * [4, 5, 6]"), "32");
    assert_eq!(calc("[1, 2; 3, 4] * [5, 6; 7, 8]"), "[[19, 22], [43, 50]]");
    assert_eq!(calc("sqrt([4, 9])"), "[2, 3]");
    assert_eq!(error("[1, 2] + [1, 2, 3]"), "Dimension mismatch (2 != 3)");
}

#[test]
fn variables_and_functions() {
    assert_eq!(calc("x = 4\ny = x^2\ny + 1"), "17");
    assert_eq!(calc("f(x) = x^2 + 1\nf(3)"), "10");
    assert_eq!(calc("f(x, y) = x * y + 1\nf(2, 3)"), "7");
    assert_eq!(calc("g(n) = n <= 1 ? 1 : n * g(n - 1)\ng(10)"), "3628800");
    assert_eq!(calc("h() = 42\nh()"), "42");
    assert_eq!(calc("k = sqrt\nk(16)"), "4");
    assert!(error("g(n) = g(n)\ng(1)").contains("Maximum call depth"));
    assert!(error("f(x) = x\nf(1, 2)").contains("Wrong number of arguments"));
}

#[test]
fn ans_and_line_references() {
    assert_eq!(calc("2 + 3\nans * 10"), "50");
    assert_eq!(calc("7\n\nline1 * 2"), "14");
    assert_eq!(calc("x = 4\nline1 + 1"), "5");
    // A multi-line expression is referenced by its first line.
    assert_eq!(calc("sum(\n1, 2\n)\nline1"), "3");
}

#[test]
fn comments() {
    assert_eq!(calc("x = 5 // the value\nx"), "5");
    assert_eq!(calc("\"a // b\""), "a // b");
}

#[test]
fn errors_are_clear() {
    assert_eq!(error("foo + 1"), "Undefined symbol foo");
    assert_eq!(error("foo(1)"), "Undefined function foo");
    assert_eq!(error("1 +"), "Unexpected end of expression (char 4)");
    assert_eq!(error("(1 + 2"), "Parenthesis ) expected (char 7)");
    assert_eq!(error("1 + 2)"), "Unexpected operator ) (char 6)");
    assert_eq!(error("1 +* 2"), "Value expected, got operator * (char 4)");
    assert_eq!(error("x = 4\nx(2)"), "x is a variable, not a function");
    assert_eq!(error("pi = 3"), "Cannot assign to 'pi': it is a protected function or constant name");
    assert_eq!(error("sum = 3"), "Cannot assign to 'sum': it is a protected function or constant name");
    assert_eq!(error("to = 3"), "\"to\" is a reserved word and cannot be assigned");
    assert_eq!(error("\"abc"), "Missing closing quote (char 1)");
    assert_eq!(error("5 $ 3"), "Unexpected character \"$\" (char 3)");
    assert!(error(&"(".repeat(10_000)).contains("nested too deeply"));
}

#[test]
fn special_values() {
    assert_eq!(calc("1 / 0"), "Infinity");
    assert_eq!(calc("-1 / 0"), "-Infinity");
    assert_eq!(calc("0 / 0"), "NaN");
}

#[test]
fn display_formats() {
    assert_eq!(calc("255 to hex"), "0xff");
    assert_eq!(calc("5 to bin"), "0b101");
    assert_eq!(calc("8 to oct"), "0o10");
    assert_eq!(calc("0.75 to fraction"), "3/4");
    assert_eq!(calc("0.1 + 0.2 to fraction"), "3/10");
    assert_eq!(calc("-1.5 to fraction"), "-3/2");
    assert_eq!(calc("3.5 kg to fraction"), "7/2 kg");
    assert_eq!(calc("123456789 to sci"), "1.23456789 × 10^8");
    assert_eq!(calc("0.00042 to sci"), "4.2 × 10^-4");
    assert_eq!(calc("0.256 to percent"), "25.6%");
    assert!(error("pi to fraction").contains("no simple fraction"));
    assert!(error("1.5 to hex").contains("whole numbers"));
}

#[test]
fn full_precision_matches_javascript() {
    assert_eq!(full("0.1 + 0.2"), "0.30000000000000004");
    assert_eq!(full("1e21"), "1e+21");
    assert_eq!(full("1.5e-7"), "1.5e-7");
    assert_eq!(full("0.000001"), "0.000001");
    assert_eq!(full("100 / 6"), "16.666666666666668");
    assert_eq!(full("-0"), "0");
}
