//! Built-in functions. Reference values come from Python's math, cmath and
//! statistics modules, or are exact.

mod common;
use common::{calc, close, error, full};
use quotient::catalog::ENTRIES;
use quotient::functions::call;

#[test]
fn every_catalog_entry_is_implemented() {
    let mut apply = |_: &quotient::value::Value, _: Vec<quotient::value::Value>| -> Result<quotient::value::Value, String> {
        Err("not used".into())
    };
    for e in ENTRIES {
        assert!(call(e.name, vec![], &mut apply).is_some(), "{} is in the catalog but not implemented", e.name);
    }
    let mut names: Vec<&str> = ENTRIES.iter().map(|e| e.name).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), ENTRIES.len(), "duplicate catalog entries");
    assert!(ENTRIES.len() >= 190, "{} functions", ENTRIES.len());
}

#[test]
fn powers_and_logarithms() {
    assert_eq!(calc("sqrt(16)"), "4");
    assert_eq!(calc("cbrt(-27)"), "-3");
    assert_eq!(calc("nthRoot(-27, 3)"), "-3");
    assert_eq!(full("nthRoot(125, 3)"), "5");
    close("nthRoot(2, 5)", 2f64.powf(0.2));
    assert!(error("nthRoot(-16, 4)").contains("odd root"));
    assert_eq!(calc("log(100, 10)"), "2");
    assert_eq!(calc("ln(e)"), "1");
    assert_eq!(calc("log10(1000)"), "3");
    assert_eq!(calc("log2(1024)"), "10");
    close("log1p(1e-10)", 9.999999999500001e-11);
    close("expm1(1e-10)", 1.00000000005e-10);
    assert_eq!(calc("square(-3)"), "9");
    assert_eq!(calc("cube(2)"), "8");
    assert_eq!(calc("pow(2, 10)"), "1024");
    assert_eq!(calc("hypot(3, 4, 12)"), "13");
}

#[test]
fn rounding() {
    assert_eq!(calc("round(2.5)"), "3");
    assert_eq!(calc("round(-2.5)"), "-3");
    assert_eq!(calc("round(2.675, 2)"), "2.68");
    assert_eq!(calc("round(1.005, 2)"), "1.01");
    assert_eq!(calc("round(1234.5678, -2)"), "1200");
    assert_eq!(calc("floor(-2.5)"), "-3");
    assert_eq!(calc("ceil(-2.5)"), "-2");
    assert_eq!(calc("fix(-2.7)"), "-2");
    // Snaps when within rounding error, like mathjs: (0.1 + 0.7) * 10 is 7.999999999999999.
    assert_eq!(calc("floor((0.1 + 0.7) * 10)"), "8");
}

#[test]
fn trigonometry() {
    close("sin(0.3)", 0.29552020666133955);
    close("cos(-0.7)", 0.7648421872844885);
    close("tan(2.5)", -0.7470222972386602);
    close("sec(0.3)", 1.0 / 0.3f64.cos());
    close("csc(2.5)", 1.0 / 2.5f64.sin());
    close("cot(10)", 1.0 / 10f64.tan());
    close("asin(0.3)", 0.3046926540153975);
    close("acos(-0.7)", 2.3461938234056494);
    close("atan(10)", 1.4711276743037347);
    close("atan2(1, -1)", 2.356194490192345);
    close("asec(2)", 1.0471975511965979);
    close("acsc(-3)", -0.3398369094541219);
    close("acot(2)", 0.4636476090008061);
    close("sinh(2.5)", 6.0502044810397875);
    close("cosh(-0.7)", 1.255169005630943);
    close("tanh(0.3)", 0.2913126124515909);
    close("sech(2.5)", 1.0 / 2.5f64.cosh());
    close("csch(0.3)", 1.0 / 0.3f64.sinh());
    close("coth(-0.7)", 1.0 / (-0.7f64).tanh());
    close("asinh(10)", 2.99822295029797);
    close("acosh(2.5)", 1.566799236972411);
    close("atanh(-0.7)", -0.8673005276940531);
    close("asech(0.5)", 1.3169578969248166);
    close("acsch(2)", 0.48121182505960347);
    close("acoth(-3)", -0.34657359027997264);
}

