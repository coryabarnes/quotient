//! Every built-in function with its signature and a one-line description.
//!
//! This list drives the help screen, autocomplete and syntax highlighting, and a
//! test checks that each entry is implemented.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub name: &'static str,
    pub signature: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

pub const ARITHMETIC: &str = "Arithmetic";
pub const POWERS: &str = "Powers, roots & logarithms";
pub const ROUNDING: &str = "Rounding";
pub const TRIG: &str = "Trigonometry";
pub const COMPLEX: &str = "Complex numbers";
pub const STATS: &str = "Statistics";
pub const PROBABILITY: &str = "Probability & combinatorics";
pub const NUMBER_THEORY: &str = "Number theory";
pub const MATRICES: &str = "Matrices & lists";
pub const SETS: &str = "Sets";
pub const LOGIC: &str = "Logic & bits";
pub const CONVERSION: &str = "Units, types & formatting";
pub const FINANCE: &str = "Finance";
pub const GEOMETRY: &str = "Geometry & angles";

macro_rules! entries {
    ($($cat:ident: $name:literal, $sig:literal, $desc:literal;)*) => {
        pub static ENTRIES: &[Entry] = &[
            $(Entry { name: $name, signature: $sig, description: $desc, category: $cat },)*
        ];
    };
}

entries! {
    ARITHMETIC: "add", "add(a, b, ...)", "Sum of the arguments (same as +)";
    ARITHMETIC: "subtract", "subtract(a, b)", "a - b";
    ARITHMETIC: "multiply", "multiply(a, b, ...)", "Product of the arguments (same as *)";
    ARITHMETIC: "divide", "divide(a, b)", "a / b";
    ARITHMETIC: "mod", "mod(a, b)", "Remainder with the sign of b (same as a mod b)";
    ARITHMETIC: "abs", "abs(x)", "Absolute value (modulus for complex numbers)";
    ARITHMETIC: "sign", "sign(x)", "-1, 0 or 1 depending on the sign of x";
    ARITHMETIC: "unaryMinus", "unaryMinus(x)", "-x";
    ARITHMETIC: "unaryPlus", "unaryPlus(x)", "x";
    ARITHMETIC: "pctchange", "pctchange(old, new)", "Percentage change from old to new";
    ARITHMETIC: "hypot", "hypot(a, b, ...)", "Square root of the sum of squares";
    ARITHMETIC: "norm", "norm(x, p?)", "Vector or matrix norm (p = 1, 2, Infinity or \"fro\")";

    POWERS: "pow", "pow(x, y)", "x to the power y (same as x ^ y)";
    POWERS: "sqrt", "sqrt(x)", "Square root; negative numbers give complex results";
    POWERS: "cbrt", "cbrt(x)", "Cube root";
    POWERS: "nthRoot", "nthRoot(x, n)", "Real n-th root; nthRoot(-27, 3) = -3";
    POWERS: "nthRoots", "nthRoots(x, n)", "All n complex n-th roots";
    POWERS: "square", "square(x)", "x²";
    POWERS: "cube", "cube(x)", "x³";
    POWERS: "exp", "exp(x)", "e to the power x";
    POWERS: "expm1", "expm1(x)", "exp(x) - 1, accurate for small x";
    POWERS: "log", "log(x, base?)", "Natural logarithm, or logarithm in the given base";
    POWERS: "ln", "ln(x)", "Natural logarithm";
    POWERS: "log10", "log10(x)", "Base-10 logarithm";
    POWERS: "log2", "log2(x)", "Base-2 logarithm";
    POWERS: "log1p", "log1p(x)", "ln(1 + x), accurate for small x";

    ROUNDING: "round", "round(x, digits?)", "Round half away from zero, optionally to decimal places";
    ROUNDING: "floor", "floor(x)", "Round down";
    ROUNDING: "ceil", "ceil(x)", "Round up";
    ROUNDING: "fix", "fix(x)", "Round toward zero";

    TRIG: "sin", "sin(x)", "Sine (radians, or an angle like 30 deg)";
    TRIG: "cos", "cos(x)", "Cosine";
    TRIG: "tan", "tan(x)", "Tangent";
    TRIG: "sec", "sec(x)", "Secant, 1 / cos(x)";
    TRIG: "csc", "csc(x)", "Cosecant, 1 / sin(x)";
    TRIG: "cot", "cot(x)", "Cotangent, 1 / tan(x)";
    TRIG: "asin", "asin(x)", "Inverse sine, in radians";
    TRIG: "acos", "acos(x)", "Inverse cosine";
    TRIG: "atan", "atan(x)", "Inverse tangent";
    TRIG: "atan2", "atan2(y, x)", "Angle of the point (x, y), in radians";
    TRIG: "asec", "asec(x)", "Inverse secant";
    TRIG: "acsc", "acsc(x)", "Inverse cosecant";
    TRIG: "acot", "acot(x)", "Inverse cotangent";
    TRIG: "sinh", "sinh(x)", "Hyperbolic sine";
    TRIG: "cosh", "cosh(x)", "Hyperbolic cosine";
    TRIG: "tanh", "tanh(x)", "Hyperbolic tangent";
    TRIG: "sech", "sech(x)", "Hyperbolic secant";
    TRIG: "csch", "csch(x)", "Hyperbolic cosecant";
    TRIG: "coth", "coth(x)", "Hyperbolic cotangent";
    TRIG: "asinh", "asinh(x)", "Inverse hyperbolic sine";
    TRIG: "acosh", "acosh(x)", "Inverse hyperbolic cosine";
    TRIG: "atanh", "atanh(x)", "Inverse hyperbolic tangent";
    TRIG: "asech", "asech(x)", "Inverse hyperbolic secant";
    TRIG: "acsch", "acsch(x)", "Inverse hyperbolic cosecant";
    TRIG: "acoth", "acoth(x)", "Inverse hyperbolic cotangent";

    GEOMETRY: "toRadians", "toRadians(degrees)", "Degrees to radians";
    GEOMETRY: "radians", "radians(degrees)", "Degrees to radians";
    GEOMETRY: "rad", "rad(degrees)", "Degrees to radians";
    GEOMETRY: "toDegrees", "toDegrees(radians)", "Radians to degrees";
    GEOMETRY: "degrees", "degrees(radians)", "Radians to degrees";
    GEOMETRY: "deg", "deg(radians)", "Radians to degrees";
    GEOMETRY: "dist2d", "dist2d(x1, y1, x2, y2)", "Distance between two points";
    GEOMETRY: "distance", "distance(p, q)", "Distance between two points given as [x, y] or [x, y, z]";

    COMPLEX: "complex", "complex(re, im)", "Builds re + im·i";
    COMPLEX: "re", "re(z)", "Real part";
    COMPLEX: "im", "im(z)", "Imaginary part";
    COMPLEX: "arg", "arg(z)", "Angle of z in radians";
    COMPLEX: "conj", "conj(z)", "Complex conjugate";

    STATS: "sum", "sum(values...)", "Sum (takes numbers, a list, or quantities with units)";
    STATS: "prod", "prod(values...)", "Product";
    STATS: "count", "count(values...)", "Number of values";
    STATS: "mean", "mean(values...)", "Average";
    STATS: "median", "median(values...)", "Middle value";
    STATS: "mode", "mode(values...)", "Most frequent value(s)";
    STATS: "min", "min(values...)", "Smallest value";
    STATS: "max", "max(values...)", "Largest value";
    STATS: "std", "std(values...)", "Sample standard deviation (n - 1)";
    STATS: "stddev", "stddev(values...)", "Population standard deviation (n)";
    STATS: "variance", "variance(values...)", "Population variance (n); add \"unbiased\" for sample variance";
    STATS: "mad", "mad(values...)", "Median absolute deviation";
    STATS: "quantileSeq", "quantileSeq(values, p)", "Quantile for 0 ≤ p ≤ 1 (linear interpolation)";
    STATS: "percentile", "percentile(values, p)", "Percentile for 0 ≤ p ≤ 100";
    STATS: "cumsum", "cumsum(values...)", "Running totals";
    STATS: "rangeCalc", "rangeCalc(values...)", "Largest minus smallest";
    STATS: "geomean", "geomean(values...)", "Geometric mean of positive values";
    STATS: "harmean", "harmean(values...)", "Harmonic mean";
    STATS: "rms", "rms(values...)", "Root mean square";
    STATS: "trend", "trend(values...)", "Step-by-step changes with average, max and min change";
    STATS: "sma", "sma(periods, values...)", "Simple moving average";
    STATS: "kldivergence", "kldivergence(p, q)", "Kullback-Leibler divergence of two distributions";

    PROBABILITY: "factorial", "factorial(n)", "n! (also written n!); non-integers use the gamma function";
    PROBABILITY: "gamma", "gamma(x)", "Gamma function";
    PROBABILITY: "erf", "erf(x)", "Error function";
    PROBABILITY: "combinations", "combinations(n, k)", "Ways to choose k of n (nCk)";
    PROBABILITY: "permutations", "permutations(n, k?)", "Ordered ways to choose k of n (nPk)";
    PROBABILITY: "multinomial", "multinomial([a, b, ...])", "Multinomial coefficient";
    PROBABILITY: "catalan", "catalan(n)", "n-th Catalan number";
    PROBABILITY: "bellNumbers", "bellNumbers(n)", "Number of partitions of an n-element set";
    PROBABILITY: "stirlingS2", "stirlingS2(n, k)", "Stirling number of the second kind";
    PROBABILITY: "composition", "composition(n, k)", "Compositions of n into k parts";
    PROBABILITY: "random", "random(min?, max?)", "Random number in [0, 1) or [min, max)";
    PROBABILITY: "randomInt", "randomInt(min?, max)", "Random whole number in [min, max)";
    PROBABILITY: "pickRandom", "pickRandom(list)", "A random element of a list";

    NUMBER_THEORY: "gcd", "gcd(a, b, ...)", "Greatest common divisor";
    NUMBER_THEORY: "lcm", "lcm(a, b, ...)", "Least common multiple";
    NUMBER_THEORY: "xgcd", "xgcd(a, b)", "[gcd, x, y] with a·x + b·y = gcd";
    NUMBER_THEORY: "invmod", "invmod(a, m)", "Inverse of a modulo m";
    NUMBER_THEORY: "isPrime", "isPrime(n)", "Whether n is prime";

    MATRICES: "matrix", "matrix(list)", "A matrix (lists are already matrices)";
    MATRICES: "zeros", "zeros(rows, cols?)", "List or matrix of zeros";
    MATRICES: "ones", "ones(rows, cols?)", "List or matrix of ones";
    MATRICES: "identity", "identity(n)", "n×n identity matrix";
    MATRICES: "range", "range(start, end, step?)", "Numbers from start up to (not including) end";
    MATRICES: "size", "size(x)", "Dimensions of a matrix";
    MATRICES: "transpose", "transpose(M)", "Swap rows and columns";
    MATRICES: "det", "det(M)", "Determinant";
    MATRICES: "inv", "inv(M)", "Inverse matrix";
    MATRICES: "pinv", "pinv(M)", "Moore-Penrose pseudo-inverse";
    MATRICES: "trace", "trace(M)", "Sum of the diagonal";
    MATRICES: "diag", "diag(x)", "Diagonal matrix from a list, or the diagonal of a matrix";
    MATRICES: "dot", "dot(a, b)", "Dot product";
    MATRICES: "cross", "cross(a, b)", "Cross product of 3-vectors";
    MATRICES: "kron", "kron(A, B)", "Kronecker product";
    MATRICES: "eigs", "eigs(M)", "Eigenvalues (complex for non-symmetric matrices)";
    MATRICES: "lusolve", "lusolve(A, b)", "Solve the linear system A·x = b";
    MATRICES: "lsolve", "lsolve(L, b)", "Solve L·x = b for lower-triangular L";
    MATRICES: "usolve", "usolve(U, b)", "Solve U·x = b for upper-triangular U";
    MATRICES: "flatten", "flatten(M)", "All elements in one list";
    MATRICES: "reshape", "reshape(M, [rows, cols])", "Same elements in a new shape";
    MATRICES: "resize", "resize(M, [rows, cols], fill?)", "Grow or shrink, padding with fill (0)";
    MATRICES: "squeeze", "squeeze(M)", "Remove dimensions of size 1";
    MATRICES: "concat", "concat(A, B, ..., dim?)", "Join lists, or matrices along dim (1 = rows, 2 = columns)";
    MATRICES: "row", "row(M, i)", "Row i";
    MATRICES: "column", "column(M, j)", "Column j";
    MATRICES: "sort", "sort(list, \"desc\"?)", "Sorted copy";
    MATRICES: "reverse", "reverse(list)", "Reversed copy";
    MATRICES: "partitionSelect", "partitionSelect(list, k)", "k-th smallest value (k from 0)";
    MATRICES: "map", "map(list, f)", "Apply function f to every element";
    MATRICES: "filter", "filter(list, f)", "Keep elements where f(x) is true";

    SETS: "setUnion", "setUnion(a, b)", "Values in a or b";
    SETS: "setIntersect", "setIntersect(a, b)", "Values in both a and b";
    SETS: "setDifference", "setDifference(a, b)", "Values in a but not b";
    SETS: "setSymDifference", "setSymDifference(a, b)", "Values in exactly one of a and b";
    SETS: "setDistinct", "setDistinct(a)", "Distinct values";
    SETS: "setIsSubset", "setIsSubset(a, b)", "Whether every value of a is in b";
    SETS: "setSize", "setSize(a, distinct?)", "Number of values (distinct ones if distinct is true)";
    SETS: "setMultiplicity", "setMultiplicity(x, a)", "How many times x occurs in a";
    SETS: "setCartesian", "setCartesian(a, b)", "All pairs [x, y]";
    SETS: "setPowerset", "setPowerset(a)", "All subsets";

    LOGIC: "and", "and(a, b)", "Logical and (same as a and b)";
    LOGIC: "or", "or(a, b)", "Logical or";
    LOGIC: "xor", "xor(a, b)", "Logical exclusive or";
    LOGIC: "not", "not(x)", "Logical not";
    LOGIC: "equal", "equal(a, b)", "a == b";
    LOGIC: "unequal", "unequal(a, b)", "a != b";
    LOGIC: "smaller", "smaller(a, b)", "a < b";
    LOGIC: "smallerEq", "smallerEq(a, b)", "a <= b";
    LOGIC: "larger", "larger(a, b)", "a > b";
    LOGIC: "largerEq", "largerEq(a, b)", "a >= b";
    LOGIC: "compare", "compare(a, b)", "-1, 0 or 1";
    LOGIC: "deepEqual", "deepEqual(a, b)", "Whether two values (or lists) are identical";
    LOGIC: "compareText", "compareText(a, b)", "Alphabetical comparison of text: -1, 0 or 1";
    LOGIC: "equalText", "equalText(a, b)", "Whether two texts are equal";
    LOGIC: "bitAnd", "bitAnd(a, b)", "Bitwise and of whole numbers";
    LOGIC: "bitOr", "bitOr(a, b)", "Bitwise or";
    LOGIC: "bitXor", "bitXor(a, b)", "Bitwise exclusive or";
    LOGIC: "bitNot", "bitNot(a)", "Bitwise not";
    LOGIC: "leftShift", "leftShift(a, n)", "Shift bits left";
    LOGIC: "rightArithShift", "rightArithShift(a, n)", "Shift bits right, keeping the sign";
    LOGIC: "rightLogShift", "rightLogShift(a, n)", "Shift bits right, filling with zeros (64-bit)";

    CONVERSION: "number", "number(x, unit?)", "A plain number; with a unit, the value in that unit";
    CONVERSION: "string", "string(x)", "Value as text";
    CONVERSION: "format", "format(x, digits?)", "Value as text with the given significant digits";
    CONVERSION: "boolean", "boolean(x)", "true or false";
    CONVERSION: "bignumber", "bignumber(x)", "High-precision number (use text for exact input: bignumber(\"0.1\"))";
    CONVERSION: "unit", "unit(value, \"unit\")", "Quantity from a number and unit text";
    CONVERSION: "splitUnit", "splitUnit(q, [units])", "Split a quantity, e.g. splitUnit(1.75 h, [h, min])";
    CONVERSION: "hex", "hex(n)", "Whole number in hexadecimal";
    CONVERSION: "bin", "bin(n)", "Whole number in binary";
    CONVERSION: "oct", "oct(n)", "Whole number in octal";
    CONVERSION: "typeOf", "typeOf(x)", "Type name of a value";
    CONVERSION: "isInteger", "isInteger(x)", "Whether x is a whole number";
    CONVERSION: "isNaN", "isNaN(x)", "Whether x is NaN";
    CONVERSION: "isNegative", "isNegative(x)", "Whether x < 0";
    CONVERSION: "isPositive", "isPositive(x)", "Whether x > 0";
    CONVERSION: "isZero", "isZero(x)", "Whether x is 0";
    CONVERSION: "isNumeric", "isNumeric(x)", "Whether x is a number";
    CONVERSION: "hasNumericValue", "hasNumericValue(x)", "Whether x is a number or numeric text";

    FINANCE: "compound", "compound(principal, ratePercent, years, perYear?)", "Compound interest: final amount";
    FINANCE: "cagr", "cagr(start, end, years)", "Compound annual growth rate, in percent";
    FINANCE: "pmt", "pmt(rate, periods, pv, fv?, type?)", "Loan or annuity payment per period (like Excel)";
    FINANCE: "fv", "fv(rate, periods, pmt, pv?, type?)", "Future value (like Excel)";
    FINANCE: "pv", "pv(rate, periods, pmt, fv?, type?)", "Present value (like Excel)";
    FINANCE: "nper", "nper(rate, pmt, pv, fv?, type?)", "Number of periods (like Excel)";
    FINANCE: "npv", "npv(rate, cashflows...)", "Net present value, flows from the end of period 1 (like Excel)";
    FINANCE: "irr", "irr(cashflows...)", "Internal rate of return (like Excel)";
}

pub fn get(name: &str) -> Option<&'static Entry> {
    ENTRIES.iter().find(|e| e.name == name)
}

/// Categories in display order.
pub fn categories() -> Vec<&'static str> {
    let mut seen = Vec::new();
    for e in ENTRIES {
        if !seen.contains(&e.category) {
            seen.push(e.category);
        }
    }
    seen
}
