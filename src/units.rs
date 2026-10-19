//! Physical units: lookup, compound units, conversion and arithmetic.
//!
//! A quantity stores its value in the units it was written in (`5 km` keeps
//! `km`), so results read naturally; conversion goes through SI.

use std::f64::consts::PI;
use std::fmt::Write as _;

use crate::format::{FormatOptions, format_f64};

/// Exponents of the base dimensions, in the order of [`BASE_NAMES`].
pub type Dim = [i32; 9];

pub const BASE_NAMES: [&str; 9] =
    ["length", "mass", "time", "current", "temperature", "amount", "luminosity", "angle", "information"];

const fn d(l: i32, m: i32, t: i32, i: i32, th: i32, n: i32, j: i32, a: i32, b: i32) -> Dim {
    [l, m, t, i, th, n, j, a, b]
}

const NONE_DIM: Dim = d(0, 0, 0, 0, 0, 0, 0, 0, 0);
const LENGTH: Dim = d(1, 0, 0, 0, 0, 0, 0, 0, 0);
const MASS: Dim = d(0, 1, 0, 0, 0, 0, 0, 0, 0);
const TIME: Dim = d(0, 0, 1, 0, 0, 0, 0, 0, 0);
const CURRENT: Dim = d(0, 0, 0, 1, 0, 0, 0, 0, 0);
const TEMPERATURE: Dim = d(0, 0, 0, 0, 1, 0, 0, 0, 0);
const AMOUNT: Dim = d(0, 0, 0, 0, 0, 1, 0, 0, 0);
const LUMINOSITY: Dim = d(0, 0, 0, 0, 0, 0, 1, 0, 0);
const ANGLE: Dim = d(0, 0, 0, 0, 0, 0, 0, 1, 0);
const INFORMATION: Dim = d(0, 0, 0, 0, 0, 0, 0, 0, 1);
const AREA: Dim = d(2, 0, 0, 0, 0, 0, 0, 0, 0);
const VOLUME: Dim = d(3, 0, 0, 0, 0, 0, 0, 0, 0);
const SPEED: Dim = d(1, 0, -1, 0, 0, 0, 0, 0, 0);
const FREQUENCY: Dim = d(0, 0, -1, 0, 0, 0, 0, 0, 0);
const FORCE: Dim = d(1, 1, -2, 0, 0, 0, 0, 0, 0);
const ENERGY: Dim = d(2, 1, -2, 0, 0, 0, 0, 0, 0);
const POWER: Dim = d(2, 1, -3, 0, 0, 0, 0, 0, 0);
const PRESSURE: Dim = d(-1, 1, -2, 0, 0, 0, 0, 0, 0);
const CHARGE: Dim = d(0, 0, 1, 1, 0, 0, 0, 0, 0);
const VOLTAGE: Dim = d(2, 1, -3, -1, 0, 0, 0, 0, 0);
const RESISTANCE: Dim = d(2, 1, -3, -2, 0, 0, 0, 0, 0);
const CAPACITANCE: Dim = d(-2, -1, 4, 2, 0, 0, 0, 0, 0);

/// Which prefixes a unit accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefixes {
    None,
    /// All SI prefixes.
    Si,
    /// SI prefixes of 1000 and above, plus binary prefixes (bytes and bits).
    Data,
}

#[derive(Debug, PartialEq)]
pub struct UnitDef {
    /// Display symbol.
    pub symbol: &'static str,
    /// Other short spellings, matched exactly and with short prefixes.
    pub aliases: &'static [&'static str],
    /// Long names, matched with long prefixes and an optional plural `s`/`es`.
    pub long: &'static [&'static str],
    /// Size in SI units (after adding `offset`).
    pub factor: f64,
    /// For temperatures: SI value = (value + offset) × factor.
    pub offset: f64,
    pub dim: Dim,
    prefixes: Prefixes,
}

