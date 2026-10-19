//! High-precision mode. Reference digits from Python's decimal module.

mod common;
use common::precise;

#[test]
fn exact_decimals() {
    assert_eq!(precise("0.1 + 0.2", 50), "0.3");
    assert_eq!(precise("0.1 + 0.2 == 0.3", 50), "true");
    assert_eq!(precise("0.1 * 3 - 0.3", 50), "0");
    assert_eq!(precise("1.1^10", 50), "2.5937424601");
    assert_eq!(precise("123456789 * 987654321", 50), "121932631112635269");
}

#[test]
fn many_digits() {
    assert_eq!(precise("1/3", 50), "0.33333333333333333333333333333333333333333333333333");
    assert_eq!(precise("sqrt(2)", 50), "1.4142135623730950488016887242096980785696718753769");
    assert_eq!(precise("pi", 50), "3.1415926535897932384626433832795028841971693993751");
    assert_eq!(precise("e", 50), "2.7182818284590452353602874713526624977572470937");
    assert_eq!(precise("ln(2)", 50), "0.69314718055994530941723212145817656807550013436026");
    assert_eq!(precise("sin(1)", 50), "0.84147098480789650665250232163029899962256306079837");
    assert_eq!(precise("atan(1) * 4", 50), precise("pi", 50));
    assert_eq!(precise("2^100", 40), "1267650600228229401496703205376");
    assert_eq!(precise("factorial(30)", 40), "265252859812191058636308480000000");
    assert_eq!(precise("combinations(100, 50)", 40), "100891344545564193334812497256");
}

#[test]
fn rounding_and_snapping() {
    assert_eq!(precise("round(2.675, 2)", 30), "2.68");
    assert_eq!(precise("round(-1.005, 2)", 30), "-1.01");
    assert_eq!(precise("floor(7 / 2)", 30), "3");
    assert_eq!(precise("sin(pi)", 30), "0");
    assert_eq!(precise("cos(pi / 2)", 30), "0");
    assert_eq!(precise("tan(pi / 4)", 30), "1");
    assert_eq!(precise("1/7", 10), "0.1428571429");
}

#[test]
fn other_types_still_work() {
    assert_eq!(precise("sqrt(-4)", 30), "2i");
    assert_eq!(precise("5 km to m", 30), "5000 m");
    assert_eq!(precise("mean([1, 2, 3, 4])", 30), "2.5");
    assert_eq!(precise("[1, 2] * 3", 30), "[3, 6]");
    assert_eq!(precise("sum(0.1, 0.2)", 30), "0.3");
}
