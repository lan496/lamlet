use crate::{
    source::Span,
    units::{Dimension, unit},
};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int,
    Float,
    Bool,
    Quantity(Dimension),
    List(Box<Type>),
    Unknown,
}

impl Type {
    pub fn name(&self) -> String {
        match self {
            Self::Int => "Int".into(),
            Self::Float => "Float".into(),
            Self::Bool => "Bool".into(),
            Self::Quantity(d) => d.name(),
            Self::List(t) => format!("List({})", t.name()),
            Self::Unknown => "Unknown".into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Quantity(f64, Dimension),
}

impl Value {
    pub fn ty(&self) -> Type {
        match self {
            Self::Int(_) => Type::Int,
            Self::Float(_) => Type::Float,
            Self::Bool(_) => Type::Bool,
            Self::Quantity(_, d) => Type::Quantity(*d),
        }
    }
    pub fn number(&self) -> Option<f64> {
        match self {
            Self::Int(n) => Some(*n as f64),
            Self::Float(n) | Self::Quantity(n, _) => Some(*n),
            Self::Bool(_) => None,
        }
    }
    pub fn dimension(&self) -> Dimension {
        if let Self::Quantity(_, d) = self {
            *d
        } else {
            Dimension::SCALAR
        }
    }
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}
#[derive(Clone, Debug)]
pub enum ExprKind {
    Literal(Value),
    Name(String),
    Neg(Box<Expr>),
    Binary(char, Box<Expr>, Box<Expr>),
}

#[derive(Clone, Debug)]
pub struct ExprError {
    pub code: &'static str,
    pub message: String,
    pub span: Span,
}
impl ExprError {
    fn new(code: &'static str, message: impl Into<String>, span: Span) -> Self {
        Self {
            code,
            message: message.into(),
            span,
        }
    }
}

#[derive(Clone, Debug)]
enum TokenKind {
    Number(String),
    Name(String),
    Punct(char),
    End,
}
#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    span: Span,
}

fn lex(text: &str, offset: usize) -> Result<Vec<Token>, ExprError> {
    let b = text.as_bytes();
    let mut i = 0;
    let mut tokens = vec![];
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        let kind = if b[i].is_ascii_digit() || b[i] == b'.' {
            while i < b.len() && (b[i].is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && matches!(b[i], b'e' | b'E') {
                i += 1;
                if i < b.len() && matches!(b[i], b'+' | b'-') {
                    i += 1;
                }
                while i < b.len() && b[i].is_ascii_digit() {
                    i += 1;
                }
            }
            TokenKind::Number(text[start..i].into())
        } else if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                i += 1;
            }
            if &text[start..i] == "kcal" && text[i..].starts_with("/mol") {
                i += 4;
            }
            TokenKind::Name(text[start..i].into())
        } else if b"+-*/()[],".contains(&b[i]) {
            i += 1;
            TokenKind::Punct(b[start] as char)
        } else {
            let end = start + text[start..].chars().next().unwrap().len_utf8();
            return Err(ExprError::new(
                "E_EXPRESSION",
                "Unsupported character in Lamlet expression",
                Span::new(offset + start, offset + end),
            ));
        };
        tokens.push(Token {
            kind,
            span: Span::new(offset + start, offset + i),
        });
        if tokens.len() > 512 {
            return Err(ExprError::new(
                "E_RESOURCE_LIMIT",
                "Expression exceeds 512 tokens",
                Span::new(offset + start, offset + i),
            ));
        }
    }
    tokens.push(Token {
        kind: TokenKind::End,
        span: Span::new(offset + i, offset + i),
    });
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    depth: usize,
}
impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }
    fn take(&mut self) -> Token {
        let t = self.peek().clone();
        if !matches!(t.kind, TokenKind::End) {
            self.pos += 1;
        }
        t
    }
    fn punct(&mut self, c: char) -> Result<Token, ExprError> {
        let token = self.take();
        if matches!(token.kind, TokenKind::Punct(p) if p == c) {
            Ok(token)
        } else {
            Err(ExprError::new(
                "E_EXPRESSION",
                format!("Expected '{c}'"),
                token.span,
            ))
        }
    }
    fn expression(&mut self, min_bp: u8) -> Result<Expr, ExprError> {
        self.depth += 1;
        if self.depth > 128 {
            return Err(ExprError::new(
                "E_RESOURCE_LIMIT",
                "Expression nesting exceeds 128 levels",
                self.peek().span,
            ));
        }
        let token = self.take();
        let mut lhs = match token.kind {
            TokenKind::Number(s) => {
                let value = if s.contains(['.', 'e', 'E']) {
                    Value::Float(s.parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(
                        || {
                            ExprError::new(
                                "E_NUMERIC_OVERFLOW",
                                "Expected a finite floating literal",
                                token.span,
                            )
                        },
                    )?)
                } else {
                    Value::Int(s.parse().map_err(|_| {
                        ExprError::new(
                            "E_NUMERIC_OVERFLOW",
                            "Integer literal exceeds signed 64-bit range",
                            token.span,
                        )
                    })?)
                };
                if let TokenKind::Name(symbol) = &self.peek().kind {
                    let (dim, factor) = unit(symbol).ok_or_else(|| {
                        ExprError::new(
                            "E_UNKNOWN_UNIT",
                            format!("Unknown unit '{symbol}'"),
                            self.peek().span,
                        )
                    })?;
                    if token.span.end == self.peek().span.start {
                        return Err(ExprError::new(
                            "E_EXPRESSION",
                            "Separate a number and unit with whitespace",
                            self.peek().span,
                        ));
                    }
                    let end = self.take().span.end;
                    let n = value.number().unwrap() * factor;
                    if !n.is_finite() {
                        return Err(ExprError::new(
                            "E_NUMERIC_OVERFLOW",
                            "Quantity overflows",
                            token.span,
                        ));
                    }
                    Expr {
                        kind: ExprKind::Literal(Value::Quantity(n, dim)),
                        span: Span::new(token.span.start, end),
                    }
                } else {
                    Expr {
                        kind: ExprKind::Literal(value),
                        span: token.span,
                    }
                }
            }
            TokenKind::Name(s) if s == "true" || s == "false" => Expr {
                kind: ExprKind::Literal(Value::Bool(s == "true")),
                span: token.span,
            },
            TokenKind::Name(s) => Expr {
                kind: ExprKind::Name(s),
                span: token.span,
            },
            TokenKind::Punct('-') => {
                let rhs = self.expression(5)?;
                let span = Span::new(token.span.start, rhs.span.end);
                Expr {
                    kind: ExprKind::Neg(Box::new(rhs)),
                    span,
                }
            }
            TokenKind::Punct('(') => {
                let mut expr = self.expression(0)?;
                let end = self.punct(')')?.span.end;
                expr.span = Span::new(token.span.start, end);
                expr
            }
            _ => {
                return Err(ExprError::new(
                    "E_EXPRESSION",
                    "Expected a literal, name, unary minus, or parenthesized expression",
                    token.span,
                ));
            }
        };
        loop {
            let (op, left_bp, right_bp) = match self.peek().kind {
                TokenKind::Punct(op @ ('+' | '-')) => (op, 1, 2),
                TokenKind::Punct(op @ ('*' | '/')) => (op, 3, 4),
                _ => break,
            };
            if left_bp < min_bp {
                break;
            }
            self.take();
            let rhs = self.expression(right_bp)?;
            let span = Span::new(lhs.span.start, rhs.span.end);
            lhs = Expr {
                kind: ExprKind::Binary(op, Box::new(lhs), Box::new(rhs)),
                span,
            };
        }
        self.depth -= 1;
        Ok(lhs)
    }
    fn finish(&self) -> Result<(), ExprError> {
        if matches!(self.peek().kind, TokenKind::End) {
            Ok(())
        } else {
            Err(ExprError::new(
                "E_EXPRESSION",
                "Unexpected expression suffix",
                self.peek().span,
            ))
        }
    }
}