#[test]
fn exact_zeros_from_rounded_pi() {
    assert_eq!(full("sin(pi)"), "0");
    assert_eq!(full("cos(pi / 2)"), "0");
    assert_eq!(full("tan(pi)"), "0");
    assert_eq!(full("cos(90 deg)"), "0");
    assert_eq!(full("csc(pi)"), "Infinity");
    // Genuinely tiny results are kept.
    assert_eq!(full("sin(1e-20)"), "1e-20");
}

#[test]
fn angles() {
    assert_eq!(calc("sin(30 deg)"), "0.5");
    assert_eq!(calc("toDegrees(pi / 4)"), "45");
    assert_eq!(calc("toRadians(180)"), calc("pi"));
    assert_eq!(calc("deg(pi)"), "180");
    assert_eq!(calc("degrees(1 rad)"), calc("180 / pi"));
    assert_eq!(calc("dist2d(0, 0, 3, 4)"), "5");
    assert_eq!(calc("distance([0, 0, 0], [2, 3, 6])"), "7");
}

#[test]
fn special_functions() {
    close("gamma(0.5)", std::f64::consts::PI.sqrt());
    close("gamma(7.3)", 1271.4236336639085);
    close("gamma(170.5)", 5.56209241456e305);
    close("gamma(-2.5)", -0.9453087204829417);
    assert_eq!(calc("gamma(5)"), "24");
    close("erf(0.5)", 0.5204998778130465);
    close("erf(1)", 0.8427007929497149);
    close("erf(3)", 0.9999779095030014);
    close("erf(4.5)", 0.9999999998033839);
    close("erf(-1.2)", -0.9103139782296353);
}

#[test]
fn statistics() {
    let d = "[2, 4, 4, 4, 5, 5, 7, 9, 1.5]";
    close(&format!("mean({d})"), 4.611111111111111);
    close(&format!("median({d})"), 4.0);
    close(&format!("std({d})"), 2.3154073315749675);
    close(&format!("stddev({d})"), 2.1829869671542776);
    close(&format!("variance({d})"), 4.765432098765432);
    close(&format!("variance({d}, \"unbiased\")"), 5.361111111111111);
    close(&format!("geomean({d})"), 4.06399843346462);
    close(&format!("harmean({d})"), 3.5010805804260574);
    close(&format!("quantileSeq({d}, 0.3)"), 4.0);
    close(&format!("mad({d})"), 1.0);
    assert_eq!(calc("percentile([1, 2, 3, 4, 5], 25)"), "2");
    assert_eq!(calc("quantileSeq([1, 2, 3, 4], [0, 0.5, 1])"), "[1, 2.5, 4]");
    assert_eq!(calc("sum(1, 2, 3)"), "6");
    assert_eq!(calc("sum([1, 2], 3)"), "6");
    assert_eq!(calc("sum()"), "0");
    assert_eq!(calc("prod(1, 2, 3, 4)"), "24");
    assert_eq!(calc("count([1, 2, 3])"), "3");
    assert_eq!(calc("min(3, 1, 2)"), "1");
    assert_eq!(calc("max([3, 1, 2])"), "3");
    assert_eq!(calc("mode(1, 2, 2, 3)"), "2");
    assert_eq!(calc("mode(1, 1, 2, 2)"), "[1, 2]");
    assert_eq!(calc("cumsum(1, 2, 3)"), "[1, 3, 6]");
    assert_eq!(calc("rms(3, 4)"), calc("sqrt(12.5)"));
    assert_eq!(calc("rangeCalc(1, 5, 3, 9, 2)"), "8");
    close("kldivergence([0.5, 0.5], [0.9, 0.1])", 0.5108256237659907);
    assert!(error("mean()").contains("empty"));
    assert!(error("std(5)").contains("at least 2"));
}

#[test]
fn statistics_with_units_and_complex() {
    assert_eq!(calc("sum([1 m, 50 cm])"), "1.5 m");
    assert_eq!(calc("mean([1 km, 500 m])"), "0.75 km");
    assert_eq!(calc("max(1 h, 59 min)"), "1 h");
    assert_eq!(calc("median([3 s, 1 min, 2 s])"), "3 s");
    assert_eq!(calc("std([1 m, 2 m, 3 m])"), "1 m");
    assert_eq!(calc("sum([1 + i, 2 - 3i])"), "3 - 2i");
    assert!(error("sum([1 m, 2])").contains("Cannot add"));
}

