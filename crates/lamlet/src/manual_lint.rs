//! Executable, reviewed constraints with immutable manual citations.
use crate::{
    diagnostic::{Diagnostic, Severity},
    semantics::{Evidence, Finding, Outcome},
    source::{Source, Span},
    syntax::Command,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub revision: String,
    pub version: String,
    pub commands: Vec<String>,
    pub review: ReviewSummary,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewSummary {
    pub candidate_paragraphs: usize,
    pub reviewed_paragraphs: usize,
    pub pending_paragraphs: usize,
    pub partial_paragraphs: usize,
    pub deferred_paragraphs: usize,
    pub complete: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    pub page: String,
    pub sha256: String,
    pub start_line: usize,
    pub end_line: usize,
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Example {
    pub input: String,
    pub outcome: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub message: String,
    pub selectors: Vec<Selector>,
    pub predicate: Predicate,
    pub severity: String,
    pub autofix: bool,
    pub source: Citation,
    pub examples: Vec<Example>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selector {
    pub command: String,
    pub position: Option<usize>,
    pub value: Option<String>,
    pub exact_arguments: Option<usize>,
    pub active_pair_style: Option<String>,
}

impl Selector {
    fn matches(&self, command: &Command, pair_style: Option<&str>) -> bool {
        if self
            .active_pair_style
            .as_deref()
            .is_some_and(|style| Some(style) != pair_style)
        {
            return false;
        }
        command
            .arguments
            .first()
            .is_some_and(|a| a.text == self.command && !a.dynamic)
            && self
                .exact_arguments
                .is_none_or(|n| n == command.arguments.len())
            && self.position.is_none_or(|i| {
                command.arguments.get(i).is_some_and(|a| {
                    !a.dynamic && self.value.as_ref().is_some_and(|v| a.text == *v)
                })
            })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    Number {
        position: usize,
        minimum: f64,
        maximum: Option<f64>,
        integer: bool,
        exclusive: bool,
    },
    OneOf {
        position: usize,
        values: Vec<String>,
    },
    Contains {
        position: usize,
        value: String,
    },
    Sampling {
        positions: [usize; 3],
    },
    BeforeBox,
    AfterBox,
    Dimension {
        expected: u8,
    },
    BoundaryFaces,
    NeighborDelay,
    NeighborPage,
    PairShiftTail,
    PairTailDimension,
    ResetDump,
    ResetTimeFix,
    ReadDataAdd,
    ReadDataOptions,
    DimensionalArgument {
        position: usize,
        dimension: u8,
        expected: i64,
    },
    StaticGroup,
    CommMulti,
    DiscDimension,
    Units {
        allowed: Vec<String>,
    },
    PairUnsupported {
        option: String,
    },
}

pub fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../data/manual_rules.json"))
            .expect("checked-in manual rule catalog must be valid")
    })
}

fn literal(command: &Command, position: usize) -> Option<&str> {
    command
        .arguments
        .get(position)
        .filter(|a| !a.dynamic && !a.text.contains('$'))
        .map(|a| a.text.as_str())
}

fn number(command: &Command, position: usize) -> Option<f64> {
    literal(command, position)?.parse().ok()
}

fn decide(value: Option<bool>) -> Outcome {
    match value {
        Some(true) => Outcome::Satisfied,
        Some(false) => Outcome::Violated,
        None => Outcome::Unknown,
    }
}

pub(crate) struct Engine {
    box_exists: Option<bool>,
    dimension: Option<u8>,
    units: Option<String>,
    neighbors: [Option<i64>; 4], // delay, every, page, one
    pair: [Option<bool>; 2],     // shift, tail
    pair_style: Option<String>,
    comm: Option<String>,
    dumps: BTreeMap<String, Span>,
    fixes: BTreeMap<String, (String, Span)>,
    inventory_known: bool,
    groups: BTreeMap<String, bool>, // dynamic membership
    state_sources: BTreeMap<&'static str, Span>,
    pub findings: Vec<Finding>,
}

impl Default for Engine {
    fn default() -> Self {
        Self {
            box_exists: Some(false),
            dimension: Some(3),
            units: Some("lj".into()),
            neighbors: [Some(0), Some(1), Some(100000), Some(2000)],
            pair: [Some(false); 2],
            pair_style: Some("none".into()),
            comm: Some("brick".into()),
            dumps: BTreeMap::new(),
            fixes: BTreeMap::new(),
            inventory_known: true,
            groups: BTreeMap::from([("all".into(), false)]),
            state_sources: BTreeMap::new(),
            findings: vec![],
        }
    }
}

impl Engine {
    fn weaken(&mut self) {
        self.box_exists = None;
        self.dimension = None;
        self.units = None;
        self.neighbors = [None; 4];
        self.pair = [None; 2];
        self.pair_style = None;
        self.comm = None;
        self.dumps.clear();
        self.fixes.clear();
        self.inventory_known = false;
        self.groups.clear();
        self.state_sources.clear();
    }

    fn evaluate(&self, command: &Command, predicate: &Predicate) -> Outcome {
        if !command.unambiguous
            || command
                .arguments
                .iter()
                .any(|a| a.dynamic || a.text.contains('$'))
        {
            return Outcome::Unknown;
        }
        let result = match predicate {
            Predicate::BeforeBox => self.box_exists.map(|v| !v),
            Predicate::AfterBox => self.box_exists,
            Predicate::Dimension { expected } => self.dimension.map(|d| d == *expected),
            Predicate::Number {
                position,
                minimum,
                maximum,
                integer,
                exclusive,
            } => number(command, *position).map(|n| {
                n.is_finite()
                    && (!integer
                        || literal(command, *position).is_some_and(|s| s.parse::<i64>().is_ok()))
                    && if *exclusive {
                        n > *minimum
                    } else {
                        n >= *minimum
                    }
                    && maximum.is_none_or(|max| n <= max)
            }),
            Predicate::OneOf { position, values } => {
                literal(command, *position).map(|v| values.iter().any(|s| v == s))
            }
            Predicate::Contains { position, value } => {
                literal(command, *position).map(|v| v.contains(value))
            }
            Predicate::Sampling { positions } => {
                let values: Option<Vec<_>> = positions
                    .iter()
                    .map(|i| literal(command, *i)?.parse::<i64>().ok())
                    .collect();
                values.map(|n| {
                    n.iter().all(|v| *v > 0)
                        && n[2] % n[0] == 0
                        && n[0].checked_mul(n[1]).is_some_and(|window| window <= n[2])
                })
            }
            Predicate::BoundaryFaces => {
                let values: Option<Vec<_>> = (1..=3).map(|i| literal(command, i)).collect();
                values.and_then(|values| {
                    values
                        .iter()
                        .all(|v| {
                            (1..=2).contains(&v.len()) && v.bytes().all(|b| b"pfsm".contains(&b))
                        })
                        .then(|| {
                            values
                                .iter()
                                .all(|v| !v.contains('p') || matches!(*v, "p" | "pp"))
                        })
                })
            }
            Predicate::NeighborDelay => {
                let n = self.neighbor_settings(command);
                n[0].zip(n[1])
                    .map(|(delay, every)| delay >= 0 && every > 0 && delay % every == 0)
            }
            Predicate::NeighborPage => {
                let n = self.neighbor_settings(command);
                n[2].zip(n[3]).map(|(page, one)| {
                    page > 0
                        && one > 0
                        && one.checked_mul(10).is_some_and(|minimum| page >= minimum)
                })
            }
            Predicate::PairShiftTail => {
                let pair = self.pair_settings(command);
                pair[0].zip(pair[1]).map(|(shift, tail)| !(shift && tail))
            }
            Predicate::PairTailDimension => self.pair_settings(command)[1]
                .zip(self.dimension)
                .map(|(tail, dimension)| !tail || dimension == 3),
            Predicate::ResetDump => {
                if self.dumps.is_empty() {
                    self.inventory_known.then_some(true)
                } else {
                    Some(false)
                }
            }
            Predicate::ResetTimeFix => {
                if self.fixes.values().any(|(style, _)| {
                    matches!(style.as_str(), "deposit" | "dt/reset" | "dt/reset/kk")
                }) {
                    Some(false)
                } else {
                    self.inventory_known.then_some(true)
                }
            }
            Predicate::ReadDataAdd => read_data_options(command)
                .and_then(|options| self.box_exists.map(|exists| exists == options.add)),
            Predicate::ReadDataOptions => {
                read_data_options(command).map(|options| options.add || !options.needs_add)
            }
            Predicate::DimensionalArgument {
                position,
                dimension,
                expected,
            } => self.dimension.and_then(|d| {
                if d != *dimension {
                    Some(true)
                } else {
                    literal(command, *position)
                        .and_then(|s| s.parse::<i64>().ok())
                        .map(|n| n == *expected)
                }
            }),
            Predicate::StaticGroup => literal(command, 2)
                .and_then(|id| self.groups.get(id))
                .map(|dynamic| !dynamic),
            Predicate::CommMulti => simple_options(
                command,
                1,
                &["mode", "cutoff", "vel", "group", "reduce/multi"],
            )
            .and_then(|options| {
                options
                    .iter()
                    .find(|(key, _)| *key == "mode")
                    .map(|(_, value)| *value)
            })
            .and_then(|mode| {
                if mode != "multi" {
                    Some(true)
                } else {
                    self.comm.as_ref().map(|c| c == "brick")
                }
            }),
            Predicate::DiscDimension => {
                let mut i = 4;
                let mut disc = false;
                while i < command.arguments.len() {
                    match literal(command, i) {
                        Some("disc") => {
                            disc = true;
                            i += 1;
                        }
                        Some("update") if literal(command, i + 1).is_some() => i += 2,
                        _ => return Outcome::Unknown,
                    }
                }
                if disc {
                    self.dimension.map(|d| d == 2)
                } else {
                    Some(true)
                }
            }
            Predicate::Units { allowed } => {
                self.units.as_ref().map(|units| allowed.contains(units))
            }
            Predicate::PairUnsupported { option } => simple_options(command, 1, PAIR_OPTIONS)
                .and_then(|options| {
                    let value = options
                        .iter()
                        .filter(|(key, _)| *key == option)
                        .map(|(_, v)| *v)
                        .next_back();
                    match (option.as_str(), value) {
                        (_, None) | ("shift" | "tail", Some("no")) => Some(true),
                        ("shift" | "tail", Some("yes")) => Some(false),
                        ("table", Some(value)) => value.parse::<i64>().ok().map(|n| n == 0),
                        _ => None,
                    }
                }),
        };
        decide(result)
    }

    pub fn command(
        &mut self,
        source: &Source,
        command: &Command,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        for rule in &catalog().rules {
            if !rule
                .selectors
                .iter()
                .any(|s| s.matches(command, self.pair_style.as_deref()))
            {
                continue;
            }
            let outcome = self.evaluate(command, &rule.predicate);
            let mut evidence = vec![Evidence {
                subject: "LAMMPS command".into(),
                definition: command
                    .arguments
                    .iter()
                    .map(|a| a.text.as_str())
                    .collect::<Vec<_>>()
                    .join(" "),
                span: source.located(command.line.span),
            }];
            let keys: &[&str] = match rule.predicate {
                Predicate::BeforeBox | Predicate::AfterBox | Predicate::ReadDataAdd => &["box"],
                Predicate::Dimension { .. }
                | Predicate::DimensionalArgument { .. }
                | Predicate::DiscDimension => &["dimension"],
                Predicate::NeighborDelay => &["neighbor_delay", "neighbor_every"],
                Predicate::NeighborPage => &["neighbor_page", "neighbor_one"],
                Predicate::PairShiftTail => &["pair_shift", "pair_tail", "pair_style"],
                Predicate::PairTailDimension => &["pair_tail", "pair_style", "dimension"],
                Predicate::StaticGroup => &["groups"],
                Predicate::CommMulti => &["comm"],
                Predicate::Units { .. } => &["units"],
                Predicate::PairUnsupported { .. } => &["pair_style"],
                _ => &[],
            };
            for key in keys {
                if let Some(span) = self.state_sources.get(key) {
                    evidence.push(Evidence {
                        subject: (*key).into(),
                        definition: String::from_utf8_lossy(&source.bytes[span.start..span.end])
                            .trim()
                            .into(),
                        span: source.located(*span),
                    });
                }
            }
            let declarations: Vec<(&str, Span)> = match rule.predicate {
                Predicate::ResetDump => self
                    .dumps
                    .iter()
                    .map(|(id, span)| (id.as_str(), *span))
                    .collect(),
                Predicate::ResetTimeFix => self
                    .fixes
                    .iter()
                    .filter(|(_, (style, _))| {
                        matches!(style.as_str(), "deposit" | "dt/reset" | "dt/reset/kk")
                    })
                    .map(|(id, (_, span))| (id.as_str(), *span))
                    .collect(),
                _ => vec![],
            };
            for (id, span) in declarations {
                evidence.push(Evidence {
                    subject: id.into(),
                    definition: String::from_utf8_lossy(&source.bytes[span.start..span.end])
                        .trim()
                        .into(),
                    span: source.located(span),
                });
            }
            let actions = if outcome == Outcome::Violated {
                vec!["Review the cited manual constraint and the intended input before changing it"]
            } else {
                vec![]
            };
            if outcome == Outcome::Violated {
                let mut diagnostic = Diagnostic::new(
                    source,
                    command.line.span,
                    Severity::Warning,
                    &rule.id,
                    rule.message.clone(),
                );
                diagnostic
                    .notes
                    .push(format!("LAMMPS {}: {}", catalog().version, rule.source.url));
                diagnostic.evidence = evidence.clone();
                diagnostic.suggested_actions = actions.iter().map(|s| (*s).into()).collect();
                diagnostics.push(diagnostic);
            }
            self.findings.push(Finding {
                rule_id: rule.id.as_str(),
                outcome,
                span: source.located(command.line.span),
                evidence,
                suggested_actions: actions,
            });
        }
        self.update(command);
    }

    fn update(&mut self, command: &Command) {
        if !command.unambiguous
            || command
                .arguments
                .iter()
                .any(|a| a.dynamic || a.text.contains('$'))
        {
            self.weaken();
            return;
        }
        match literal(command, 0) {
            Some("clear") => {
                let findings = std::mem::take(&mut self.findings);
                *self = Self::default();
                self.findings = findings;
            }
            Some("dimension") => {
                self.dimension = literal(command, 1)
                    .and_then(|s| s.parse().ok())
                    .filter(|d| matches!(d, 2 | 3));
                self.state_sources.insert("dimension", command.line.span);
            }
            Some("units") => {
                self.units = literal(command, 1).map(str::to_owned);
                self.state_sources.insert("units", command.line.span);
            }
            Some("create_box") => {
                self.box_exists = Some(true);
                self.state_sources.insert("box", command.line.span);
            }
            Some("read_data") => {
                if read_data_options(command).is_none_or(|o| o.callback) {
                    self.weaken();
                } else {
                    self.box_exists = Some(true);
                    self.state_sources.insert("box", command.line.span);
                    self.pair = [None; 2];
                    self.pair_style = None;
                }
            }
            Some("read_restart") => {
                self.weaken();
                self.box_exists = Some(true);
                self.state_sources.insert("box", command.line.span);
            }
            Some("neigh_modify") => {
                self.neighbors = self.neighbor_settings(command);
                if let Some(options) = simple_options(command, 1, NEIGHBOR_OPTIONS) {
                    for (key, _) in options {
                        let field = match key {
                            "delay" => "neighbor_delay",
                            "every" => "neighbor_every",
                            "page" => "neighbor_page",
                            "one" => "neighbor_one",
                            _ => continue,
                        };
                        self.state_sources.insert(field, command.line.span);
                    }
                }
            }
            Some("pair_modify") => {
                self.pair = self.pair_settings(command);
                if let Some(options) = simple_options(command, 1, PAIR_OPTIONS) {
                    for (key, _) in options {
                        let field = match key {
                            "shift" => "pair_shift",
                            "tail" => "pair_tail",
                            _ => continue,
                        };
                        self.state_sources.insert(field, command.line.span);
                    }
                }
            }
            Some("pair_style") => {
                let next = literal(command, 1);
                if self.pair_style.as_deref() != next
                    || next.is_some_and(|s| s.starts_with("hybrid"))
                {
                    self.pair = if self.pair_style.is_some()
                        && next.is_some_and(|s| !s.starts_with("hybrid"))
                    {
                        [Some(false); 2]
                    } else {
                        [None; 2]
                    };
                    self.state_sources.remove("pair_shift");
                    self.state_sources.remove("pair_tail");
                }
                self.pair_style = next.map(str::to_owned);
                self.state_sources.insert("pair_style", command.line.span);
            }
            Some("comm_style") => {
                self.comm = literal(command, 1).map(str::to_owned);
                self.state_sources.insert("comm", command.line.span);
            }
            Some("dump") if command.arguments.len() >= 6 => {
                if let Some(id) = literal(command, 1) {
                    self.dumps.insert(id.into(), command.line.span);
                }
            }
            Some("undump") => {
                if let Some(id) = literal(command, 1) {
                    self.dumps.remove(id);
                } else {
                    self.dumps.clear();
                    self.inventory_known = false;
                }
            }
            Some("fix") if command.arguments.len() >= 4 => {
                if let (Some(id), Some(style)) = (literal(command, 1), literal(command, 3)) {
                    if !known_fix(style) {
                        self.weaken();
                    }
                    self.fixes
                        .insert(id.into(), (style.into(), command.line.span));
                }
            }
            Some("unfix") => {
                if let Some(id) = literal(command, 1) {
                    self.fixes.remove(id);
                } else {
                    self.fixes.clear();
                    self.inventory_known = false;
                }
            }
            Some("group") => {
                if let (Some(id), Some(style)) = (literal(command, 1), literal(command, 2)) {
                    match style {
                        "dynamic" => {
                            self.groups.insert(id.into(), true);
                        }
                        "static" => {
                            self.groups.insert(id.into(), false);
                        }
                        "delete" => {
                            self.groups.remove(id);
                        }
                        _ => {
                            if self.inventory_known {
                                self.groups.entry(id.into()).or_insert(false);
                            }
                        }
                    }
                    self.state_sources.insert("groups", command.line.span);
                }
            }
            Some(
                "include" | "jump" | "if" | "python" | "shell" | "plugin" | "partition" | "rerun"
                | "temper" | "temper/npt" | "temper/grem" | "prd" | "tad" | "neb" | "hyper",
            ) => self.weaken(),
            Some("variable")
                if literal(command, 2) == Some("python")
                    || command.arguments.iter().any(|a| a.text.contains("py_")) =>
            {
                self.weaken()
            }
            Some("run" | "minimize")
                if command.arguments.len()
                    > (if literal(command, 0) == Some("run") {
                        2
                    } else {
                        5
                    })
                    || self.fixes.values().any(|(style, _)| !known_fix(style)) =>
            {
                self.weaken()
            }
            Some(name)
                if catalog()
                    .commands
                    .binary_search_by(|c| c.as_str().cmp(name))
                    .is_ok() => {}
            _ => self.weaken(),
        }
    }

    fn neighbor_settings(&self, command: &Command) -> [Option<i64>; 4] {
        let Some(options) = simple_options(command, 1, NEIGHBOR_OPTIONS) else {
            return [None; 4];
        };
        let mut settings = self.neighbors;
        for (key, value) in options {
            if let Some(i) = ["delay", "every", "page", "one"]
                .iter()
                .position(|s| *s == key)
            {
                settings[i] = value.parse().ok();
            }
        }
        settings
    }

    fn pair_settings(&self, command: &Command) -> [Option<bool>; 2] {
        let Some(options) = simple_options(command, 1, PAIR_OPTIONS) else {
            return [None; 2];
        };
        let mut settings = self.pair;
        for (key, value) in options {
            if let Some(i) = ["shift", "tail"].iter().position(|s| *s == key) {
                settings[i] = match value {
                    "yes" => Some(true),
                    "no" => Some(false),
                    _ => None,
                };
            }
        }
        settings
    }
}

const NEIGHBOR_OPTIONS: &[&str] = &[
    "delay", "every", "page", "one", "check", "once", "cluster", "include", "binsize", "bin/hash",
];
const PAIR_OPTIONS: &[&str] = &[
    "shift",
    "tail",
    "mix",
    "table",
    "table/disp",
    "tabinner",
    "tabinner/disp",
    "compute",
    "compute/tally",
    "neigh/trim",
];

fn simple_options<'a>(
    command: &'a Command,
    start: usize,
    keys: &[&str],
) -> Option<Vec<(&'a str, &'a str)>> {
    let mut options = vec![];
    let mut i = start;
    while i < command.arguments.len() {
        let key = literal(command, i)?;
        if !keys.contains(&key) {
            return None;
        }
        options.push((key, literal(command, i + 1)?));
        i += 2;
    }
    Some(options)
}

