use crate::source::{LocatedSpan, Source, Span};
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Unknown,
}

#[derive(Clone, Debug, Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub file: String,
    pub span: LocatedSpan,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub notes: Vec<String>,
    pub suggestion: Option<String>,
}

impl Diagnostic {
    pub fn new(
        source: &Source,
        span: Span,
        severity: Severity,
        code: &str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code: code.into(),
            message: message.into(),
            file: source.path.clone(),
            span: source.located(span),
            expected: None,
            actual: None,
            notes: vec![],
            suggestion: None,
        }
    }
}

#[derive(Debug, Default, Serialize)]
pub struct Summary {
    pub errors: usize,
    pub warnings: usize,
    pub unknowns: usize,
}

#[derive(Debug, Serialize)]
pub struct Report<'a> {
    pub schema_version: u32,
    pub status: &'static str,
    pub input: &'a str,
    pub diagnostics: &'a [Diagnostic],
    pub summary: Summary,
}

impl<'a> Report<'a> {
    pub fn new(input: &'a str, diagnostics: &'a [Diagnostic]) -> Self {
        let mut summary = Summary::default();
        for d in diagnostics {
            match d.severity {
                Severity::Error => summary.errors += 1,
                Severity::Warning => summary.warnings += 1,
                Severity::Unknown => summary.unknowns += 1,
            }
        }
        let status = if summary.errors > 0 {
            "error"
        } else if summary.warnings + summary.unknowns > 0 {
            "warning"
        } else {
            "ok"
        };
        Self {
            schema_version: 1,
            status,
            input,
            diagnostics,
            summary,
        }
    }
}