#[test]
fn combinatorics() {
    assert_eq!(calc("combinations(52, 5)"), "2598960");
    assert_eq!(full("combinations(60, 30)"), "118264581564861420");
    assert_eq!(calc("permutations(10, 3)"), "720");
    assert_eq!(calc("permutations(5)"), "120");
    assert_eq!(calc("multinomial([2, 3, 4])"), "1260");
    assert_eq!(calc("catalan(10)"), "16796");
    assert_eq!(calc("stirlingS2(10, 4)"), "34105");
    assert_eq!(calc("bellNumbers(10)"), "115975");
    assert_eq!(calc("composition(10, 3)"), "36");
    assert_eq!(calc("factorial(20)"), "2.4329020081766e+18");
    assert_eq!(full("factorial(20)"), "2432902008176640000");
    assert_eq!(calc("0!"), "1");
    close("2.5!", 3.3233509704478426);
    assert_eq!(calc("factorial(171)"), "Infinity");
    assert!(error("(-1)!").contains("negative"));
    assert!(error("combinations(3, 5)").contains("larger"));
}

#[test]
fn number_theory() {
    assert_eq!(calc("gcd(48, 18, 30)"), "6");
    assert_eq!(calc("gcd(0, 5)"), "5");
    assert_eq!(calc("lcm(4, 6, 10)"), "60");
    assert_eq!(calc("xgcd(240, 46)"), "[2, -9, 47]");
    assert_eq!(calc("invmod(3, 11)"), "4");
    assert!(error("invmod(4, 8)").contains("no inverse"));
    assert_eq!(calc("isPrime(1000000007)"), "true");
    assert!(error("isPrime(2^60)").contains("2^53"));
    assert_eq!(calc("isPrime(9007199254740881)"), "true");
    assert_eq!(calc("isPrime([2, 4, 97])"), "[true, false, true]");
    assert!(error("gcd(1.5, 3)").contains("whole number"));
}

#[test]
fn random_numbers_stay_in_range() {
    for _ in 0..50 {
        let r: f64 = full("random()").parse().unwrap();
        assert!((0.0..1.0).contains(&r));
        let r: f64 = full("random(5, 10)").parse().unwrap();
        assert!((5.0..10.0).contains(&r));
        let n: f64 = full("randomInt(1, 7)").parse().unwrap();
        assert!((1.0..7.0).contains(&n) && n.fract() == 0.0);
        assert!(["1", "2", "3"].contains(&full("pickRandom([1, 2, 3])").as_str()));
    }
}