pub fn parse(text: &str, offset: usize) -> Result<Expr, ExprError> {
    let mut p = Parser {
        tokens: lex(text, offset)?,
        pos: 0,
        depth: 0,
    };
    let expr = p.expression(0)?;
    p.finish()?;
    Ok(expr)
}

pub fn parse_list(text: &str, offset: usize) -> Result<Vec<Expr>, ExprError> {
    let mut p = Parser {
        tokens: lex(text, offset)?,
        pos: 0,
        depth: 0,
    };
    p.punct('[')?;
    let mut items = vec![];
    if !matches!(p.peek().kind, TokenKind::Punct(']')) {
        loop {
            items.push(p.expression(0)?);
            if !matches!(p.peek().kind, TokenKind::Punct(',')) {
                break;
            }
            p.take();
        }
    }
    p.punct(']')?;
    p.finish()?;
    Ok(items)
}

pub type Scope = BTreeMap<String, Value>;

pub fn evaluate(expr: &Expr, scopes: &[Scope]) -> Result<Value, ExprError> {
    let error = |code, message| ExprError::new(code, message, expr.span);
    let value =
        match &expr.kind {
            ExprKind::Literal(v) => v.clone(),
            ExprKind::Name(name) => scopes
                .iter()
                .rev()
                .find_map(|s| s.get(name))
                .cloned()
                .ok_or_else(|| {
                    error(
                        "E_UNBOUND_NAME",
                        format!("No compile-time binding named '{name}'"),
                    )
                })?,
            ExprKind::Neg(rhs) => match evaluate(rhs, scopes)? {
                Value::Int(n) => Value::Int(n.checked_neg().ok_or_else(|| {
                    error("E_NUMERIC_OVERFLOW", "Integer negation overflows".into())
                })?),
                Value::Float(n) => Value::Float(-n),
                Value::Quantity(n, d) => Value::Quantity(-n, d),
                Value::Bool(_) => {
                    return Err(error(
                        "E_TYPE_MISMATCH",
                        "Unary minus requires a number".into(),
                    ));
                }
            },
            ExprKind::Binary(op, lhs, rhs) => {
                let l = evaluate(lhs, scopes)?;
                let r = evaluate(rhs, scopes)?;
                let (a, b) = l.number().zip(r.number()).ok_or_else(|| {
                    error("E_TYPE_MISMATCH", "Arithmetic does not accept Bool".into())
                })?;
                if *op == '/' && b == 0.0 {
                    return Err(error(
                        "E_DIVISION_BY_ZERO",
                        "Division by a compile-time zero".into(),
                    ));
                }
                if let (Value::Int(a), Value::Int(b)) = (&l, &r) {
                    if *op != '/' {
                        let n = match op {
                            '+' => a.checked_add(*b),
                            '-' => a.checked_sub(*b),
                            '*' => a.checked_mul(*b),
                            _ => unreachable!(),
                        };
                        return n.map(Value::Int).ok_or_else(|| {
                            error("E_NUMERIC_OVERFLOW", "Integer arithmetic overflows".into())
                        });
                    }
                }
                let dl = l.dimension();
                let dr = r.dimension();
                let d = if matches!(op, '+' | '-') {
                    if dl != dr {
                        return Err(error(
                            "E_DIMENSION_MISMATCH",
                            format!("Cannot {op} {} and {}", l.ty().name(), r.ty().name()),
                        ));
                    }
                    dl
                } else {
                    dl.combine(dr, *op == '/').ok_or_else(|| {
                        error("E_NUMERIC_OVERFLOW", "Dimension exponent overflows".into())
                    })?
                };
                let n = match op {
                    '+' => a + b,
                    '-' => a - b,
                    '*' => a * b,
                    '/' => a / b,
                    _ => unreachable!(),
                };
                if d == Dimension::SCALAR {
                    Value::Float(n)
                } else {
                    Value::Quantity(n, d)
                }
            }
        };
    if value.number().is_some_and(|n| !n.is_finite()) {
        return Err(error(
            "E_NUMERIC_OVERFLOW",
            "Nonfinite compile-time result".into(),
        ));
    }
    Ok(value)
}
