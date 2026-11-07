use crate::expr::{self, Expr, ExprError};
use crate::source::Span;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quote {
    None,
    Single,
    Double,
    Triple,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalLine {
    pub span: Span,
    /// Byte positions of continuation markers removed by LAMMPS.
    pub continuations: Vec<Span>,
}

/// Recognize LAMMPS logical lines without normalizing their source bytes.
pub fn logical_lines(bytes: &[u8]) -> Vec<LogicalLine> {
    let mut lines = vec![];
    let mut start = 0;
    let mut physical_start = 0;
    let mut quote = Quote::None;
    let mut comment = false;
    let mut continuations = vec![];
    while physical_start < bytes.len() {
        let end = bytes[physical_start..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(bytes.len(), |p| physical_start + p + 1);
        let mut i = physical_start;
        while i < end {
            if comment {
                break;
            }
            match quote {
                Quote::None => match bytes[i] {
                    b'#' => comment = true,
                    b'\'' => quote = Quote::Single,
                    b'"' if bytes[i..end].starts_with(b"\"\"\"") => {
                        quote = Quote::Triple;
                        i += 2;
                    }
                    b'"' => quote = Quote::Double,
                    _ => {}
                },
                Quote::Single if bytes[i] == b'\'' => quote = Quote::None,
                Quote::Double if bytes[i] == b'"' => quote = Quote::None,
                Quote::Triple if bytes[i..end].starts_with(b"\"\"\"") => {
                    quote = Quote::None;
                    i += 2;
                }
                _ => {}
            }
            i += 1;
        }
        let printable_end = bytes[physical_start..end]
            .iter()
            .rposition(|b| !b.is_ascii_whitespace())
            .map(|p| physical_start + p);
        let continued = quote != Quote::Triple && printable_end.is_some_and(|p| bytes[p] == b'&');
        if continued {
            continuations.push(Span::new(printable_end.unwrap(), end));
        }
        if (!continued && quote != Quote::Triple) || end == bytes.len() {
            lines.push(LogicalLine {
                span: Span::new(start, end),
                continuations: std::mem::take(&mut continuations),
            });
            start = end;
            quote = Quote::None;
            comment = false;
        }
        physical_start = end;
    }
    lines
}

#[derive(Clone, Debug)]
pub struct Interpolation {
    pub span: Span,
    pub expression: Expr,
    pub unchecked: bool,
}

#[derive(Clone, Debug)]
pub struct Argument {
    pub span: Span,
    pub text: String,
    pub quoted: bool,
    pub dynamic: bool,
    pub interpolation: Option<Interpolation>,
}

#[derive(Clone, Debug)]
pub struct Command {
    pub line: LogicalLine,
    pub arguments: Vec<Argument>,
    pub unambiguous: bool,
}

#[derive(Clone, Debug)]
pub enum Node {
    Let {
        name: String,
        annotation: Option<crate::resources::ResourceKind>,
        expression: Expr,
        span: Span,
    },
    For {
        name: String,
        items: Vec<Expr>,
        body: Vec<Node>,
        span: Span,
    },
    Command(Command),
    Scope {
        body: Vec<Node>,
        end: Span,
    },
    Managed(Command),
    Drop {
        id: String,
        span: Span,
    },
}

fn syntax_error(message: impl Into<String>, span: Span) -> ExprError {
    ExprError {
        code: "E_SYNTAX",
        message: message.into(),
        span,
    }
}

/// A logical view with a byte-to-original-source map; used only for analysis.
struct LogicalText {
    bytes: Vec<u8>,
    positions: Vec<usize>,
    end: usize,
}
impl LogicalText {
    fn new(source: &[u8], line: &LogicalLine) -> Self {
        let mut bytes = vec![];
        let mut positions = vec![];
        let mut i = line.span.start;
        for continuation in &line.continuations {
            bytes.extend_from_slice(&source[i..continuation.start]);
            positions.extend(i..continuation.start);
            i = continuation.end;
        }
        bytes.extend_from_slice(&source[i..line.span.end]);
        positions.extend(i..line.span.end);
        Self {
            bytes,
            positions,
            end: line.span.end,
        }
    }
    fn span(&self, start: usize, end: usize) -> Span {
        Span::new(
            self.positions.get(start).copied().unwrap_or(self.end),
            if end > start {
                self.positions[end - 1] + 1
            } else {
                self.positions.get(end).copied().unwrap_or(self.end)
            },
        )
    }
    fn map_expr(&self, expr: &mut Expr) {
        expr.span = self.span(expr.span.start, expr.span.end);
        match &mut expr.kind {
            expr::ExprKind::Neg(e) => self.map_expr(e),
            expr::ExprKind::Binary(_, a, b) => {
                self.map_expr(a);
                self.map_expr(b);
            }
            _ => {}
        }
    }
    fn map_error(&self, mut error: ExprError) -> ExprError {
        error.span = self.span(error.span.start, error.span.end);
        error
    }
}

pub fn command(source: &[u8], line: LogicalLine, extended: bool) -> Result<Command, ExprError> {
    let text = LogicalText::new(source, &line);
    let b = &text.bytes;
    let mut i = 0;
    let mut arguments = vec![];
    let mut unambiguous = true;
    while i < b.len() {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if i == b.len() || b[i] == b'#' {
            break;
        }
        let start = i;
        let mut quote = Quote::None;
        let mut quoted = false;
        let mut dynamic = false;
        let mut interpolation = None;
        while i < b.len() {
            if quote == Quote::None && (b[i].is_ascii_whitespace() || b[i] == b'#') {
                break;
            }
            match quote {
                Quote::None => {
                    if b[i] == b'\'' || b[i] == b'"' {
                        if i != start {
                            unambiguous = false;
                        }
                        quoted = true;
                        if b[i..].starts_with(b"\"\"\"") {
                            quote = Quote::Triple;
                            i += 3;
                        } else {
                            quote = if b[i] == b'\'' {
                                Quote::Single
                            } else {
                                Quote::Double
                            };
                            i += 1;
                        }
                        continue;
                    }
                    if b[i] == b'$' {
                        dynamic = true;
                        unambiguous = false;
                        // Runtime substitutions can contain spaces and can expand to many arguments.
                        if b.get(i + 1) == Some(&b'(') {
                            i += 2;
                            let mut depth = 1;
                            while i < b.len() && depth > 0 {
                                if b[i] == b'(' {
                                    depth += 1;
                                } else if b[i] == b')' {
                                    depth -= 1;
                                }
                                i += 1;
                            }
                            continue;
                        }
                    }
                    if extended
                        && b[i] == b'@'
                        && (b[i..].starts_with(b"@{") || b[i..].starts_with(b"@unchecked"))
                    {
                        let (prefix, unchecked) = if b[i..].starts_with(b"@unchecked{") {
                            (11, true)
                        } else if b[i..].starts_with(b"@{") {
                            (2, false)
                        } else {
                            return Err(syntax_error(
                                "Malformed Lamlet interpolation",
                                text.span(i, i + 1),
                            ));
                        };
                        if i != start {
                            return Err(syntax_error(
                                "Interpolation must occupy a complete unquoted argument",
                                text.span(start, i + prefix),
                            ));
                        }
                        let end = unquoted_byte(&b[i + prefix..], b'}')
                            .map(|p| i + prefix + p)
                            .ok_or_else(|| {
                                syntax_error("Unclosed interpolation", text.span(i, b.len()))
                            })?;
                        if arguments.is_empty() {
                            return Err(syntax_error(
                                "Interpolation cannot synthesize a command name",
                                text.span(i, end + 1),
                            ));
                        }
                        if end + 1 < b.len()
                            && !b[end + 1].is_ascii_whitespace()
                            && b[end + 1] != b'#'
                        {
                            return Err(syntax_error(
                                "Interpolation must occupy a complete argument",
                                text.span(i, end + 2),
                            ));
                        }
                        let expression_text =
                            std::str::from_utf8(&b[i + prefix..end]).map_err(|_| {
                                syntax_error(
                                    "Lamlet expressions must be UTF-8",
                                    text.span(i, end + 1),
                                )
                            })?;
                        let mut expression = expr::parse(expression_text, i + prefix)
                            .map_err(|e| text.map_error(e))?;
                        text.map_expr(&mut expression);
                        interpolation = Some(Interpolation {
                            span: text.span(i, end + 1),
                            expression,
                            unchecked,
                        });
                        i = end + 1;
                        continue;
                    }
                }
                Quote::Single | Quote::Double => {
                    let delimiter = if quote == Quote::Single { b'\'' } else { b'"' };
                    if b[i] == delimiter {
                        quote = Quote::None;
                        if i + 1 < b.len() && !b[i + 1].is_ascii_whitespace() && b[i + 1] != b'#' {
                            unambiguous = false;
                        }
                    }
                }
                Quote::Triple => {
                    if b[i..].starts_with(b"\"\"\"") {
                        quote = Quote::None;
                        i += 3;
                        continue;
                    }
                }
            }
            i += 1;
        }
        if quote != Quote::None {
            unambiguous = false;
        }
        let raw = String::from_utf8_lossy(&b[start..i]);
        let value = if quoted {
            if raw.starts_with("\"\"\"") && raw.ends_with("\"\"\"") && raw.len() >= 6 {
                raw[3..raw.len() - 3].into()
            } else if raw.len() >= 2
                && ((raw.starts_with('\'') && raw.ends_with('\''))
                    || (raw.starts_with('"') && raw.ends_with('"')))
            {
                raw[1..raw.len() - 1].into()
            } else {
                raw.into_owned()
            }
        } else {
            raw.into_owned()
        };
        arguments.push(Argument {
            span: text.span(start, i),
            text: value,
            quoted,
            dynamic,
            interpolation,
        });
    }
    Ok(Command {
        line,
        arguments,
        unambiguous,
    })
}

fn identifier(name: &str) -> bool {
    let mut chars = name.bytes();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_')
}

fn unquoted_byte(bytes: &[u8], target: u8) -> Option<usize> {
    let mut quote = Quote::None;
    let mut i = 0;
    while i < bytes.len() {
        match quote {
            Quote::None if bytes[i] == target => return Some(i),
            Quote::None if bytes[i..].starts_with(b"\"\"\"") => {
                quote = Quote::Triple;
                i += 3;
                continue;
            }
            Quote::None if bytes[i] == b'"' => quote = Quote::Double,
            Quote::None if bytes[i] == b'\'' => quote = Quote::Single,
            Quote::Double if bytes[i] == b'\\' => {
                i += 2;
                continue;
            }
            Quote::Double if bytes[i] == b'"' => quote = Quote::None,
            Quote::Single if bytes[i] == b'\'' => quote = Quote::None,
            Quote::Triple if bytes[i..].starts_with(b"\"\"\"") => {
                quote = Quote::None;
                i += 3;
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    None
}

pub fn parse(source: &[u8], extended: bool) -> Result<Vec<Node>, ExprError> {
    let lines = logical_lines(source);
    let mut position = 0;
    parse_block(source, &lines, &mut position, extended, 0)
}

fn parse_block(
    source: &[u8],
    lines: &[LogicalLine],
    position: &mut usize,
    extended: bool,
    depth: usize,
) -> Result<Vec<Node>, ExprError> {
    let mut nodes = vec![];
    while let Some(line) = lines.get(*position) {
        *position += 1;
        let text = LogicalText::new(source, line);
        let trimmed_start = text
            .bytes
            .iter()
            .position(|c| !c.is_ascii_whitespace())
            .unwrap_or(text.bytes.len());
        let content_end = unquoted_byte(&text.bytes, b'#').unwrap_or(text.bytes.len());
        let directive = text.bytes.get(trimmed_start) == Some(&b'@');
        if extended && content_end >= trimmed_start {
            let content = std::str::from_utf8(&text.bytes[trimmed_start..content_end]);
            if let Ok(content) = content {
                let content = content.trim_end();
                if depth > 0 && content == "}" {
                    return Ok(nodes);
                }
            }
        }
        if !extended || !directive {
            nodes.push(Node::Command(command(source, line.clone(), extended)?));
            continue;
        }
        let content = std::str::from_utf8(&text.bytes[trimmed_start..content_end])
            .map_err(|_| syntax_error("Lamlet directives must be UTF-8", line.span))?
            .trim_end();
        if let Some(binding) = content
            .strip_prefix("@let ")
            .or_else(|| content.strip_prefix("@let\t"))
        {
            let (name, expression_text) = binding
                .split_once('=')
                .ok_or_else(|| syntax_error("Expected @let NAME = EXPR", line.span))?;
            let (name, annotation) = if let Some((name, annotation)) = name.split_once(':') {
                let kind =
                    crate::resources::ResourceKind::parse(annotation.trim()).ok_or_else(|| {
                        syntax_error("Expected a resource type annotation", line.span)
                    })?;
                (name.trim(), Some(kind))
            } else {
                (name.trim(), None)
            };
            if !identifier(name) {
                return Err(syntax_error("Invalid binding name", line.span));
            }
            let offset = trimmed_start + content.len() - expression_text.len();
            let mut expression =
                expr::parse(expression_text, offset).map_err(|e| text.map_error(e))?;
            text.map_expr(&mut expression);
            nodes.push(Node::Let {
                name: name.into(),
                annotation,
                expression,
                span: line.span,
            });
        } else if content
            .strip_suffix('{')
            .is_some_and(|s| s.trim_end() == "@scope")
        {
            if depth >= 64 {
                return Err(syntax_error("Block nesting exceeds 64 levels", line.span));
            }
            let body = parse_block(source, lines, position, extended, depth + 1)?;
            let end = lines[*position - 1].span;
            nodes.push(Node::Scope { body, end });
        } else if let Some(id) = content
            .strip_prefix("@drop ")
            .or_else(|| content.strip_prefix("@drop\t"))
        {
            let id = id.trim();
            if id.is_empty() || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
                return Err(syntax_error("Expected @drop RESOURCE_ID", line.span));
            }
            nodes.push(Node::Drop {
                id: id.into(),
                span: line.span,
            });
        } else if ["@fix", "@compute", "@dump"].iter().any(|name| {
            content
                .strip_prefix(name)
                .is_some_and(|s| s.starts_with(char::is_whitespace))
        }) {
            let header_span = Span::new(
                text.span(trimmed_start, trimmed_start + 1).end,
                line.span.end,
            );
            let header_line = LogicalLine {
                span: header_span,
                continuations: line.continuations.clone(),
            };
            nodes.push(Node::Managed(command(source, header_line, true)?));
        } else if let Some(header) = content
            .strip_prefix("@for ")
            .or_else(|| content.strip_prefix("@for\t"))
        {
            if depth >= 64 {
                return Err(syntax_error("Loop nesting exceeds 64 levels", line.span));
            }
            let header = header
                .strip_suffix('{')
                .ok_or_else(|| syntax_error("Expected @for NAME in [EXPR, ...] {", line.span))?;
            let name_end = header
                .find(char::is_whitespace)
                .ok_or_else(|| syntax_error("Expected a loop variable and iterable", line.span))?;
            let name = &header[..name_end];
            let rest = header[name_end..].trim_start();
            let list = rest
                .strip_prefix("in")
                .filter(|s| s.starts_with(char::is_whitespace))
                .ok_or_else(|| syntax_error("Expected 'in' before the loop iterable", line.span))?;
            if !identifier(name) {
                return Err(syntax_error("Invalid loop variable", line.span));
            }
            let offset = trimmed_start + content.len() - 1 - list.len();
            let mut items = expr::parse_list(list, offset).map_err(|e| text.map_error(e))?;
            for item in &mut items {
                text.map_expr(item);
            }
            let body = parse_block(source, lines, position, extended, depth + 1)?;
            nodes.push(Node::For {
                name: name.into(),
                items,
                body,
                span: line.span,
            });
        } else {
            return Err(syntax_error(
                "Unknown or malformed Lamlet directive",
                line.span,
            ));
        }
    }
    if depth > 0 {
        Err(syntax_error(
            "Unclosed Lamlet block",
            lines.last().map_or(Span::default(), |l| l.span),
        ))
    } else {
        Ok(nodes)
    }
}
