//! Tokenizer and recursive-descent parser for a single expression.
//!
//! Precedence, lowest first (following mathjs where it has an equivalent):
//!
//! | level        | syntax                                   |
//! |--------------|------------------------------------------|
//! | ternary      | `c ? a : b`                              |
//! | or, xor, and | `a or b`, `a xor b`, `a and b`           |
//! | comparison   | `==  !=  <  <=  >  >=`                   |
//! | conversion   | `5 km to mi`, `x in hex`                 |
//! | range        | `1:10`, `0:0.5:2`                        |
//! | additive     | `+  -` (`100 + 10%` adds ten percent)    |
//! | percent      | `20% of 150`, `15% off 80`, `5% on 200`  |
//! | multiply     | `*  /  %  mod`                           |
//! | implicit     | `2pi`, `3(x + 1)`, `5 km`                |
//! | unary        | `-x  +x  not x  √x`                      |
//! | power        | `^` (right-associative)                  |
//! | postfix      | `5!`, `50%`, `m[2]`, `m[1, 2]`           |

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Xor,
}

impl BinOp {
    pub fn symbol(self) -> &'static str {
        match self {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Mod => "mod",
            BinOp::Pow => "^",
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Le => "<=",
            BinOp::Gt => ">",
            BinOp::Ge => ">=",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Xor => "xor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PercentOp {
    /// `20% of 150` = 30
    Of,
    /// `15% off 80` = 68
    Off,
    /// `5% on 200` = 210
    On,
}

#[derive(Debug, Clone)]
pub enum Expr {
    /// A number literal; `text` keeps the digits for exact high-precision parsing.
    Num { value: f64, text: String },
    Str(String),
    Var(String),
    Matrix(Vec<Expr>),
    /// `start:end` or `start:step:end` (inclusive).
    Range(Box<Expr>, Option<Box<Expr>>, Box<Expr>),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Factorial(Box<Expr>),
    Percent(Box<Expr>),
    PercentOf(PercentOp, Box<Expr>, Box<Expr>),
    Index(Box<Expr>, Vec<Expr>),
    Call(String, Vec<Expr>),
    /// `value to target`, where the target is a unit or a format (`hex`, `fraction`, ...).
    Convert(Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expr(Expr),
    Assign(String, Expr),
    FuncDef { name: String, params: Vec<String>, body: Expr, body_src: String },
}

/// Words with a meaning in the grammar; they cannot be used as names.
pub const KEYWORDS: &[&str] = &["to", "in", "of", "off", "on", "mod", "and", "or", "xor", "not"];

pub fn is_keyword(word: &str) -> bool {
    KEYWORDS.contains(&word)
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64, String),
    Str(String),
    Ident(String),
    Op(&'static str),
}

#[derive(Debug, Clone)]
struct Token {
    tok: Tok,
    /// 1-based character position, used in error messages like mathjs does.
    pos: usize,
}

const OPERATORS: &[&str] = &[
    "==", "!=", "<=", ">=", "+", "-", "*", "/", "^", "%", "!", "(", ")", "[", "]", ",", ";", "=", "<",
    ">", "?", ":", "√",
];

/// Nesting deeper than this is rejected instead of risking a stack overflow.
const MAX_DEPTH: usize = 200;

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn tokenize(src: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        let pos = start + 1;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        // 0x1F, 0b101, 0o17
        if c == '0' && matches!(chars.get(i + 1), Some('x' | 'b' | 'o')) {
            let radix = match chars[i + 1] {
                'x' => 16,
                'b' => 2,
                _ => 8,
            };
            let mut j = i + 2;
            while j < chars.len() && (chars[j].is_digit(radix) || chars[j] == '_') {
                j += 1;
            }
            let digits: String = chars[i + 2..j].iter().filter(|c| **c != '_').collect();
            if !digits.is_empty() && !chars.get(j).is_some_and(|c| is_ident_char(*c)) {
                let n = u64::from_str_radix(&digits, radix)
                    .map_err(|_| format!("Number too large (char {pos})"))?;
                toks.push(Token { tok: Tok::Num(n as f64, n.to_string()), pos });
                i = j;
                continue;
            }
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(|d| d.is_ascii_digit())) {
            let digits_from = |mut j: usize| {
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '_') {
                    j += 1;
                }
                j
            };
            i = digits_from(i);
            if i < chars.len() && chars[i] == '.' {
                i = digits_from(i + 1);
            }
            // Only treat `e` as an exponent when digits follow, so `2e` stays `2 * e`.
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = digits_from(j);
                }
            }
            let text: String = chars[start..i].iter().filter(|c| **c != '_').collect();
            let n = text.parse::<f64>().map_err(|_| format!("Invalid number \"{text}\" (char {pos})"))?;
            toks.push(Token { tok: Tok::Num(n, text), pos });
            continue;
        }
        if c == '"' || c == '\'' {
            let mut j = i + 1;
            let mut s = String::new();
            while j < chars.len() && chars[j] != c {
                s.push(chars[j]);
                j += 1;
            }
            if j >= chars.len() {
                return Err(format!("Missing closing quote (char {pos})"));
            }
            toks.push(Token { tok: Tok::Str(s), pos });
            i = j + 1;
            continue;
        }
        // °C, °F and bare ° (degrees)
        if c == '°' {
            let name = match chars.get(i + 1) {
                Some('C') if !chars.get(i + 2).is_some_and(|c| is_ident_char(*c)) => "degC",
                Some('F') if !chars.get(i + 2).is_some_and(|c| is_ident_char(*c)) => "degF",
                _ => "deg",
            };
            i += if name == "deg" { 1 } else { 2 };
            toks.push(Token { tok: Tok::Ident(name.into()), pos });
            continue;
        }
        if c == 'π' {
            toks.push(Token { tok: Tok::Ident("pi".into()), pos });
            i += 1;
            continue;
        }
        if is_ident_start(c) {
            while i < chars.len() && is_ident_char(chars[i]) {
                i += 1;
            }
            let name: String = chars[start..i].iter().collect();
            toks.push(Token { tok: Tok::Ident(name), pos });
            continue;
        }
        // Typographic operators that show up in pasted text.
        let typographic = match c {
            '×' | '·' | '⋅' => Some("*"),
            '÷' => Some("/"),
            '−' | '–' => Some("-"),
            '≠' => Some("!="),
            '≤' => Some("<="),
            '≥' => Some(">="),
            _ => None,
        };
        if let Some(op) = typographic {
            toks.push(Token { tok: Tok::Op(op), pos });
            i += 1;
            continue;
        }
        if let Some(power) = "⁰¹²³⁴⁵⁶⁷⁸⁹".chars().position(|s| s == c) {
            toks.push(Token { tok: Tok::Op("^"), pos });
            toks.push(Token { tok: Tok::Num(power as f64, power.to_string()), pos });
            i += 1;
            continue;
        }
        let rest: String = chars[i..chars.len().min(i + 2)].iter().collect();
        match OPERATORS.iter().find(|op| rest.starts_with(**op)) {
            Some(op) => {
                toks.push(Token { tok: Tok::Op(op), pos });
                i += op.chars().count();
            }
            None => return Err(format!("Unexpected character \"{c}\" (char {pos})")),
        }
    }
    Ok(toks)
}

