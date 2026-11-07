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

#[derive(Debug, Serialize)]
pub struct SourceMap<'a> {
    pub format_version: u32,
    pub original_source: &'a str,
    pub output: &'a str,
    pub lines: &'a [LineMapping],
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
        // A whole logical-command span remains accurate when interpolation removes
        // continuation line breaks. No generated sentinel text is needed.
        for part in bytes.split_inclusive(|b| *b == b'\n') {
            self.mappings.push(LineMapping {
                generated_line: self.mappings.len() + 1,
                source: source.located(span),
                expansion: context.to_vec(),
            });
            self.bytes.extend_from_slice(part);
        }
    }
}