macro_rules! unit {
    ($sym:expr, [$($alias:expr),*], [$($long:expr),*], $factor:expr, $dim:expr, $prefixes:ident) => {
        UnitDef { symbol: $sym, aliases: &[$($alias),*], long: &[$($long),*], factor: $factor, offset: 0.0, dim: $dim, prefixes: Prefixes::$prefixes }
    };
}

pub static UNITS: &[UnitDef] = &[
    // Length
    unit!("m", [], ["meter", "metre"], 1.0, LENGTH, Si),
    unit!("inch", [], ["inch"], 0.0254, LENGTH, None),
    unit!("ft", [], ["foot", "feet"], 0.3048, LENGTH, None),
    unit!("yd", [], ["yard"], 0.9144, LENGTH, None),
    unit!("mi", [], ["mile"], 1609.344, LENGTH, None),
    unit!("nmi", [], ["nauticalmile"], 1852.0, LENGTH, None),
    unit!("mil", [], [], 2.54e-5, LENGTH, None),
    unit!("angstrom", [], ["angstrom"], 1e-10, LENGTH, None),
    unit!("au", [], [], 149_597_870_700.0, LENGTH, None),
    unit!("ly", [], ["lightyear"], 9_460_730_472_580_800.0, LENGTH, None),
    unit!("pc", [], ["parsec"], 3.085_677_581_491_367_3e16, LENGTH, None),
    // Mass
    unit!("g", [], ["gram", "gramme"], 0.001, MASS, Si),
    unit!("tonne", [], ["tonne"], 1000.0, MASS, None),
    unit!("ton", [], ["ton"], 907.184_74, MASS, None),
    unit!("lb", ["lbs"], ["pound"], 0.453_592_37, MASS, None),
    unit!("oz", [], ["ounce"], 0.028_349_523_125, MASS, None),
    unit!("stone", [], ["stone"], 6.350_293_18, MASS, None),
    unit!("grain", [], ["grain"], 6.479_891e-5, MASS, None),
    // Time
    unit!("s", ["sec"], ["second"], 1.0, TIME, Si),
    unit!("min", ["mins"], ["minute"], 60.0, TIME, None),
    unit!("h", ["hr", "hrs"], ["hour"], 3600.0, TIME, None),
    unit!("day", [], ["day"], 86_400.0, TIME, None),
    unit!("week", ["wk"], ["week"], 604_800.0, TIME, None),
    unit!("month", [], ["month"], 2_629_800.0, TIME, None),
    unit!("year", ["yr", "yrs"], ["year"], 31_557_600.0, TIME, None),
    unit!("decade", [], ["decade"], 315_576_000.0, TIME, None),
    unit!("century", [], ["century", "centuries"], 3_155_760_000.0, TIME, None),
    // Temperature (offsets set below via TEMPERATURES)
    unit!("K", [], ["kelvin"], 1.0, TEMPERATURE, Si),
    // Electric current, amount, luminosity
    unit!("A", [], ["ampere", "amp"], 1.0, CURRENT, Si),
    unit!("mol", [], ["mole"], 1.0, AMOUNT, Si),
    unit!("cd", [], ["candela"], 1.0, LUMINOSITY, Si),
    // Angle
    unit!("rad", [], ["radian"], 1.0, ANGLE, Si),
    unit!("deg", [], ["degree"], PI / 180.0, ANGLE, None),
    unit!("grad", [], ["gradian"], PI / 200.0, ANGLE, None),
    unit!("turn", [], ["turn", "revolution"], 2.0 * PI, ANGLE, None),
    unit!("arcmin", [], ["arcminute"], PI / 10_800.0, ANGLE, None),
    unit!("arcsec", [], ["arcsecond"], PI / 648_000.0, ANGLE, None),
    // Information
    unit!("bit", [], ["bit"], 1.0, INFORMATION, Data),
    unit!("B", [], ["byte"], 8.0, INFORMATION, Data),
    // Area
    unit!("ha", [], ["hectare"], 10_000.0, AREA, None),
    unit!("acre", [], ["acre"], 4_046.856_422_4, AREA, None),
    // Volume
    unit!("L", ["l"], ["liter", "litre"], 0.001, VOLUME, Si),
    unit!("cc", [], [], 1e-6, VOLUME, None),
    unit!("gal", [], ["gallon"], 0.003_785_411_784, VOLUME, None),
    unit!("qt", [], ["quart"], 0.000_946_352_946, VOLUME, None),
    unit!("pt", [], ["pint"], 0.000_473_176_473, VOLUME, None),
    unit!("cup", [], ["cup"], 0.000_236_588_236_5, VOLUME, None),
    unit!("floz", [], ["fluidounce"], 2.957_352_956_25e-5, VOLUME, None),
    unit!("tbsp", [], ["tablespoon"], 1.478_676_478_125e-5, VOLUME, None),
    unit!("tsp", [], ["teaspoon"], 4.928_921_593_75e-6, VOLUME, None),
    // Speed
    unit!("mph", [], [], 0.447_04, SPEED, None),
    unit!("kph", ["kmh"], [], 1000.0 / 3600.0, SPEED, None),
    unit!("knot", ["kn", "kt"], ["knot"], 1852.0 / 3600.0, SPEED, None),
    // Frequency
    unit!("Hz", [], ["hertz"], 1.0, FREQUENCY, Si),
    unit!("rpm", [], [], 1.0 / 60.0, FREQUENCY, None),
    // Force
    unit!("N", [], ["newton"], 1.0, FORCE, Si),
    unit!("lbf", [], ["poundforce"], 4.448_221_615_260_5, FORCE, None),
    unit!("kgf", [], ["kilogramforce"], 9.806_65, FORCE, None),
    unit!("dyn", [], ["dyne"], 1e-5, FORCE, None),
    // Energy
    unit!("J", [], ["joule"], 1.0, ENERGY, Si),
    unit!("cal", [], ["calorie"], 4.184, ENERGY, Si),
    unit!("Wh", [], [], 3600.0, ENERGY, Si),
    unit!("eV", [], ["electronvolt"], 1.602_176_634e-19, ENERGY, Si),
    unit!("BTU", [], [], 1_055.055_852_62, ENERGY, None),
    unit!("erg", [], ["erg"], 1e-7, ENERGY, None),
    // Power
    unit!("W", [], ["watt"], 1.0, POWER, Si),
    unit!("hp", [], ["horsepower"], 745.699_871_582_270_2, POWER, None),
    // Pressure
    unit!("Pa", [], ["pascal"], 1.0, PRESSURE, Si),
    unit!("bar", [], ["bar"], 100_000.0, PRESSURE, Si),
    unit!("atm", [], ["atmosphere"], 101_325.0, PRESSURE, None),
    unit!("psi", [], [], 6_894.757_293_168_361, PRESSURE, None),
    unit!("mmHg", [], [], 133.322_387_415, PRESSURE, None),
    unit!("torr", [], ["torr"], 101_325.0 / 760.0, PRESSURE, None),
    // Electricity
    unit!("C", [], ["coulomb"], 1.0, CHARGE, Si),
    unit!("Ah", [], [], 3600.0, CHARGE, Si),
    unit!("V", [], ["volt"], 1.0, VOLTAGE, Si),
    unit!("ohm", [], ["ohm"], 1.0, RESISTANCE, Si),
    unit!("F", [], ["farad"], 1.0, CAPACITANCE, Si),
];

