use crate::source::{LocatedSpan, Source, Span};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ExpansionContext {
    pub variable: String,
    pub iteration: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct LineMapping {
    pub generated_line: usize,
    pub source: LocatedSpan,
    pub expansion: Vec<ExpansionContext>,
}

#[derive(Debug, Default)]
pub struct Output {
    pub bytes: Vec<u8>,
    pub mappings: Vec<LineMapping>,
}

impl Output {
    pub fn emit(
        &mut self,
        source: &Source,
        span: Span,
        bytes: &[u8],
        context: &[ExpansionContext],
    ) {
        // Each command starts on a line boundary; command text retains physical lines.
        let mut original_line = span.start;
        for part in bytes.split_inclusive(|b| *b == b'\n') {
            let end = source.bytes[original_line..span.end]
                .iter()
                .position(|b| *b == b'\n')
                .map_or(span.end, |p| original_line + p + 1);
            self.mappings.push(LineMapping {
                generated_line: self.mappings.len() + 1,
                source: source.located(Span::new(original_line, end)),
                expansion: context.to_vec(),
            });
            self.bytes.extend_from_slice(part);
            original_line = end;
        }
    }
}