struct DataOptions {
    add: bool,
    needs_add: bool,
    callback: bool,
}
fn read_data_options(command: &Command) -> Option<DataOptions> {
    let mut options = DataOptions {
        add: false,
        needs_add: false,
        callback: false,
    };
    let mut i = 2;
    while i < command.arguments.len() {
        let key = literal(command, i)?;
        let count = match key {
            "add" => {
                options.add = true;
                match literal(command, i + 1)? {
                    "append" | "merge" => 1,
                    value if value.parse::<i64>().is_ok() => {
                        if literal(command, i + 2).is_some_and(|s| s.parse::<i64>().is_ok()) {
                            2
                        } else {
                            1
                        }
                    }
                    _ => return None,
                }
            }
            "offset" => {
                options.needs_add = true;
                5
            }
            "shift" => {
                options.needs_add = true;
                3
            }
            "group"
            | "extra/atom/types"
            | "extra/bond/types"
            | "extra/angle/types"
            | "extra/dihedral/types"
            | "extra/improper/types"
            | "extra/bond/per/atom"
            | "extra/angle/per/atom"
            | "extra/dihedral/per/atom"
            | "extra/improper/per/atom"
            | "extra/special/per/atom" => 1,
            "nocoeff" => 0,
            "fix" => {
                options.callback = true;
                3
            }
            _ => return None,
        };
        if i + count >= command.arguments.len() {
            return None;
        }
        i += count + 1;
    }
    Some(options)
}

fn known_fix(style: &str) -> bool {
    let base = style
        .rsplit_once('/')
        .filter(|(_, suffix)| matches!(*suffix, "omp" | "kk" | "gpu" | "intel" | "opt"))
        .map_or(style, |(base, _)| base);
    matches!(
        base,
        "nve"
            | "nvt"
            | "npt"
            | "nph"
            | "langevin"
            | "momentum"
            | "setforce"
            | "shake"
            | "rattle"
            | "temp/rescale"
            | "temp/berendsen"
            | "ave/time"
            | "ave/atom"
            | "ave/chunk"
            | "ave/grid"
            | "ave/histo"
            | "deposit"
            | "dt/reset"
    )
}