pub static TEMPERATURES: &[UnitDef] = &[
    UnitDef {
        symbol: "degC",
        aliases: &["°C"],
        long: &["celsius"],
        factor: 1.0,
        offset: 273.15,
        dim: TEMPERATURE,
        prefixes: Prefixes::None,
    },
    UnitDef {
        symbol: "degF",
        aliases: &["°F"],
        long: &["fahrenheit"],
        factor: 5.0 / 9.0,
        offset: 459.67,
        dim: TEMPERATURE,
        prefixes: Prefixes::None,
    },
    UnitDef {
        symbol: "degR",
        aliases: &[],
        long: &["rankine"],
        factor: 5.0 / 9.0,
        offset: 0.0,
        dim: TEMPERATURE,
        prefixes: Prefixes::None,
    },
];

#[derive(Debug, PartialEq)]
pub struct Prefix {
    pub short: &'static str,
    pub long: &'static str,
    pub factor: f64,
    binary: bool,
}

pub static NO_PREFIX: Prefix = Prefix { short: "", long: "", factor: 1.0, binary: false };

static PREFIXES: &[Prefix] = &[
    Prefix { short: "Q", long: "quetta", factor: 1e30, binary: false },
    Prefix { short: "R", long: "ronna", factor: 1e27, binary: false },
    Prefix { short: "Y", long: "yotta", factor: 1e24, binary: false },
    Prefix { short: "Z", long: "zetta", factor: 1e21, binary: false },
    Prefix { short: "E", long: "exa", factor: 1e18, binary: false },
    Prefix { short: "P", long: "peta", factor: 1e15, binary: false },
    Prefix { short: "T", long: "tera", factor: 1e12, binary: false },
    Prefix { short: "G", long: "giga", factor: 1e9, binary: false },
    Prefix { short: "M", long: "mega", factor: 1e6, binary: false },
    Prefix { short: "k", long: "kilo", factor: 1e3, binary: false },
    Prefix { short: "h", long: "hecto", factor: 1e2, binary: false },
    Prefix { short: "da", long: "deca", factor: 1e1, binary: false },
    Prefix { short: "d", long: "deci", factor: 1e-1, binary: false },
    Prefix { short: "c", long: "centi", factor: 1e-2, binary: false },
    Prefix { short: "m", long: "milli", factor: 1e-3, binary: false },
    Prefix { short: "u", long: "micro", factor: 1e-6, binary: false },
    Prefix { short: "µ", long: "", factor: 1e-6, binary: false },
    Prefix { short: "μ", long: "", factor: 1e-6, binary: false },
    Prefix { short: "n", long: "nano", factor: 1e-9, binary: false },
    Prefix { short: "p", long: "pico", factor: 1e-12, binary: false },
    Prefix { short: "f", long: "femto", factor: 1e-15, binary: false },
    Prefix { short: "a", long: "atto", factor: 1e-18, binary: false },
    Prefix { short: "Ki", long: "kibi", factor: 1024.0, binary: true },
    Prefix { short: "Mi", long: "mebi", factor: 1_048_576.0, binary: true },
    Prefix { short: "Gi", long: "gibi", factor: 1_073_741_824.0, binary: true },
    Prefix { short: "Ti", long: "tebi", factor: 1_099_511_627_776.0, binary: true },
    Prefix { short: "Pi", long: "pebi", factor: 1_125_899_906_842_624.0, binary: true },
    Prefix { short: "Ei", long: "exbi", factor: 1_152_921_504_606_846_976.0, binary: true },
];