#[test]
fn matrices() {
    assert_eq!(calc("det([1, 2; 3, 4])"), "-2");
    assert_eq!(calc("inv([1, 2; 3, 4])"), "[[-2, 1], [1.5, -0.5]]");
    assert!(error("inv([1, 2; 2, 4])").contains("determinant is zero"));
    assert_eq!(calc("transpose([1, 2, 3; 4, 5, 6])"), "[[1, 4], [2, 5], [3, 6]]");
    assert_eq!(calc("eigs([4, 1; 2, 3])"), "[2, 5]");
    assert_eq!(calc("eigs([2, 1; 1, 2])"), "[1, 3]");
    assert_eq!(calc("eigs([0, 1; -1, 0])"), "[-i, i]");
    assert_eq!(calc("lusolve([2, 1; 1, 3], [3, 5])"), "[0.8, 1.4]");
    assert!(error("lusolve([1, 2; 2, 4], [1, 2])").contains("singular"));
    assert_eq!(calc("lsolve([2, 0; 1, 3], [4, 5])"), "[2, 1]");
    assert_eq!(calc("usolve([2, 1; 0, 3], [4, 6])"), "[1, 2]");
    assert_eq!(calc("pinv([1, 2; 2, 4])"), "[[0.04, 0.08], [0.08, 0.16]]");
    assert_eq!(calc("trace([1, 2; 3, 4])"), "5");
    assert_eq!(calc("diag([1, 2])"), "[[1, 0], [0, 2]]");
    assert_eq!(calc("diag([1, 2; 3, 4])"), "[1, 4]");
    assert_eq!(calc("dot([1, 2, 3], [4, 5, 6])"), "32");
    assert_eq!(calc("cross([1, 0, 0], [0, 1, 0])"), "[0, 0, 1]");
    assert_eq!(calc("kron([1, 2], [1, 10])"), "[1, 10, 2, 20]");
    assert_eq!(calc("norm([3, 4])"), "5");
    assert_eq!(calc("norm([3, 4], 1)"), "7");
    assert_eq!(calc("norm([3, -4], Infinity)"), "4");
    assert_eq!(calc("norm([1, 2; 3, 4], 1)"), "6");
    close("norm([1, 2; 3, 4])", 30f64.sqrt());
    close("norm([1, 2; 3, 4], 2)", 5.464985704219043);
    assert_eq!(calc("zeros(2, 2)"), "[[0, 0], [0, 0]]");
    assert_eq!(calc("ones(3)"), "[1, 1, 1]");
    assert_eq!(calc("identity(2)"), "[[1, 0], [0, 1]]");
    assert_eq!(calc("range(1, 5)"), "[1, 2, 3, 4]");
    assert_eq!(calc("range(0, 1, 0.25)"), "[0, 0.25, 0.5, 0.75]");
    assert_eq!(calc("size([1, 2; 3, 4; 5, 6])"), "[3, 2]");
    assert_eq!(calc("size(5)"), "[]");
    assert_eq!(calc("flatten([1, [2, 3], [[4]]])"), "[1, 2, 3, 4]");
    assert_eq!(calc("reshape(1:6, [2, 3])"), "[[1, 2, 3], [4, 5, 6]]");
    assert!(error("reshape(1:6, [4, 2])").contains("cannot fit"));
    assert_eq!(calc("resize([1, 2; 3, 4], [3, 3], 9)"), "[[1, 2, 9], [3, 4, 9], [9, 9, 9]]");
    assert_eq!(calc("squeeze([[1, 2, 3]])"), "[1, 2, 3]");
    assert_eq!(calc("squeeze([[5]])"), "5");
    assert_eq!(calc("concat([1, 2], [3])"), "[1, 2, 3]");
    assert_eq!(calc("concat([1, 2; 3, 4], [5; 6])"), "[[1, 2, 5], [3, 4, 6]]");
    assert_eq!(calc("concat([1, 2; 3, 4], [5, 6], 1)"), "[[1, 2], [3, 4], [5, 6]]");
    assert_eq!(calc("row([1, 2; 3, 4], 2)"), "[3, 4]");
    assert_eq!(calc("column([1, 2; 3, 4], 2)"), "[2, 4]");
    assert_eq!(calc("sort([3, 1, 2])"), "[1, 2, 3]");
    assert_eq!(calc("sort([3, 1, 2], \"desc\")"), "[3, 2, 1]");
    assert_eq!(calc("sort([2 m, 50 cm, 1 km])"), "[50 cm, 2 m, 1 km]");
    assert_eq!(calc("reverse([1, 2, 3])"), "[3, 2, 1]");
    assert_eq!(calc("partitionSelect([5, 1, 4, 2], 1)"), "2");
    assert_eq!(calc("map([1, 4, 9], sqrt)"), "[1, 2, 3]");
    assert_eq!(calc("f(x) = x > 2\nfilter([1, 2, 3, 4], f)"), "[3, 4]");
    assert_eq!(calc("double(x) = 2x\nmap([1, 2; 3, 4], double)"), "[[2, 4], [6, 8]]");
}

#[test]
fn sets() {
    assert_eq!(calc("setUnion([1, 2, 3], [2, 5])"), "[1, 2, 3, 5]");
    assert_eq!(calc("setIntersect([1, 2, 3], [2, 3, 4])"), "[2, 3]");
    assert_eq!(calc("setDifference([1, 2, 3], [2])"), "[1, 3]");
    assert_eq!(calc("setSymDifference([1, 2, 3], [2, 3, 4])"), "[1, 4]");
    assert_eq!(calc("setDistinct([3, 1, 3, 2])"), "[1, 2, 3]");
    assert_eq!(calc("setIsSubset([1, 2], [1, 2, 3])"), "true");
    assert_eq!(calc("setSize([1, 1, 2])"), "3");
    assert_eq!(calc("setSize([1, 1, 2], true)"), "2");
    assert_eq!(calc("setMultiplicity(1, [1, 1, 2])"), "2");
    assert_eq!(calc("setCartesian([1, 2], [3, 4])"), "[[1, 3], [1, 4], [2, 3], [2, 4]]");
    assert_eq!(calc("setPowerset([1, 2])"), "[[], [1], [2], [1, 2]]");
}

