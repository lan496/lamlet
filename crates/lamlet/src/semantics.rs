//! Conditional physical rules over known facts; this layer never rewrites commands.
use crate::{
    diagnostic::{Diagnostic, Severity},
    source::{LocatedSpan, Source, Span},
    syntax::Command,
};
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Satisfied,
    Violated,
    Unknown,
}

#[derive(Debug, Serialize)]
pub struct Rule {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub applies_to: &'static [&'static str],
    pub confidence: &'static str,
    pub autofix: bool,
    pub reference_version: &'static str,
    pub references: &'static [&'static str],
    pub limitations: &'static str,
}
pub const TEMPERATURE_CONSISTENCY: Rule = Rule {
    id: "LML-PHYS-002",
    title: "Temperature definition consistency",
    category: "physical-consistency",
    applies_to: &[
        "fix:nvt",
        "fix:npt",
        "compute:temp",
        "compute:temp/com",
        "fix_modify:temp",
        "thermo_modify:temp",
    ],
    confidence: "conditional",
    autofix: false,
    reference_version: "LAMMPS documentation 30Sep2026",
    references: &[
        "https://docs.lammps.org/compute_temp.html",
        "https://docs.lammps.org/compute_temp_com.html",
        "https://docs.lammps.org/fix_nh.html",
        "https://docs.lammps.org/thermo_modify.html",
    ],
    limitations: "Evaluated at plain run commands for known nvt/npt and displayed thermo temp; group identity and unmodified temp/temp/com definitions only. Unknown is not an error. No physical model is rewritten.",
};
pub const DYNAMIC_DOF: Rule = Rule {
    id: "LML-PHYS-010",
    title: "Temperature normalization when atom counts change",
    category: "physical-consistency",
    applies_to: &[
        "compute:temp",
        "compute:temp/com",
        "compute_modify:dynamic/dof",
        "fix:deposit",
        "fix:pour",
        "fix:gcmc",
        "fix:evaporate",
        "run",
    ],
    confidence: "conditional",
    autofix: false,
    reference_version: "LAMMPS documentation 30Sep2026",
    references: &[
        "https://docs.lammps.org/compute_modify.html",
        "https://docs.lammps.org/compute_temp.html",
        "https://docs.lammps.org/fix_deposit.html",
        "https://docs.lammps.org/fix_evaporate.html",
    ],
    limitations: "Positive literal run with active deposit/pour/gcmc/evaporate and selected thermo or nvt/npt temperature. Known temp/temp/com default dynamic/dof no and literal modifications only. Atom exchange and group membership may be conditional; no actual count change is proven. Different groups, unknown compute definitions/options and opaque effects are Unknown. No correction is inserted.",
};
pub static RULES: &[Rule] = &[TEMPERATURE_CONSISTENCY, DYNAMIC_DOF];

#[derive(Clone, Debug, Serialize)]
pub struct Evidence {
    pub subject: String,
    pub definition: String,
    pub span: LocatedSpan,
}
#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub rule_id: &'static str,
    pub outcome: Outcome,
    pub span: LocatedSpan,
    pub evidence: Vec<Evidence>,
    pub suggested_actions: Vec<&'static str>,
}
#[derive(Clone)]
struct Temperature {
    group: String,
    style: String,
    span: Span,
    unmodified: bool,
    dynamic_dof: Option<bool>,
    dof_span: Span,
}
struct Thermostat {
    temperature: Option<String>,
    span: Span,
}
struct AtomExchange {
    group: String,
    style: String,
    span: Span,
}