impl UnitDef {
    /// "SI prefixes", "data prefixes (k, Ki, ...)" or nothing.
    pub fn prefix_note(&self) -> &'static str {
        match self.prefixes {
            Prefixes::None => "",
            Prefixes::Si => "takes SI prefixes (k, m, µ, ...)",
            Prefixes::Data => "takes k, M, G, ... and Ki, Mi, Gi, ...",
        }
    }
}

fn accepts(def: &UnitDef, p: &Prefix) -> bool {
    match def.prefixes {
        Prefixes::None => false,
        Prefixes::Si => !p.binary,
        Prefixes::Data => p.binary || p.factor >= 1e3,
    }
}

fn all_units() -> impl Iterator<Item = &'static UnitDef> {
    UNITS.iter().chain(TEMPERATURES)
}

fn singular(name: &str) -> impl Iterator<Item = &str> {
    [name.strip_suffix("es"), name.strip_suffix('s')].into_iter().flatten()
}

/// Resolves a unit name such as `km`, `kilometers`, `MiB` or `degC`.
pub fn lookup(name: &str) -> Option<(&'static Prefix, &'static UnitDef)> {
    // Exact symbols and long names win over prefixed forms (`min`, `pt`, `nmi`).
    for def in all_units() {
        if def.symbol == name || def.aliases.contains(&name) || def.long.contains(&name) {
            return Some((&NO_PREFIX, def));
        }
    }
    for base in singular(name) {
        if let Some(def) = all_units().find(|u| u.long.contains(&base)) {
            return Some((&NO_PREFIX, def));
        }
    }
    for p in PREFIXES {
        if let Some(rest) = name.strip_prefix(p.short).filter(|r| !r.is_empty()) {
            if let Some(def) = all_units().find(|u| u.symbol == rest || u.aliases.contains(&rest)) {
                if accepts(def, p) {
                    return Some((p, def));
                }
            }
        }
        if p.long.is_empty() {
            continue;
        }
        if let Some(rest) = name.strip_prefix(p.long).filter(|r| !r.is_empty()) {
            let candidates = std::iter::once(rest).chain(singular(rest));
            for base in candidates {
                if let Some(def) = all_units().find(|u| u.long.contains(&base))
                    && accepts(def, p)
                {
                    return Some((p, def));
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone, PartialEq)]
pub struct Term {
    pub prefix: &'static Prefix,
    pub def: &'static UnitDef,
    pub power: i32,
}

impl Term {
    fn scale(&self) -> f64 {
        self.prefix.factor * self.def.factor
    }

    fn same_unit(&self, other: &Term) -> bool {
        std::ptr::eq(self.def, other.def) && std::ptr::eq(self.prefix, other.prefix)
    }
}

/// A product of unit terms, e.g. `km h^-1`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Unit {
    pub terms: Vec<Term>,
}

impl Unit {
    pub fn single(prefix: &'static Prefix, def: &'static UnitDef) -> Self {
        Self { terms: vec![Term { prefix, def, power: 1 }] }
    }

    pub fn dim(&self) -> Dim {
        let mut dim = NONE_DIM;
        for t in &self.terms {
            for (d, base) in dim.iter_mut().zip(t.def.dim) {
                *d += base * t.power;
            }
        }
        dim
    }

    /// SI size of one of this unit, ignoring temperature offsets.
    pub fn scale(&self) -> f64 {
        self.terms.iter().map(|t| t.scale().powi(t.power)).product()
    }

    /// The temperature offset, if this is a lone offset unit like `degC`.
    fn offset(&self) -> Option<f64> {
        match self.terms.as_slice() {
            [t] if t.power == 1 && t.def.offset != 0.0 => Some(t.def.offset),
            _ => None,
        }
    }

    fn to_si(&self, value: f64) -> f64 {
        match self.offset() {
            Some(off) => (value + off) * self.scale(),
            None => value * self.scale(),
        }
    }

    fn from_si(&self, si: f64) -> f64 {
        match self.offset() {
            Some(off) => si / self.scale() - off,
            None => si / self.scale(),
        }
    }

    pub fn is_angle(&self) -> bool {
        self.dim() == ANGLE
    }

    pub fn is_dimensionless(&self) -> bool {
        self.dim() == NONE_DIM
    }

    pub fn powi(&self, n: i32) -> Unit {
        Unit {
            terms: self
                .terms
                .iter()
                .map(|t| Term { power: t.power * n, ..t.clone() })
                .filter(|t| t.power != 0)
                .collect(),
        }
    }

    /// Raises to a fractional power such as ½, if every exponent stays whole.
    pub fn powf(&self, p: f64) -> Option<Unit> {
        let mut terms = Vec::new();
        for t in &self.terms {
            let power = t.power as f64 * p;
            if (power - power.round()).abs() > 1e-9 {
                return None;
            }
            if power.round() != 0.0 {
                terms.push(Term { power: power.round() as i32, ..t.clone() });
            }
        }
        Some(Unit { terms })
    }
}

impl std::fmt::Display for Unit {
    /// `m`, `km / h`, `kg m / s^2`, `m^2`, `s^-1`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let term = |t: &Term, power: i32| {
            let mut s = format!("{}{}", t.prefix.short, t.def.symbol);
            if power != 1 {
                let _ = write!(s, "^{power}");
            }
            s
        };
        let num: Vec<String> = self.terms.iter().filter(|t| t.power > 0).map(|t| term(t, t.power)).collect();
        let den: Vec<String> = self.terms.iter().filter(|t| t.power < 0).map(|t| term(t, -t.power)).collect();
        match (num.is_empty(), den.len()) {
            (_, 0) => f.write_str(&num.join(" ")),
            (true, _) => {
                let neg: Vec<String> = self.terms.iter().map(|t| term(t, t.power)).collect();
                f.write_str(&neg.join(" "))
            }
            (false, 1) => write!(f, "{} / {}", num.join(" "), den[0]),
            (false, _) => write!(f, "{} / ({})", num.join(" "), den.join(" ")),
        }
    }
}

