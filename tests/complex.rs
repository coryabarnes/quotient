//! Complex numbers. Branch cuts for real inputs follow mathjs (complex.js), which
//! the original app used.

mod common;
use common::{calc, error, full};

#[test]
fn arithmetic() {
    assert_eq!(calc("(2 + 3i) * (1 - i)"), "5 + i");
    assert_eq!(calc("(1 + 2i) / (3 - 4i)"), "-0.2 + 0.4i");
    assert_eq!(calc("i^2"), "-1");
    assert_eq!(calc("(1 + i)^2"), "2i");
    assert_eq!(calc("abs(3 + 4i)"), "5");
    assert_eq!(calc("conj(2 + 3i)"), "2 - 3i");
    assert_eq!(calc("re(2 + 3i)"), "2");
    assert_eq!(calc("im(2 + 3i)"), "3");
    assert_eq!(calc("arg(1 + i)"), calc("pi / 4"));
    assert_eq!(calc("complex(1, -1)"), "1 - i");
    assert_eq!(calc("2 + 3i == 2 + 3i"), "true");
}

#[test]
fn real_functions_extend_to_complex() {
    assert_eq!(calc("sqrt(-16)"), "4i");
    assert_eq!(calc("sqrt(-1) * sqrt(-1)"), "-1");
    assert_eq!(calc("log(-1)"), calc("pi") + "i");
    assert_eq!(calc("(-8)^(1/3)"), "1 + 1.7320508075689i");
    assert_eq!(calc("asin(2)"), "1.5707963267949 - 1.3169578969248i");
    assert_eq!(calc("acos(2)"), "1.3169578969248i");
    assert_eq!(calc("acosh(0.5)"), "1.0471975511966i");
    assert_eq!(calc("atanh(2)"), "0.54930614433405 - 1.5707963267949i");
    assert_eq!(calc("exp(i * pi)"), "-1");
    assert_eq!(calc("exp(i * pi / 2)"), "i");
    assert_eq!(calc("(1 + i)^0.5"), "1.0986841134678 + 0.45508986056223i");
    assert_eq!(calc("nthRoots(16, 4)"), "[2, 2i, -2, -2i]");
    assert_eq!(calc("nthRoots(-1, 2)"), "[i, -i]");
}

#[test]
fn display() {
    assert_eq!(calc("-i"), "-i");
    assert_eq!(calc("3 - 2i"), "3 - 2i");
    assert_eq!(calc("0.5i"), "0.5i");
    // Full precision shows the floating-point residue that the default hides.
    assert_eq!(full("exp(i * pi)"), "-1 + 1.2246467991473532e-16i");
}

#[test]
fn unsupported_operations_are_errors() {
    assert_eq!(error("2 + 3i < 5"), "Complex numbers cannot be compared with < or >");
    assert_eq!(error("(1 + i) mod 2"), "mod is not defined for complex numbers");
    assert_eq!(error("(1 + i) * 5 m"), "Complex numbers with units are not supported");
    assert_eq!(error("i = 2"), "Cannot assign to 'i': it is a protected function or constant name");
}