pub fn parse(src: &str) -> Result<Stmt, String> {
    let toks = tokenize(src)?;
    let end_pos = src.chars().count() + 1;
    let mut p = Parser { toks, i: 0, end_pos, depth: 0, no_range: 0 };
    let stmt = p.statement(src)?;
    if let Some(t) = p.peek_token() {
        return Err(format!("Unexpected {} (char {})", describe(&t.tok), t.pos));
    }
    Ok(stmt)
}

fn describe(tok: &Tok) -> String {
    match tok {
        Tok::Num(_, text) => format!("number {text}"),
        Tok::Str(s) => format!("text \"{s}\""),
        Tok::Ident(s) if is_keyword(s) => format!("keyword \"{s}\""),
        Tok::Ident(s) => format!("symbol \"{s}\""),
        Tok::Op(o) => format!("operator {o}"),
    }
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    end_pos: usize,
    depth: usize,
    /// Inside `a ? b : c`, a `:` ends the branch instead of making a range.
    no_range: usize,
}

impl Parser {
    fn peek_token(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }

    fn peek(&self) -> Option<&Tok> {
        self.peek_at(0)
    }

    fn peek_at(&self, offset: usize) -> Option<&Tok> {
        self.toks.get(self.i + offset).map(|t| &t.tok)
    }

    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Some(Tok::Op(o)) if *o == op)
    }

    fn is_word(&self, word: &str) -> bool {
        matches!(self.peek(), Some(Tok::Ident(s)) if s == word)
    }

    fn eat_op(&mut self, op: &str) -> bool {
        let found = self.is_op(op);
        if found {
            self.i += 1;
        }
        found
    }

    fn eat_word(&mut self, word: &str) -> bool {
        let found = self.is_word(word);
        if found {
            self.i += 1;
        }
        found
    }

    fn pos(&self) -> usize {
        self.peek_token().map_or(self.end_pos, |t| t.pos)
    }

    fn expect_op(&mut self, op: &str) -> Result<(), String> {
        if self.eat_op(op) {
            Ok(())
        } else {
            Err(format!("Parenthesis {op} expected (char {})", self.pos()))
        }
    }

    /// True when the token can begin an operand; used to tell `a % b` from `50%`
    /// and to detect implicit multiplication.
    fn starts_operand(&self, offset: usize) -> bool {
        match self.peek_at(offset) {
            Some(Tok::Num(..) | Tok::Str(_)) => true,
            Some(Tok::Ident(s)) => !is_keyword(s),
            Some(Tok::Op(o)) => *o == "(" || *o == "[" || *o == "√",
            None => false,
        }
    }

    fn nested<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T, String>) -> Result<T, String> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err("Expression is nested too deeply".into());
        }
        let result = f(self);
        self.depth -= 1;
        result
    }

    fn statement(&mut self, src: &str) -> Result<Stmt, String> {
        if let Some(Tok::Ident(name)) = self.peek().cloned() {
            if self.peek_at(1) == Some(&Tok::Op("=")) {
                if is_keyword(&name) {
                    return Err(format!("\"{name}\" is a reserved word and cannot be assigned"));
                }
                self.i += 2;
                return Ok(Stmt::Assign(name, self.expr()?));
            }
            if let Some(params) = self.function_signature() {
                if is_keyword(&name) {
                    return Err(format!("\"{name}\" is a reserved word and cannot be assigned"));
                }
                let body = self.expr()?;
                let body_src = src.split_once('=').map_or("", |(_, b)| b).trim().to_string();
                return Ok(Stmt::FuncDef { name, params, body, body_src });
            }
        }
        Ok(Stmt::Expr(self.expr()?))
    }

    /// Matches `name(a, b, ...) =` and consumes it, or leaves the position untouched.
    fn function_signature(&mut self) -> Option<Vec<String>> {
        let start = self.i;
        self.i += 1;
        let mut params = Vec::new();
        let mut matched = self.eat_op("(");
        if matched && !self.eat_op(")") {
            loop {
                match self.peek().cloned() {
                    Some(Tok::Ident(p)) if !is_keyword(&p) => {
                        params.push(p);
                        self.i += 1;
                    }
                    _ => {
                        matched = false;
                        break;
                    }
                }
                if self.eat_op(")") {
                    break;
                }
                if !self.eat_op(",") {
                    matched = false;
                    break;
                }
            }
        }
        if matched && self.eat_op("=") {
            Some(params)
        } else {
            self.i = start;
            None
        }
    }

    pub fn expr(&mut self) -> Result<Expr, String> {
        self.nested(Self::ternary)
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let cond = self.or()?;
        if !self.eat_op("?") {
            return Ok(cond);
        }
        self.no_range += 1;
        let yes = self.nested(Self::ternary);
        self.no_range -= 1;
        let yes = yes?;
        if !self.eat_op(":") {
            return Err(format!("False part of condition expected, \":\" missing (char {})", self.pos()));
        }
        let no = self.nested(Self::ternary)?;
        Ok(Expr::Ternary(Box::new(cond), Box::new(yes), Box::new(no)))
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut lhs = self.xor()?;
        while self.eat_word("or") {
            lhs = Expr::Binary(BinOp::Or, Box::new(lhs), Box::new(self.xor()?));
        }
        Ok(lhs)
    }

    fn xor(&mut self) -> Result<Expr, String> {
        let mut lhs = self.and()?;
        while self.eat_word("xor") {
            lhs = Expr::Binary(BinOp::Xor, Box::new(lhs), Box::new(self.and()?));
        }
        Ok(lhs)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut lhs = self.comparison()?;
        while self.eat_word("and") {
            lhs = Expr::Binary(BinOp::And, Box::new(lhs), Box::new(self.comparison()?));
        }
        Ok(lhs)
    }

    fn comparison(&mut self) -> Result<Expr, String> {
        let mut lhs = self.conversion()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("==")) => BinOp::Eq,
                Some(Tok::Op("!=")) => BinOp::Ne,
                Some(Tok::Op("<")) => BinOp::Lt,
                Some(Tok::Op("<=")) => BinOp::Le,
                Some(Tok::Op(">")) => BinOp::Gt,
                Some(Tok::Op(">=")) => BinOp::Ge,
                _ => return Ok(lhs),
            };
            self.i += 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.conversion()?));
        }
    }

    fn conversion(&mut self) -> Result<Expr, String> {
        let mut lhs = self.range()?;
        while self.eat_word("to") || self.eat_word("in") {
            if !self.starts_operand(0) {
                return Err(format!("Unit or format expected after \"to\" (char {})", self.pos()));
            }
            lhs = Expr::Convert(Box::new(lhs), Box::new(self.range()?));
        }
        Ok(lhs)
    }

    fn range(&mut self) -> Result<Expr, String> {
        let start = self.additive()?;
        if self.no_range > 0 || !self.eat_op(":") {
            return Ok(start);
        }
        let second = self.additive()?;
        if self.eat_op(":") {
            let end = self.additive()?;
            return Ok(Expr::Range(Box::new(start), Some(Box::new(second)), Box::new(end)));
        }
        Ok(Expr::Range(Box::new(start), None, Box::new(second)))
    }

    fn additive(&mut self) -> Result<Expr, String> {
        let mut lhs = self.percent_of()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("+")) => BinOp::Add,
                Some(Tok::Op("-")) => BinOp::Sub,
                _ => return Ok(lhs),
            };
            self.i += 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.percent_of()?));
        }
    }

    fn percent_of(&mut self) -> Result<Expr, String> {
        let lhs = self.multiplicative()?;
        let op = if self.eat_word("of") {
            PercentOp::Of
        } else if self.eat_word("off") {
            PercentOp::Off
        } else if self.eat_word("on") {
            PercentOp::On
        } else {
            return Ok(lhs);
        };
        let rhs = self.multiplicative()?;
        Ok(Expr::PercentOf(op, Box::new(lhs), Box::new(rhs)))
    }

    fn multiplicative(&mut self) -> Result<Expr, String> {
        let mut lhs = self.implicit()?;
        loop {
            let op = match self.peek() {
                Some(Tok::Op("*")) => BinOp::Mul,
                Some(Tok::Op("/")) => BinOp::Div,
                Some(Tok::Op("%")) => BinOp::Mod,
                Some(Tok::Ident(s)) if s == "mod" => BinOp::Mod,
                _ => return Ok(lhs),
            };
            self.i += 1;
            lhs = Expr::Binary(op, Box::new(lhs), Box::new(self.implicit()?));
        }
    }

    /// Implicit multiplication (`2pi`, `2 x`, `3(x + 1)`, `5 km`) binds tighter than `*` and `/`.
    fn implicit(&mut self) -> Result<Expr, String> {
        let mut lhs = self.unary()?;
        while matches!(self.peek(), Some(Tok::Ident(s)) if !is_keyword(s)) || self.is_op("(") || self.is_op("√") {
            let rhs = if self.is_op("√") { self.nested(Self::unary)? } else { self.nested(Self::power)? };
            lhs = Expr::Binary(BinOp::Mul, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        if self.eat_op("-") {
            return Ok(Expr::Neg(Box::new(self.nested(Self::unary)?)));
        }
        if self.eat_op("+") {
            return self.nested(Self::unary);
        }
        if self.eat_word("not") {
            return Ok(Expr::Not(Box::new(self.nested(Self::unary)?)));
        }
        if self.eat_op("√") {
            return Ok(Expr::Call("sqrt".into(), vec![self.nested(Self::unary)?]));
        }
        self.power()
    }

    fn power(&mut self) -> Result<Expr, String> {
        let base = self.postfix()?;
        if self.eat_op("^") {
            let exponent = self.nested(Self::unary)?;
            return Ok(Expr::Binary(BinOp::Pow, Box::new(base), Box::new(exponent)));
        }
        Ok(base)
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut e = self.primary()?;
        loop {
            if self.eat_op("!") {
                e = Expr::Factorial(Box::new(e));
            } else if self.is_op("%") && !self.starts_operand(1) {
                self.i += 1;
                e = Expr::Percent(Box::new(e));
            } else if self.eat_op("[") {
                let idx = self.list("]")?;
                if idx.is_empty() {
                    return Err(format!("Index expected (char {})", self.pos()));
                }
                e = Expr::Index(Box::new(e), idx);
            } else {
                return Ok(e);
            }
        }
    }

    fn primary(&mut self) -> Result<Expr, String> {
        let pos = self.pos();
        let Some(tok) = self.peek().cloned() else {
            return Err(format!("Unexpected end of expression (char {pos})"));
        };
        self.i += 1;
        match tok {
            Tok::Num(value, text) => Ok(Expr::Num { value, text }),
            Tok::Str(s) => Ok(Expr::Str(s)),
            // `mod(7, 3)`, `and(a, b)`: keywords that are also functions.
            Tok::Ident(name) if is_keyword(&name) && crate::catalog::get(&name).is_some() && self.eat_op("(") => {
                let args = self.list(")")?;
                Ok(Expr::Call(name, args))
            }
            Tok::Ident(name) if is_keyword(&name) => Err(format!("Unexpected keyword \"{name}\" (char {pos})")),
            Tok::Ident(name) => {
                if self.eat_op("(") {
                    let args = self.list(")")?;
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Tok::Op("(") => {
                // Ranges are allowed again inside parentheses: `c ? (1:3) : 0`.
                let saved = std::mem::take(&mut self.no_range);
                let e = self.expr();
                self.no_range = saved;
                let e = e?;
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("[") => self.matrix(),
            Tok::Op(o) => Err(format!("Value expected, got operator {o} (char {pos})")),
        }
    }

    /// Comma-separated expressions up to `close` (the opening token is already consumed).
    fn list(&mut self, close: &str) -> Result<Vec<Expr>, String> {
        let saved = std::mem::take(&mut self.no_range);
        let result = self.list_inner(close);
        self.no_range = saved;
        result
    }

    fn list_inner(&mut self, close: &str) -> Result<Vec<Expr>, String> {
        let mut items = Vec::new();
        if self.eat_op(close) {
            return Ok(items);
        }
        loop {
            items.push(self.expr()?);
            if self.eat_op(close) {
                return Ok(items);
            }
            if !self.eat_op(",") {
                return Err(format!("Parenthesis {close} expected (char {})", self.pos()));
            }
        }
    }

    /// `[1, 2, 3]` or `[1, 2; 3, 4]` (rows separated by `;`).
    fn matrix(&mut self) -> Result<Expr, String> {
        let saved = std::mem::take(&mut self.no_range);
        let result = self.matrix_inner();
        self.no_range = saved;
        result
    }

    fn matrix_inner(&mut self) -> Result<Expr, String> {
        let mut rows = Vec::new();
        let mut row = Vec::new();
        if self.eat_op("]") {
            return Ok(Expr::Matrix(row));
        }
        loop {
            row.push(self.expr()?);
            if self.eat_op(",") {
                continue;
            }
            if self.eat_op(";") {
                rows.push(Expr::Matrix(std::mem::take(&mut row)));
                continue;
            }
            if !self.eat_op("]") {
                return Err(format!("Parenthesis ] expected (char {})", self.pos()));
            }
            if rows.is_empty() {
                return Ok(Expr::Matrix(row));
            }
            rows.push(Expr::Matrix(row));
            return Ok(Expr::Matrix(rows));
        }
    }
}
