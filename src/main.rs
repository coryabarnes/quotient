use std::io::{IsTerminal, Read};
use std::process::ExitCode;

use quotient::{Kind, Settings, calculate};

#[cfg(feature = "gui")]
mod gui;

const USAGE: &str = "\
Quotient - math in plain text

Usage:
  quotient [FILE]              Open the app (optionally with FILE)
  quotient --eval FILE         Print the results for FILE (- reads stdin)
  quotient -c TEXT             Print the results for TEXT, e.g. -c \"5 km to mi\"

Options for --eval and -c:
  --digits N|full              Significant digits to show (default 14)
  --precision N                High-precision arithmetic with N digits
  --thousands                  Group digits: 1,234,567

Exits with status 1 if any line has an error.
";

enum Input {
    File(String),
    Text(String),
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("quotient: {message}\n\n{USAGE}");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut settings = Settings::default();
    let mut input = None;
    let mut path = None;
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            "-e" | "--eval" => match it.next() {
                Some(f) => input = Some(Input::File(f)),
                None => return usage_error("--eval needs a file"),
            },
            "-c" => match it.next() {
                Some(t) => input = Some(Input::Text(t)),
                None => return usage_error("-c needs text to calculate"),
            },
            "--digits" => match it.next().as_deref() {
                Some("full") => settings.format.digits = None,
                Some(n) => match n.parse::<usize>() {
                    Ok(d) if (1..=17).contains(&d) => settings.format.digits = Some(d),
                    _ => return usage_error("--digits must be 1 to 17 or \"full\""),
                },
                None => return usage_error("--digits needs a value"),
            },
            "--precision" => match it.next().map(|n| n.parse::<usize>()) {
                Some(Ok(d)) if (2..=1000).contains(&d) => settings.precision = Some(d),
                _ => return usage_error("--precision must be a number from 2 to 1000"),
            },
            "--thousands" => settings.format.thousands = true,
            flag if flag.starts_with('-') && flag != "-" => return usage_error(&format!("unknown option {flag}")),
            file => {
                if path.is_some() {
                    return usage_error("only one file can be opened");
                }
                path = Some(file.to_string());
            }
        }
    }
    match input {
        Some(input) => print_results(input, &settings),
        #[cfg(feature = "gui")]
        None => run_gui(path.map(Into::into)),
        #[cfg(not(feature = "gui"))]
        None => match path {
            Some(file) => print_results(Input::File(file), &settings),
            None => usage_error("this build has no app; use --eval or -c"),
        },
    }
}

#[cfg(feature = "gui")]
fn run_gui(path: Option<std::path::PathBuf>) -> ExitCode {
    match gui::run(path) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("quotient: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Prints each result line, coloured when writing to a terminal.
fn print_results(input: Input, settings: &Settings) -> ExitCode {
    let text = match input {
        Input::Text(t) => t,
        Input::File(file) => {
            let read = if file == "-" {
                let mut s = String::new();
                std::io::stdin().read_to_string(&mut s).map(|_| s)
            } else {
                std::fs::read_to_string(&file)
            };
            match read {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("quotient: {file}: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
    };
    let color = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none();
    let mut failed = false;
    for out in calculate(&text, settings).outputs {
        failed |= out.kind == Kind::Error;
        let code = match out.kind {
            Kind::Info => "90",
            Kind::Result => "32",
            Kind::Assignment => "35",
            Kind::Error => "31",
        };
        if color {
            println!("\x1b[{code}m{}\x1b[0m", out.text);
        } else {
            println!("{}", out.text);
        }
    }
    if failed { ExitCode::FAILURE } else { ExitCode::SUCCESS }
}
