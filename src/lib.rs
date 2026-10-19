//! Math in plain text: a calculator where every line is an expression.
//!
//! Started as a Rust rewrite of calctxt (<https://calctxt.com>) and extended with
//! units, complex numbers, high precision and around 200 functions.

pub mod big;
pub mod catalog;
pub mod document;
pub mod eval;
pub mod format;
pub mod functions;
pub mod highlight;
pub mod parser;
pub mod units;
pub mod value;

pub use document::{Document, Kind, Output, Settings, calculate, parse_expressions};
pub use format::FormatOptions;

/// The example document shown on first launch.
pub const WELCOME: &str = include_str!("welcome.txt");