#[test]
fn logic_and_bits() {
    assert_eq!(calc("equal(2, 2)"), "true");
    assert_eq!(calc("largerEq(2, 3)"), "false");
    assert_eq!(calc("compare(2, 3)"), "-1");
    assert_eq!(calc("compare(1 km, 999 m)"), "1");
    assert_eq!(calc("deepEqual([1, [2]], [1, [2]])"), "true");
    assert_eq!(calc("deepEqual([1, 2], [1, 3])"), "false");
    assert_eq!(calc("compareText(\"a\", \"b\")"), "-1");
    assert_eq!(calc("equalText(\"a\", \"a\")"), "true");
    assert_eq!(calc("bitAnd(12, 10)"), "8");
    assert_eq!(calc("bitOr(12, 10)"), "14");
    assert_eq!(calc("bitXor(12, 10)"), "6");
    assert_eq!(calc("bitNot(5)"), "-6");
    assert_eq!(calc("leftShift(1, 10)"), "1024");
    assert_eq!(calc("rightArithShift(-16, 2)"), "-4");
    assert_eq!(calc("rightLogShift(16, 2)"), "4");
    assert!(error("leftShift(1, 60)").contains("2^53"));
}

#[test]
fn types_and_conversion() {
    assert_eq!(calc("typeOf(5)"), "number");
    assert_eq!(calc("typeOf(5 km)"), "Unit");
    assert_eq!(calc("typeOf(1 + i)"), "Complex");
    assert_eq!(calc("typeOf([1])"), "Matrix");
    assert_eq!(calc("isInteger(4.0)"), "true");
    assert_eq!(calc("isNaN(0 / 0)"), "true");
    assert_eq!(calc("isNegative(-2 m)"), "true");
    assert_eq!(calc("isNumeric(\"5\")"), "false");
    assert_eq!(calc("hasNumericValue(\"5\")"), "true");
    assert_eq!(calc("number(\"3.5\")"), "3.5");
    assert_eq!(calc("number(true)"), "1");
    assert_eq!(calc("string(1 / 4)"), "0.25");
    assert_eq!(calc("format(pi, 3)"), "3.14");
    assert_eq!(calc("boolean(\"false\")"), "false");
    assert_eq!(calc("hex(255)"), "0xff");
    assert_eq!(calc("bin(-5)"), "-0b101");
    assert_eq!(calc("oct(64)"), "0o100");
    assert_eq!(calc("bignumber(\"0.1\") + bignumber(\"0.2\") == bignumber(\"0.3\")"), "true");
    assert!(error("number(\"abc\")").contains("Cannot convert"));
}

/// The original calctxt's own functions, ported from its test suite. Its text
/// "Error: ..." results are now real errors.
#[test]
fn calctxt_functions() {
    assert_eq!(calc("trend(10, 15, 12, 18)"), "Values: [10, 15, 12, 18]\n10 → 15 (+5)\n15 → 12 (-3)\n12 → 18 (+6)\nAverage change: 2.667\nMax change: 6\nMin change: -3");
    assert_eq!(calc("trend([1, 2, 4, 7])"), calc("trend(1, 2, 4, 7)"));
    assert!(error("trend(5)").contains("at least 2"));
    assert_eq!(calc("stddev()"), "0");
    close("stddev(1, 2, 3, 4, 5)", 2f64.sqrt());
    close("variance([2, 4, 4, 4, 5, 5, 7, 9])", 4.0);
    assert_eq!(calc("rangeCalc(-5, 3, -2, 7)"), "12");
    close("geomean(1, 3, 9)", 3.0);
    assert!(error("geomean(1, -2, 3)").contains("positive"));
    close("harmean(1, 2, 4)", 12.0 / 7.0);
    assert!(error("harmean(1, 0, 3)").contains("zero"));
    close("rms(1, 2, 3)", (14.0f64 / 3.0).sqrt());
    assert_eq!(calc("pctchange(100, 150)"), "50");
    assert_eq!(calc("pctchange(100, 75)"), "-25");
    assert!(error("pctchange(0, 100)").contains("zero"));
    close("cagr(1000, 1500, 3)", 14.471424255333186);
    assert!(error("cagr(-1000, 1500, 3)").contains("positive"));
    close("compound(1000, 5, 10, 4)", 1000.0 * (1.0 + 0.05 / 4.0f64).powi(40));
    assert_eq!(calc("sma(3, 10, 12, 15, 18)"), "Period 3: 12.333\nPeriod 4: 15.000");
    assert!(error("sma(5, 1, 2)").contains("at least 5"));
    assert_eq!(calc("gcd(48, 18)"), "6");
    assert_eq!(calc("lcm(12, 8)"), "24");
}
