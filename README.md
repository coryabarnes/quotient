# Quotient

Math in plain text. Type calculations on the left, one per line; each result
appears beside its line as you type.

Quotient started as a Rust rewrite of [calctxt](https://calctxt.com) by Jason
Lotito and goes well beyond it: units, complex numbers, high precision, 196
functions, and an editor with highlighting and autocomplete.

```
price = 1299                          1299
tax = 8% of price                     103.92
price + tax                           1402.92
60 mph * 2.5 h to km                  241.4016 km
72 °F to °C                           22.222222222222 degC
sqrt(-16)                             4i
pmt(6.5% / 12, 360, 300000)           -1896.2040704789
```

## Running it

```sh
cargo run --release                         # the app
cargo run --release -- notes.txt            # open a file
cargo run --release -- -c "5 km to mi"      # calculate from the command line
cargo run --release -- --eval notes.txt     # print a file's results (- for stdin)
cargo test                                  # 84 tests, including GUI tests
```

Command-line options: `--digits N|full`, `--precision N` (high precision with N
digits), `--thousands`. The exit status is 1 if any line has an error.

## What you can write

| | |
|---|---|
| `x = 5`, `f(x) = x^2 + 1` | Variables and your own functions (recursion works) |
| `ans`, `line3` | The previous result, the result on line 3 |
| `2pi`, `3(x + 1)`, `5 km` | Implicit multiplication |
| `20% of 150`, `80 + 15%`, `25% off 80`, `5% on 200` | Percentages that mean what they say |
| `5 km to mi`, `100 km/h in mph` | Unit conversion: 78 units, SI and binary prefixes, plurals and long names |
| `sqrt(-4)`, `(1 + 2i) * i`, `e^(i pi)` | Complex numbers |
| `255 to hex`, `0.75 to fraction`, `x to sci`, `x to percent` | Display formats |
| `[1, 2; 3, 4]`, `1:10`, `m[2, 1]` | Matrices, ranges and indexing |
| `x > 0 ? 1 : -1`, `and`, `or`, `not`, `xor` | Conditions |
| `0xff`, `0b101`, `1_000_000`, `×`, `÷`, `π`, `√`, `²` | Literals and pasted symbols |
| `// note` | Comments, on their own line or after a calculation |

Open brackets continue onto the next line, so long function calls can span
lines. The full function list is in the app (F1) and in `src/catalog.rs`.

## The app

- Results line up with the lines that produced them; click one to copy it.
- Syntax highlighting, and autocomplete for functions and your variables (Tab).
- View menu: light/dark theme, digits shown (14 by default, so `0.1 + 0.2` shows
  `0.3`), thousands separators, and high precision (30, 50 or 100 digits).
- Untitled work is kept between sessions; saved files auto-save as you type.
- Searchable help with every function and unit.
- Export or copy all results as text; drag a file onto the window to open it.

## Correctness

Results are checked against independent references: Python's `math`, `cmath`,
`statistics` and `decimal` modules (50-digit comparisons for high precision),
Microsoft's Excel documentation for the finance functions, and NIST/SI unit
definitions. Some deliberate choices:

- Equality uses a relative tolerance of 1e-12, like mathjs, so `0.1 + 0.2 == 0.3`.
- `floor`, `ceil` and `fix` snap to the nearest integer when within rounding error,
  and `sin(pi)` is exactly 0 (the residue is smaller than pi's own rounding).
- `round` and the display round half away from zero on the decimal digits, so
  `round(1.005, 2)` is 1.01 as written.
- Complex results for real inputs outside a function's domain follow mathjs.
- `std` is the sample standard deviation; `variance` and `stddev` are population
  (as in calctxt). Add `"unbiased"` for the sample variance.
- Finance functions follow Excel's sign conventions.

## How it is built

| File | |
|---|---|
| `src/parser.rs` | Tokenizer and recursive-descent parser (precedence table at the top) |
| `src/eval.rs` | Evaluator: operators across numbers, complex, high precision, units and matrices |
| `src/units.rs` | Unit table, prefixes, compound units, conversion |
| `src/big.rs` | High-precision numbers on `astro-float` |
| `src/functions/` | The built-in functions, by area |
| `src/catalog.rs` | Every function's signature and description |
| `src/document.rs` | Splits a document into expressions and evaluates them in order |
| `src/highlight.rs` | Highlighting and autocomplete |
| `src/gui.rs` | The desktop app (`eframe`/`egui`) and its tests |

## Differences from calctxt

calctxt (v0.3.3) is an Electron app wrapping mathjs. Quotient keeps its document
format (any calctxt file opens) and its custom functions (`trend`, `sma`, `cagr`,
...), and changes:

- Errors from functions like `geomean(-1)` are real errors instead of text results.
- Each recalculation starts fresh, so deleted variables don't linger.
- An unclosed bracket affects only its own line instead of swallowing the rest.
- `toRadians`, `ln` and trailing `// comments`, which calctxt documented but
  failed on, work.
- mathjs features not included: BigNumber fractions, symbolic algebra
  (`simplify`, `derivative`), sparse matrices, `compile`/`parse`/`chain`, and
  matrix decompositions returned as objects (`lup`, `slu`, `qr`).

## License

Copyright © 2026 Cory Barnes. Released under the MIT License, the same license as
calctxt; portions are ported from calctxt, © Jason Lotito. See [LICENSE](LICENSE).
