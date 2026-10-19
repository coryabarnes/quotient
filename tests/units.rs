//! Units. Conversion factors are exact definitions (NIST SP 811 / SI).

mod common;
use common::{calc, error, full};

fn value(doc: &str, unit: &str) -> f64 {
    let s = full(doc);
    let v = s.strip_suffix(unit).unwrap_or_else(|| panic!("{doc}: expected unit {unit}, got {s}"));
    v.trim().parse().unwrap()
}

fn approx(doc: &str, unit: &str, expected: f64) {
    let got = value(doc, unit);
    assert!((got - expected).abs() <= 1e-12 * expected.abs().max(1e-300), "{doc}: expected {expected} {unit}, got {got}");
}

#[test]
fn conversions() {
    approx("1 inch to cm", "cm", 2.54);
    approx("1 mi to km", "km", 1.609344);
    approx("1 lb to kg", "kg", 0.45359237);
    approx("1 gal to L", "L", 3.785411784);
    approx("1 atm to psi", "psi", 101325.0 / 6894.757293168361);
    approx("1 kWh to J", "J", 3.6e6);
    approx("1 knot to kph", "kph", 1.852);
    approx("1 acre to m^2", "m^2", 4046.8564224);
    approx("100 km/h to mph", "mph", 100000.0 / 3600.0 / 0.44704);
    approx("1 cup to tsp", "tsp", 48.0);
    approx("1 ly to km", "km", 9.4607304725808e12);
    approx("1 au to m", "m", 149597870700.0);
    approx("1 eV to J", "J", 1.602176634e-19);
    approx("1 hp to W", "W", 745.6998715822702);
    approx("1 mmHg to Pa", "Pa", 133.322387415);
    approx("1 BTU to J", "J", 1055.05585262);
}

#[test]
fn temperatures_use_offsets() {
    assert_eq!(calc("100 degC to degF"), "212 degF");
    assert_eq!(calc("-40 degF to degC"), "-40 degC");
    assert_eq!(calc("0 K to degC"), "-273.15 degC");
    assert_eq!(calc("72 °F to °C"), "22.222222222222 degC");
    assert_eq!(calc("300 K to degF"), "80.33 degF");
    assert_eq!(calc("491.67 degR to degF"), "32 degF");
    assert_eq!(calc("20 degC + 5 degC"), "25 degC");
    assert_eq!(calc("2 * 20 degC"), "40 degC");
}

#[test]
fn prefixes_names_and_plurals() {
    assert_eq!(calc("1 km to m"), "1000 m");
    assert_eq!(calc("1 mm to um"), "1000 um");
    assert_eq!(calc("1 µs to ns"), "1000 ns");
    assert_eq!(calc("3 kilometers + 200 meters"), "3.2 km");
    assert_eq!(calc("2 hours to seconds"), "7200 s");
    assert_eq!(calc("5 inches to cm"), "12.7 cm");
    assert_eq!(calc("1 KiB to B"), "1024 B");
    assert_eq!(calc("1.5 GiB to MB"), "1610.612736 MB");
    assert_eq!(calc("8 Mbit to MB"), "1 MB");
    assert_eq!(calc("1 kibibyte to bytes"), "1024 B");
    assert_eq!(calc("500 mL to L"), "0.5 L");
    assert_eq!(calc("1000 mAh to C"), "3600 C");
    assert_eq!(calc("1 hPa to Pa"), "100 Pa");
    // Exact names win over prefix + unit: min is minutes, not milli-inches.
    assert_eq!(calc("1 min to s"), "60 s");
    assert_eq!(calc("1 pt to mL"), "473.176473 mL");
}

#[test]
fn arithmetic() {
    assert_eq!(calc("5 km + 300 m"), "5.3 km");
    assert_eq!(calc("5 m * 3 m"), "15 m^2");
    assert_eq!(calc("10 km / 2 h"), "5 km / h");
    assert_eq!(calc("10 km / 500 m"), "20");
    assert_eq!(calc("(10 m)^2"), "100 m^2");
    assert_eq!(calc("sqrt(16 m^2)"), "4 m");
    assert_eq!(calc("2 kg * 9.81 m/s^2 to N"), "19.62 N");
    assert_eq!(calc("1 N m to J"), "1 J");
    assert_eq!(calc("60 mph * 2.5 h to km"), "241.4016 km");
    assert_eq!(calc("1 / (2 s)"), "0.5 s^-1");
    assert_eq!(calc("60 rpm to Hz"), "1 Hz");
    assert_eq!(calc("5 km < 6000 m"), "true");
    assert_eq!(calc("1 h + 30 min to min"), "90 min");
    assert_eq!(calc("abs(-5 km)"), "5 km");
    assert_eq!(calc("round(5.567 km, 1)"), "5.6 km");
    assert_eq!(calc("10% of 50 kg"), "5 kg");
    assert_eq!(calc("50 m + 10%"), "55 m");
}

#[test]
fn angles_feed_trigonometry() {
    assert_eq!(calc("sin(30 deg)"), "0.5");
    assert_eq!(calc("cos(0.5 turn)"), "-1");
    assert_eq!(calc("90 deg to rad"), calc("pi / 2") + " rad");
    assert!(error("sin(5 m)").contains("expects an angle"));
}

#[test]
fn helpers() {
    assert_eq!(calc("splitUnit(1.75 h, [h, min])"), "[1 h, 45 min]");
    assert_eq!(calc("splitUnit(1 mi, [\"ft\", \"inch\"])"), "[5280 ft, 0 inch]");
    assert_eq!(calc("number(5 km, m)"), "5000");
    assert_eq!(calc("number(5 km, \"m\")"), "5000");
    assert_eq!(calc("unit(5, \"cm\") to mm"), "50 mm");
    assert_eq!(calc("unit(\"2 kg\")"), "2 kg");
}

#[test]
fn variables_shadow_units() {
    assert_eq!(calc("m = 3\n2 m"), "6");
    // ...but "to" always means units.
    assert_eq!(calc("m = 3\n2 km to m"), "2000 m");
}

#[test]
fn mismatched_units_are_errors() {
    assert_eq!(error("5 km to s"), "Cannot convert km (length) to s (time)");
    assert_eq!(error("5 m + 2 s"), "Units do not match: m (length) and s (time)");
    assert_eq!(error("5 m + 2"), "Cannot add m and a number without units");
    assert_eq!(error("5 to km"), "Cannot convert a number without units to km");
    assert_eq!(error("5 km to furlongs"), "Unknown unit \"furlongs\"");
    assert!(error("sqrt(2 m)").contains("whole-number units"));
    assert!(error("2 ^ (3 m)").contains("Cannot"));
}

#[test]
fn compound_unit_display() {
    assert_eq!(calc("1 kg m / s^2"), "1 kg m / s^2");
    assert_eq!(calc("3 J / (kg K)"), "3 J / (kg K)");
    assert_eq!(calc("9.81 m/s^2 * 2 s to m/s"), "19.62 m / s");
}
