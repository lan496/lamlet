use crate::source::{LocatedSpan, Source, Span};
use serde::Serialize;

/// Stable public diagnostic codes, also exposed by `lamlet schema`.
pub static CODES: &[(&str, &str)] = &[
    (
        "E_ARGUMENT_COUNT",
        "Wrong argument count in a schema-covered form",
    ),
    ("E_CLI", "Invalid command-line arguments"),
    ("E_COMMAND_ORDER", "Definitely invalid command ordering"),
    ("E_DIMENSION_MISMATCH", "Incompatible physical dimensions"),
    ("E_DIVISION_BY_ZERO", "Compile-time division by zero"),
    (
        "E_DUPLICATE_BINDING",
        "Redefinition in the same lexical scope",
    ),
    (
        "E_EXPANSION_LIMIT",
        "Compile-time expansion line or work limit exceeded",
    ),
    ("E_EXPRESSION", "Malformed or unsupported Lamlet expression"),
    ("E_IO", "Input or output filesystem error"),
    (
        "E_NUMERIC_OVERFLOW",
        "Integer overflow or nonfinite numerical result",
    ),
    (
        "E_REFINEMENT",
        "Known value violates a numerical refinement",
    ),
    (
        "E_RESOURCE_LIMIT",
        "Expression size or nesting limit exceeded",
    ),
    ("E_SYNTAX", "Malformed Lamlet syntax"),
    ("E_TYPE_MISMATCH", "Incompatible source-language types"),
    ("E_UNBOUND_NAME", "Compile-time binding is not in scope"),
    ("E_UNKNOWN_ID", "Referenced ID is definitely absent"),
    ("E_UNKNOWN_UNIT", "Unknown explicit unit symbol"),
    (
        "E_UNSUPPORTED_UNITS",
        "Physical conversion requires known metal or real units",
    ),
    (
        "E_UNVERIFIED_ARGUMENT",
        "Strict mode requires schema proof or @unchecked",
    ),
    (
        "E_EFFECT_CONFLICT",
        "Known integrators conflict on the same group",
    ),
    ("E_EXECUTION", "LAMMPS could not start or returned failure"),
    (
        "E_INPUT_SCRIPT",
        "Transitive Lamlet include compilation is not supported",
    ),
    (
        "E_OUTPUT_COLLISION",
        "Typed output aliases an input, source, or another writer",
    ),
    ("E_PATH_CONTEXT", "Runtime working directory is unknown"),
    ("E_PATH_ESCAPE", "Typed path escapes its configured root"),
    ("E_PATH_IO", "Typed path cannot be resolved"),
    ("E_PATH_KIND", "Filesystem object has the wrong kind"),
    ("E_PATH_NOT_FOUND", "Explicit input resource does not exist"),
    ("E_PATH_SYNTAX", "Unsupported typed path syntax"),
    ("E_PROJECT_CONFIG", "Invalid project configuration"),
    (
        "E_PROJECT_REQUIRED",
        "Typed resources require a project context",
    ),
    (
        "E_SCOPE_DEPENDENCY",
        "Live resource still depends on the resource being released",
    ),
    (
        "E_SCOPE_ESCAPE",
        "Unstructured control flow can bypass resource cleanup",
    ),
    ("E_SCOPE_OWNERSHIP", "Managed resource ownership violation"),
    (
        "E_SCOPE_RESOURCE",
        "Invalid managed resource declaration or ambiguous drop",
    ),
    (
        "E_SCOPE_UNVERIFIED",
        "Strict mode cannot verify resource effects",
    ),
    (
        "E_SCOPE_USE_AFTER_FREE",
        "Managed resource is outside its lifetime",
    ),
    (
        "W_DYNAMIC_PATH",
        "Runtime filename is not statically verified",
    ),
    (
        "W_SCOPE_UNVERIFIED",
        "Opaque command may invalidate resource ownership or cleanup",
    ),
    (
        "U_COMMAND_ORDER",
        "Box state is unknown at an ordering constraint",
    ),
    ("U_ID_STATE", "ID existence is unknown"),
    (
        "U_OPAQUE_COMMAND",
        "Command boundaries or effects are not verified by Lamlet",
    ),
    ("U_PARTIAL_COMMAND", "Only part of the command is covered"),
    (
        "U_RUNTIME_VALUE",
        "Operand value is not known at compile time",
    ),
    ("U_UNIT_STYLE", "Unit style is not recognized by Lamlet"),
    (
        "W_UNCHECKED_INTERPOLATION",
        "Explicit operand validation escape hatch",
    ),
    (
        "W_UNVERIFIED_ARGUMENT",
        "Interpolated operand is not covered by a schema",
    ),
];

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
