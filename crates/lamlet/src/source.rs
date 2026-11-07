use serde::Serialize;

/// A half-open byte range in the original input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

#[derive(Clone, Debug)]
pub struct Source {
    pub path: String,
    pub bytes: Vec<u8>,
    line_starts: Vec<usize>,
}

impl Source {
    pub fn new(path: impl Into<String>, bytes: Vec<u8>) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            bytes
                .iter()
                .enumerate()
                .filter_map(|(i, b)| (*b == b'\n').then_some(i + 1)),
        );
        Self {
            path: path.into(),
            bytes,
            line_starts,
        }
    }

    pub fn location(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.bytes.len());
        let line = self.line_starts.partition_point(|start| *start <= offset) - 1;
        let column = String::from_utf8_lossy(&self.bytes[self.line_starts[line]..offset])
            .chars()
            .count()
            + 1;
        (line + 1, column)
    }

    pub fn located(&self, span: Span) -> LocatedSpan {
        let (start_line, start_column) = self.location(span.start);
        let (end_line, end_column) = self.location(span.end);
        LocatedSpan {
            start_byte: span.start,
            end_byte: span.end,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct LocatedSpan {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}