pub(crate) struct Engine {
    computes: BTreeMap<String, Temperature>,
    thermostats: BTreeMap<String, Thermostat>,
    exchanges: BTreeMap<String, AtomExchange>,
    thermo: Option<String>,
    thermo_computes: BTreeMap<String, Span>,
    displays_temp: Option<bool>,
    pub findings: Vec<Finding>,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            computes: BTreeMap::from([(
                "thermo_temp".into(),
                Temperature {
                    group: "all".into(),
                    style: "temp".into(),
                    span: Span::default(),
                    unmodified: true,
                    dynamic_dof: Some(false),
                    dof_span: Span::default(),
                },
            )]),
            thermostats: BTreeMap::new(),
            exchanges: BTreeMap::new(),
            thermo: Some("thermo_temp".into()),
            thermo_computes: BTreeMap::new(),
            displays_temp: Some(true),
            findings: vec![],
        }
    }
}
impl Engine {
    fn weaken(&mut self) {
        self.computes.clear();
        self.exchanges.clear();
        self.thermo = None;
        self.thermo_computes.clear();
        self.displays_temp = None;
        for fix in self.thermostats.values_mut() {
            fix.temperature = None;
        }
    }
    pub fn command(
        &mut self,
        source: &Source,
        command: &Command,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let args = &command.arguments;
        let Some(first) = args.first() else {
            return;
        };
        if !command.unambiguous {
            self.weaken();
            return;
        }
        match first.text.as_str() {
            "clear" => {
                let findings = std::mem::take(&mut self.findings);
                *self = Self::default();
                self.findings = findings;
            }
            "compute" if args.len() >= 4 => {
                let id = &args[1].text;
                if args.len() == 4 && matches!(args[3].text.as_str(), "temp" | "temp/com") {
                    self.computes.insert(
                        id.clone(),
                        Temperature {
                            group: args[2].text.clone(),
                            style: args[3].text.clone(),
                            span: command.line.span,
                            unmodified: true,
                            dynamic_dof: Some(false),
                            dof_span: command.line.span,
                        },
                    );
                } else {
                    self.computes.remove(id);
                }
            }
            "uncompute" if args.len() == 2 => {
                self.computes.remove(&args[1].text);
            }
            "compute_modify" if args.len() >= 2 => {
                if let Some(temperature) = self.computes.get_mut(&args[1].text) {
                    // The bias-comparison rule still requires an unmodified definition.
                    temperature.unmodified = false;
                    temperature.dof_span = command.line.span;
                    let options = &args[2..];
                    if options.is_empty() || options.len() % 2 != 0 {
                        temperature.dynamic_dof = None;
                    } else {
                        for pair in options.chunks_exact(2) {
                            match pair[0].text.as_str() {
                                "dynamic/dof" => {
                                    temperature.dynamic_dof = match pair[1].text.as_str() {
                                        "yes" => Some(true),
                                        "no" => Some(false),
                                        _ => None,
                                    }
                                }
                                "extra/dof"
                                    if pair[1].text.parse::<f64>().is_ok_and(f64::is_finite) => {}
                                _ => {
                                    temperature.dynamic_dof = None;
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            "fix" if args.len() >= 4 => {
                let id = &args[1].text;
                if matches!(
                    args[3].text.as_str(),
                    "deposit" | "pour" | "gcmc" | "evaporate"
                ) {
                    self.exchanges.insert(
                        id.clone(),
                        AtomExchange {
                            group: args[2].text.clone(),
                            style: args[3].text.clone(),
                            span: command.line.span,
                        },
                    );
                } else {
                    self.exchanges.remove(id);
                }
                if matches!(args[3].text.as_str(), "nvt" | "npt") {
                    let temperature = format!("{id}_temp");
                    self.computes.insert(
                        temperature.clone(),
                        Temperature {
                            group: args[2].text.clone(),
                            style: "temp".into(),
                            span: command.line.span,
                            unmodified: true,
                            dynamic_dof: Some(false),
                            dof_span: command.line.span,
                        },
                    );
                    self.thermostats.insert(
                        id.clone(),
                        Thermostat {
                            temperature: Some(temperature),
                            span: command.line.span,
                        },
                    );
                } else {
                    self.thermostats.remove(id);
                }
            }
            "unfix" if args.len() == 2 => {
                self.exchanges.remove(&args[1].text);
                self.thermostats.remove(&args[1].text);
                self.computes.remove(&format!("{}_temp", args[1].text));
            }
            "fix_modify" if args.len() >= 2 => {
                if let Some(fix) = self.thermostats.get_mut(&args[1].text) {
                    if args.len() == 4 && args[2].text == "temp" {
                        fix.temperature = Some(args[3].text.clone());
                        fix.span = command.line.span;
                    } else {
                        fix.temperature = None;
                    }
                }
            }
            "thermo_modify" => {
                self.thermo = if args.len() == 3 && args[1].text == "temp" {
                    Some(args[2].text.clone())
                } else {
                    None
                };
            }
            "thermo_style" if args.len() >= 2 => {
                self.thermo = Some("thermo_temp".into());
                self.thermo_computes.clear();
                if args[1].text == "custom" {
                    for arg in &args[2..] {
                        if let Some(id) = arg
                            .text
                            .strip_prefix("c_")
                            .filter(|id| !id.contains(['[', ']']))
                        {
                            self.thermo_computes.insert(id.into(), arg.span);
                        }
                    }
                }
                self.displays_temp = match args[1].text.as_str() {
                    "one" | "multi" | "yaml" => Some(true),
                    "custom" => Some(args[2..].iter().any(|a| a.text == "temp")),
                    _ => None,
                };
            }
            "run" if args.len() == 2 => {
                self.evaluate(source, command.line.span, diagnostics);
                if args[1].text.parse::<u64>().is_ok_and(|steps| steps > 0) {
                    self.evaluate_dynamic_dof(source, command.line.span, diagnostics);
                }
            }
            "units" | "atom_style" | "timestep" | "thermo" | "dump" | "undump" | "dump_modify"
            | "print" | "label" | "variable" | "log" | "write_data" | "write_restart" => {}
            _ => self.weaken(),
        }
    }
    fn evidence(&self, source: &Source, subject: String, id: Option<&str>, span: Span) -> Evidence {
        let definition = id
            .and_then(|id| self.computes.get(id))
            .filter(|t| t.unmodified);
        Evidence {
            subject,
            definition: match (id, definition) {
                (Some(id), Some(t)) => {
                    format!("compute {id}: {}, group {}", t.style, t.group)
                }
                (Some(id), None) => format!("compute {id}: unknown definition"),
                _ => "unknown temperature selection".into(),
            },
            span: source.located(definition.map_or(span, |t| t.span)),
        }
    }
    fn evaluate(&mut self, source: &Source, span: Span, diagnostics: &mut Vec<Diagnostic>) {
        for (id, fix) in &self.thermostats {
            let a = fix
                .temperature
                .as_ref()
                .and_then(|id| self.computes.get(id));
            let b = self.thermo.as_ref().and_then(|id| self.computes.get(id));
            let outcome = match (a, b, self.displays_temp) {
                (Some(a), Some(b), Some(true))
                    if a.group == b.group && a.unmodified && b.unmodified =>
                {
                    if a.style == b.style {
                        Outcome::Satisfied
                    } else {
                        Outcome::Violated
                    }
                }
                _ => Outcome::Unknown,
            };
            let actions = if outcome == Outcome::Violated {
                vec![
                    "Use the same temperature compute via fix_modify temp and thermo_modify temp",
                    "Keep both definitions after reviewing the intended velocity bias",
                ]
            } else {
                vec![]
            };
            let evidence = vec![
                self.evidence(
                    source,
                    format!("fix {id}"),
                    fix.temperature.as_deref(),
                    fix.span,
                ),
                self.evidence(source, "thermo temp".into(), self.thermo.as_deref(), span),
            ];
            if outcome == Outcome::Violated {
                let mut diagnostic = Diagnostic::new(
                    source,
                    span,
                    Severity::Warning,
                    TEMPERATURE_CONSISTENCY.id,
                    "Thermostat and thermo output use different known temperature definitions; this may be intentional",
                );
                diagnostic.evidence = evidence.clone();
                diagnostic.suggested_actions = actions.iter().map(|s| (*s).into()).collect();
                diagnostics.push(diagnostic);
            }
            self.findings.push(Finding {
                rule_id: TEMPERATURE_CONSISTENCY.id,
                outcome,
                span: source.located(span),
                evidence,
                suggested_actions: actions,
            });
        }
    }

    fn evaluate_dynamic_dof(
        &mut self,
        source: &Source,
        span: Span,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut selected: BTreeMap<Option<&str>, Vec<(String, Span)>> = BTreeMap::new();
        if self.displays_temp == Some(true) {
            selected
                .entry(self.thermo.as_deref())
                .or_default()
                .push(("thermo temp".into(), span));
        }
        for (id, at) in &self.thermo_computes {
            // Other scalar computes do not normalize temperature.
            if self.computes.contains_key(id) {
                selected
                    .entry(Some(id.as_str()))
                    .or_default()
                    .push((format!("thermo c_{id}"), *at));
            }
        }
        for (id, thermostat) in &self.thermostats {
            selected
                .entry(thermostat.temperature.as_deref())
                .or_default()
                .push((format!("fix {id} temperature selection"), thermostat.span));
        }
        for (exchange_id, exchange) in &self.exchanges {
            for (id, uses) in &selected {
                let temperature = id.and_then(|id| self.computes.get(id));
                let outcome = match temperature {
                    Some(t) if t.dynamic_dof == Some(true) => Outcome::Satisfied,
                    Some(t)
                        if t.dynamic_dof == Some(false)
                            && (t.group == exchange.group
                                || t.group == "all"
                                || exchange.group == "all") =>
                    {
                        Outcome::Violated
                    }
                    _ => Outcome::Unknown,
                };
                let mut evidence = vec![
                    Evidence {
                        subject: format!("fix {exchange_id} atom exchange"),
                        definition: format!(
                            "{}, group {}; atom count may change during the run",
                            exchange.style, exchange.group
                        ),
                        span: source.located(exchange.span),
                    },
                    Evidence {
                        subject: format!("compute {} normalization", id.unwrap_or("<unknown>")),
                        definition: temperature.map_or_else(
                            || "unknown temperature definition".into(),
                            |t| {
                                format!(
                                    "{}, group {}; dynamic/dof {}",
                                    t.style,
                                    t.group,
                                    match t.dynamic_dof {
                                        Some(true) => "yes",
                                        Some(false) => "no",
                                        None => "unknown",
                                    }
                                )
                            },
                        ),
                        span: source.located(temperature.map_or(span, |t| t.dof_span)),
                    },
                ];
                evidence.extend(uses.iter().map(|(subject, at)| Evidence {
                    subject: subject.clone(),
                    definition: format!("compute {}", id.unwrap_or("<unknown>")),
                    span: source.located(*at),
                }));
                let actions = if outcome == Outcome::Violated {
                    vec![
                        "Review whether atom exchange changes this temperature group's population",
                        "Use compute_modify on the selected temperature with dynamic/dof yes if its atom count changes",
                    ]
                } else {
                    vec![]
                };
                if outcome == Outcome::Violated {
                    let mut diagnostic = Diagnostic::new(
                        source,
                        span,
                        Severity::Warning,
                        DYNAMIC_DOF.id,
                        "An active atom-exchange fix may change the population of a selected temperature compute with dynamic/dof no",
                    );
                    diagnostic.evidence = evidence.clone();
                    diagnostic.suggested_actions =
                        actions.iter().map(|action| (*action).into()).collect();
                    diagnostics.push(diagnostic);
                }
                self.findings.push(Finding {
                    rule_id: DYNAMIC_DOF.id,
                    outcome,
                    span: source.located(span),
                    evidence,
                    suggested_actions: actions,
                });
            }
        }
    }
}
