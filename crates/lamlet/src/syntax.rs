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