/// Human description of a dimension for error messages, e.g. `length / time`.
pub fn describe_dim(dim: &Dim) -> String {
    if *dim == NONE_DIM {
        return "no units".into();
    }
    let part = |sign: i32| {
        BASE_NAMES
            .iter()
            .zip(dim)
            .filter(|(_, p)| p.signum() == sign)
            .map(|(n, p)| if p.abs() == 1 { n.to_string() } else { format!("{n}^{}", p.abs()) })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let (num, den) = (part(1), part(-1));
    match (num.is_empty(), den.is_empty()) {
        (_, true) => num,
        (true, false) => format!("1 / {den}"),
        (false, false) => format!("{num} / {den}"),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Quantity {
    pub value: f64,
    pub unit: Unit,
}

/// The result of combining units: either still a quantity or a plain number.
pub enum Combined {
    Quantity(Quantity),
    Number(f64),
}

impl Quantity {
    pub fn format(&self, opts: FormatOptions) -> String {
        format!("{} {}", format_f64(self.value, opts), self.unit)
    }

    pub fn to_si(&self) -> f64 {
        self.unit.to_si(self.value)
    }

    /// Converts to another unit of the same dimension.
    pub fn convert(&self, target: &Unit) -> Result<Quantity, String> {
        if self.unit.dim() != target.dim() {
            return Err(format!(
                "Cannot convert {} ({}) to {} ({})",
                self.unit,
                describe_dim(&self.unit.dim()),
                target,
                describe_dim(&target.dim())
            ));
        }
        Ok(Quantity { value: target.from_si(self.to_si()), unit: target.clone() })
    }

    /// The other quantity's value expressed in this quantity's unit.
    pub fn value_of(&self, other: &Quantity) -> Result<f64, String> {
        if self.unit.dim() != other.unit.dim() {
            return Err(format!(
                "Units do not match: {} ({}) and {} ({})",
                self.unit,
                describe_dim(&self.unit.dim()),
                other.unit,
                describe_dim(&other.unit.dim())
            ));
        }
        Ok(other.convert(&self.unit)?.value)
    }

    /// Multiplies (`sign` = 1) or divides (`sign` = -1) two quantities, merging
    /// terms of the same kind (`km * m` becomes `km^2`).
    pub fn combine(&self, other: &Quantity, sign: i32) -> Combined {
        let mut value = if sign > 0 { self.value * other.value } else { self.value / other.value };
        let mut terms = self.unit.terms.clone();
        for t in &other.unit.terms {
            let power = t.power * sign;
            if let Some(existing) = terms.iter_mut().find(|e| e.same_unit(t)) {
                existing.power += power;
            } else if let Some(existing) = terms.iter_mut().find(|e| e.def.dim == t.def.dim && e.def.offset == 0.0 && t.def.offset == 0.0) {
                value *= (t.scale() / existing.scale()).powi(power);
                existing.power += power;
            } else {
                terms.push(Term { power, ..t.clone() });
            }
        }
        terms.retain(|t| t.power != 0);
        let unit = Unit { terms };
        if unit.is_dimensionless() {
            Combined::Number(value * unit.scale())
        } else {
            Combined::Quantity(Quantity { value, unit })
        }
    }
}
