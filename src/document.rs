//! Turns a whole document into results.
//!
//! `parse_expressions` ports the original `expression-parser.js`: one expression
//! per line, except that lines are joined while brackets are unbalanced, so a
//! function call can span several lines. `calculate` evaluates them top to bottom.

use crate::big;
use crate::eval::{self, Scope};
use crate::format::FormatOptions;

#[derive(Debug, Clone, PartialEq)]
pub struct Expression {
    pub expression: String,
    pub start_line: usize,
    pub end_line: usize,
}

impl Expression {
    fn new(expression: impl Into<String>, start_line: usize, end_line: usize) -> Self {
        Self { expression: expression.into(), start_line, end_line }
    }

    pub fn is_empty(&self) -> bool {
        self.expression.is_empty()
    }

    pub fn is_comment(&self) -> bool {
        self.expression.trim_start().starts_with("//")
    }
}

/// Removes a trailing `// comment`, ignoring `//` inside quoted text.
pub fn strip_inline_comment(line: &str) -> &str {
    let mut quote = None;
    let mut prev_slash = false;
    for (i, c) in line.char_indices() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c == '/' && prev_slash => return line[..i - 1].trim_end(),
            None => {}
        }
        prev_slash = c == '/' && quote.is_none();
    }
    line
}

pub fn parse_expressions(input: &str) -> Vec<Expression> {
    let lines: Vec<&str> = input.split('\n').collect();
    let mut expressions = Vec::new();
    parse_lines(&lines, 0, &mut expressions);
    expressions
}

fn parse_lines(lines: &[&str], first: usize, expressions: &mut Vec<Expression>) {
    let mut current = String::new();
    let mut open_brackets: i32 = 0;
    let mut start_line = first;

    for i in first..lines.len() {
        let trimmed = lines[i].trim();

        if open_brackets == 0 && (trimmed.is_empty() || trimmed.starts_with("//")) {
            expressions.push(Expression::new(trimmed, i, i));
            continue;
        }

        let code = strip_inline_comment(trimmed);
        if current.is_empty() {
            start_line = i;
            current = code.to_string();
        } else {
            current.push(' ');
            current.push_str(code);
        }

        // The original only balanced `()`; `[]` is included so matrices can span lines.
        for c in code.chars() {
            match c {
                '(' | '[' => open_brackets += 1,
                ')' | ']' => open_brackets -= 1,
                _ => {}
            }
        }

        if open_brackets <= 0 {
            // An extra closing bracket ends the expression; the evaluator reports it.
            open_brackets = 0;
            if !current.trim().is_empty() {
                expressions.push(Expression::new(current.trim(), start_line, i));
            }
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        // A bracket that is never closed: rather than swallowing the rest of the
        // document (as the original did), report that line alone and carry on.
        let line = strip_inline_comment(lines[start_line].trim());
        expressions.push(Expression::new(line, start_line, start_line));
        parse_lines(lines, start_line + 1, expressions);
    }
}

/// Display and calculation settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Settings {
    pub format: FormatOptions,
    /// Significant digits for high-precision arithmetic, or `None` for ordinary numbers.
    pub precision: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Comments, blank lines and messages.
    Info,
    /// `expression = value`.
    Result,
    /// `name = value`.
    Assignment,
    /// `expression → Error: message`.
    Error,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    pub kind: Kind,
    /// The full line as the original showed it, e.g. `x + 1 = 6`. Continuation
    /// lines of multi-line results are indented by two spaces.
    pub text: String,
    /// Just the formatted result (or the error message), for display beside the line.
    pub value: Option<String>,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub outputs: Vec<Output>,
    /// Names assigned in the document, in order of first assignment.
    pub variables: Vec<String>,
}

pub const EMPTY_MESSAGE: &str = "Enter math expressions above to see results here.";

/// Evaluates every expression in `input` top to bottom with a fresh scope.
/// `ans` is the previous result and `lineN` the result on line N.
pub fn calculate(input: &str, settings: &Settings) -> Document {
    // Deeply nested or recursive expressions need more stack than a default
    // thread has (2 MB for spawned threads); the memory is only touched if used.
    const STACK: usize = 64 * 1024 * 1024;
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(s, || big::with_precision(settings.precision, || calculate_inner(input, settings)))
            .map(|h| h.join().unwrap_or_else(|_| crash_report(input)))
            .unwrap_or_else(|_| big::with_precision(settings.precision, || calculate_inner(input, settings)))
    })
}

/// Shown if evaluation panics, which would be a bug: the document stays usable.
fn crash_report(input: &str) -> Document {
    let last = input.lines().count().saturating_sub(1);
    Document {
        outputs: vec![Output {
            kind: Kind::Error,
            text: "Internal error while calculating; please report this document.".into(),
            value: Some("Internal error while calculating".into()),
            start_line: 0,
            end_line: last,
        }],
        variables: Vec::new(),
    }
}

fn calculate_inner(input: &str, settings: &Settings) -> Document {
    let mut doc = Document::default();
    let trimmed = input.trim();
    if trimmed.is_empty() {
        doc.outputs.push(Output { kind: Kind::Info, text: EMPTY_MESSAGE.into(), value: None, start_line: 0, end_line: 0 });
        return doc;
    }
    // The original trims the input first; keep line numbers relative to the real text.
    let offset = input[..input.find(trimmed).unwrap_or(0)].matches('\n').count();

    let mut scope = Scope::new();
    for expr in parse_expressions(trimmed) {
        let (start_line, end_line) = (expr.start_line + offset, expr.end_line + offset);
        let output = |kind, text: String, value| Output { kind, text, value, start_line, end_line };
        if expr.is_empty() || expr.is_comment() {
            doc.outputs.push(output(Kind::Info, expr.expression.clone(), None));
            continue;
        }
        match eval::evaluate(&expr.expression, &mut scope) {
            Ok(r) => {
                let value = r.value.format(settings.format);
                let (kind, text) = match &r.assigned {
                    Some(name) => {
                        let var = name.split('(').next().unwrap_or(name).to_string();
                        if !doc.variables.contains(&var) {
                            doc.variables.push(var);
                        }
                        (Kind::Assignment, format!("{name} = {value}"))
                    }
                    None => (Kind::Result, format!("{} = {value}", expr.expression)),
                };
                // A function definition has no value worth showing beside the line.
                let shown = (!matches!(r.value, crate::value::Value::Func(_))).then_some(value);
                scope.insert("ans".into(), r.value.clone());
                scope.insert(format!("line{}", start_line + 1), r.value);
                doc.outputs.push(output(kind, text.replace('\n', "\n  "), shown));
            }
            Err(e) => doc.outputs.push(output(Kind::Error, format!("{} → Error: {e}", expr.expression), Some(e))),
        }
    }
    doc
}
