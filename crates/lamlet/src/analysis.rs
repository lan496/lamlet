use crate::{
    diagnostic::{Diagnostic, Severity},
    expr::Type,
    hir,
    schema::{self, Requirement},
    source::{Source, Span},
    units::Dimension,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fact {
    Present,
    Absent,
    Unknown,
}

#[derive(Clone, Debug)]
pub struct Inventory {
    facts: BTreeMap<String, Fact>,
    fallback: Fact,
}
impl Inventory {
    fn empty() -> Self {
        Self {
            facts: BTreeMap::new(),
            fallback: Fact::Absent,
        }
    }
    fn weaken(&mut self) {
        self.facts.clear();
        self.fallback = Fact::Unknown;
    }
    fn get(&self, id: &str) -> Fact {
        self.facts.get(id).copied().unwrap_or(self.fallback)
    }
    fn set(&mut self, id: &str, fact: Fact) {
        self.facts.insert(id.into(), fact);
    }
}

#[derive(Clone, Debug)]
pub struct State {
    pub units: Option<String>,
    pub box_exists: Fact,
    pub atom_style: Option<String>,
    pub groups: Inventory,
    pub fixes: Inventory,
    pub computes: Inventory,
    opaque_callbacks: bool,
}
impl Default for State {
    fn default() -> Self {
        let mut groups = Inventory::empty();
        groups.set("all", Fact::Present);
        let mut computes = Inventory::empty();
        for id in ["thermo_temp", "thermo_press", "thermo_pe"] {
            computes.set(id, Fact::Present);
        }
        Self {
            units: Some("lj".into()),
            box_exists: Fact::Absent,
            atom_style: Some("atomic".into()),
            groups,
            fixes: Inventory::empty(),
            computes,
            opaque_callbacks: false,
        }
    }
}
impl State {
    fn weaken(&mut self) {
        self.units = None;
        self.box_exists = Fact::Unknown;
        self.atom_style = None;
        self.groups.weaken();
        self.groups.set("all", Fact::Present);
        self.fixes.weaken();
        self.computes.weaken();
        self.opaque_callbacks = true;
    }
}

fn diagnostic(
    source: &Source,
    span: Span,
    severity: Severity,
    code: &str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(source, span, severity, code, message)
}

fn requirement(
    source: &Source,
    operand: &hir::Operand,
    req: Requirement,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if operand.unchecked {
        return;
    }
    let Some(value) = &operand.value else {
        diagnostics.push(diagnostic(
            source,
            operand.span,
            Severity::Unknown,
            "U_RUNTIME_VALUE",
            "Operand cannot be determined at compile time",
        ));
        return;
    };
    let dimension = match req {
        Requirement::PositiveTime => Some(Dimension::TIME),
        Requirement::Temperature => Some(Dimension::TEMPERATURE),
        Requirement::Pressure => Some(Dimension::PRESSURE),
        Requirement::NonNegativeInt => None,
    };
    let actual = value.ty();
    let mismatch = match (dimension, &actual) {
        (Some(d), Type::Quantity(got)) => d != *got,
        (Some(_), Type::Int | Type::Float) => false,
        (None, Type::Int) => false,
        _ => true,
    };
    if mismatch {
        let expected = dimension.map_or_else(|| "NonNegativeInt".into(), Dimension::name);
        let code = if matches!(actual, Type::Quantity(_)) {
            "E_DIMENSION_MISMATCH"
        } else {
            "E_TYPE_MISMATCH"
        };
        let mut d = diagnostic(
            source,
            operand.span,
            Severity::Error,
            code,
            format!(
                "Expected {expected}, but this expression has {}",
                actual.name()
            ),
        );
        d.expected = Some(expected);
        d.actual = Some(actual.name());
        if dimension == Some(Dimension::TIME) {
            d.suggestion = Some("Use a positive time quantity such as 2 fs".into());
        }
        diagnostics.push(d);
        return;
    }
    let n = value.number().unwrap();
    if !n.is_finite() {
        diagnostics.push(diagnostic(
            source,
            operand.span,
            Severity::Error,
            "E_NUMERIC_OVERFLOW",
            "Numeric operand must be finite",
        ));
        return;
    }
    let invalid = match req {
        Requirement::PositiveTime => n <= 0.0,
        Requirement::NonNegativeInt => n < 0.0,
        _ => false,
    };
    if invalid {
        diagnostics.push(diagnostic(
            source,
            operand.span,
            Severity::Error,
            "E_REFINEMENT",
            format!("Value does not satisfy {req:?}"),
        ));
    }
}

fn reference(
    source: &Source,
    span: Span,
    inventory: &Inventory,
    id: &str,
    kind: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match inventory.get(id) {
        Fact::Present => {}
        Fact::Absent => diagnostics.push(diagnostic(
            source,
            span,
            Severity::Error,
            "E_UNKNOWN_ID",
            format!("{kind} ID '{id}' is definitely absent"),
        )),
        Fact::Unknown => diagnostics.push(diagnostic(
            source,
            span,
            Severity::Unknown,
            "U_ID_STATE",
            format!("Cannot prove that {kind} ID '{id}' exists"),
        )),
    }
}

/// Validate local operands before applying sequential state effects.
pub fn analyze(
    source: &Source,
    command: &hir::Command<'_>,
    state: &mut State,
    strict: bool,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let syntax = command.syntax;
    let args = &syntax.arguments;
    let Some(first) = args.first() else {
        return;
    };
    let name = first.text.as_str();
    let base_schema = schema::find(name);
    let mut requirements = vec![None; args.len()];
    let mut complete_prefix = true;
    let mut schema = base_schema;
    if syntax.unambiguous {
        if let Some(base) = base_schema {
            let count = args.len() - 1;
            if count < base.min_arguments || base.max_arguments.is_some_and(|max| count > max) {
                diagnostics.push(diagnostic(
                    source,
                    syntax.line.span,
                    Severity::Error,
                    "E_ARGUMENT_COUNT",
                    format!("Wrong argument count for {name}"),
                ));
                return;
            }
            for operand in base.operands {
                requirements[operand.position] = Some(operand.requirement);
            }
            if name == "fix" {
                schema = schema::find(&format!("fix/{}", args[3].text));
                if let Some(fix) = schema {
                    let mut i = 4;
                    while i < args.len() {
                        let Some(keyword) = fix
                            .keywords
                            .iter()
                            .find(|k| k.keyword == args[i].text && args[i].interpolation.is_none())
                        else {
                            complete_prefix = false;
                            break;
                        };
                        if i + keyword.operands.len() >= args.len() {
                            diagnostics.push(diagnostic(
                                source,
                                args[i].span,
                                Severity::Error,
                                "E_ARGUMENT_COUNT",
                                format!(
                                    "fix {} keyword '{}' requires {} operands",
                                    args[3].text,
                                    keyword.keyword,
                                    keyword.operands.len()
                                ),
                            ));
                            break;
                        }
                        for (offset, req) in keyword.operands.iter().enumerate() {
                            requirements[i + 1 + offset] = Some(*req);
                        }
                        i += 1 + keyword.operands.len();
                    }
                }
            }
        }
    }
    for (i, operand) in command.operands.iter().enumerate().skip(1) {
        if operand.unchecked {
            continue;
        }
        if let Some(req) = requirements[i] {
            requirement(source, operand, req, diagnostics);
        } else if operand.introduced {
            let (severity, code) = if strict {
                (Severity::Error, "E_UNVERIFIED_ARGUMENT")
            } else {
                (Severity::Warning, "W_UNVERIFIED_ARGUMENT")
            };
            diagnostics.push(diagnostic(
                source,
                operand.span,
                severity,
                code,
                "Lamlet cannot verify this command operand; use @unchecked{EXPR} to acknowledge it",
            ));
        }
    }
    if !syntax.unambiguous {
        diagnostics.push(diagnostic(
            source,
            syntax.line.span,
            Severity::Unknown,
            "U_OPAQUE_COMMAND",
            "Runtime substitution or ambiguous token boundaries prevent command analysis",
        ));
        state.weaken();
        return;
    }
    if base_schema.is_none() {
        diagnostics.push(diagnostic(
            source,
            syntax.line.span,
            Severity::Unknown,
            "U_OPAQUE_COMMAND",
            format!("'{name}' is not verified by Lamlet; tracked state becomes unknown"),
        ));
        state.weaken();
        return;
    }
    if !complete_prefix
        || schema.is_none()
        || matches!(
            name,
            "compute" | "group" | "atom_style" | "read_data" | "read_restart" | "create_box"
        )
        || (name == "run" && args.len() > 2)
    {
        diagnostics.push(diagnostic(
            source,
            syntax.line.span,
            Severity::Unknown,
            "U_PARTIAL_COMMAND",
            "Only the documented subset of this command is verified",
        ));
    }
    match name {
        "units" | "atom_style" => {
            match state.box_exists {
                Fact::Present => diagnostics.push(diagnostic(
                    source,
                    first.span,
                    Severity::Error,
                    "E_COMMAND_ORDER",
                    format!("{name} cannot be changed after a simulation box exists"),
                )),
                Fact::Unknown => diagnostics.push(diagnostic(
                    source,
                    first.span,
                    Severity::Unknown,
                    "U_COMMAND_ORDER",
                    format!("Cannot prove that {name} appears before box creation"),
                )),
                Fact::Absent => {}
            }
            if name == "units" {
                if [
                    "lj", "real", "metal", "si", "cgs", "electron", "micro", "nano",
                ]
                .contains(&args[1].text.as_str())
                {
                    state.units = Some(args[1].text.clone());
                } else {
                    state.units = None;
                    diagnostics.push(diagnostic(
                        source,
                        args[1].span,
                        Severity::Unknown,
                        "U_UNIT_STYLE",
                        "Unit style is not recognized by Lamlet",
                    ));
                }
            } else {
                state.atom_style = Some(
                    args[1..]
                        .iter()
                        .map(|a| a.text.as_str())
                        .collect::<Vec<_>>()
                        .join(" "),
                );
            }
        }
        "read_data" => {
            state.box_exists = Fact::Present;
            state.groups.weaken();
            state.groups.set("all", Fact::Present);
            state.fixes.weaken();
        }
        "read_restart" => {
            state.weaken();
            state.box_exists = Fact::Present;
        }
        "create_box" => state.box_exists = Fact::Present,
        "group" => {
            let fact = match args[2].text.as_str() {
                "delete" => Fact::Absent,
                "clear" | "static" => state.groups.get(&args[1].text),
                "empty" | "region" | "type" | "id" | "molecule" | "variable" | "include"
                | "subtract" | "union" | "intersect" | "dynamic" => Fact::Present,
                _ => Fact::Unknown,
            };
            state.groups.set(&args[1].text, fact);
        }
        "fix" => {
            reference(
                source,
                args[2].span,
                &state.groups,
                &args[2].text,
                "group",
                diagnostics,
            );
            if schema.is_none() {
                state.weaken();
            }
            if matches!(args[3].text.as_str(), "nvt" | "npt") {
                // Thermostats and barostats create internal compute IDs.
                state.computes.weaken();
            }
            state.fixes.set(&args[1].text, Fact::Present);
        }
        "compute" => {
            state.weaken();
            state.computes.set(&args[1].text, Fact::Present);
        }
        "unfix" => {
            reference(
                source,
                args[1].span,
                &state.fixes,
                &args[1].text,
                "fix",
                diagnostics,
            );
            state.fixes.set(&args[1].text, Fact::Absent);
        }
        "uncompute" => {
            reference(
                source,
                args[1].span,
                &state.computes,
                &args[1].text,
                "compute",
                diagnostics,
            );
            state.computes.set(&args[1].text, Fact::Absent);
        }
        "run" if args.len() > 2 || state.opaque_callbacks => state.weaken(),
        _ => {}
    }
}
