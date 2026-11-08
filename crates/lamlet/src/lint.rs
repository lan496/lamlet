//! Reviewed manual constraints. Findings never rewrite or reject legacy input.
use crate::{
    diagnostic::{Diagnostic, Severity},
    semantics::{Evidence, Finding, Outcome, Rule},
    source::{Source, Span},
    syntax::Command,
};
use std::collections::BTreeMap;

macro_rules! rule {
    ($id:literal, $title:literal, $category:literal, [$($applies:literal),+], [$($page:literal),+], $limits:literal) => {
        Rule {
            id: $id,
            title: $title,
            category: $category,
            applies_to: &[$($applies),+],
            confidence: "conditional",
            autofix: false,
            reference_version: "LAMMPS documentation 30Sep2026",
            references: &[$(concat!("https://docs.lammps.org/", $page, ".html")),+],
            limitations: $limits,
        }
    };
}

pub const INTEGRATION: Rule = rule!(
    "LML-PHYS-001",
    "Overlapping time integrators",
    "physical-consistency",
    ["fix:nve", "fix:nvt", "fix:npt", "fix:nph"],
    ["fix_nve", "fix_nh"],
    "At positive literal run commands; known integrators on the same group or all versus another group. Empty groups and intentional specialized schemes require review. Distinct group overlap is Unknown."
);
pub const THERMOSTATS: Rule = rule!(
    "LML-PHYS-003",
    "Multiple temperature controllers",
    "physical-consistency",
    [
        "fix:nvt",
        "fix:npt",
        "fix:langevin",
        "fix:temp/rescale",
        "fix:temp/berendsen"
    ],
    ["fix_nh", "fix_langevin"],
    "Same overlap limits as integrator checking. Reports a conditional warning, since multiple controls can be intentional."
);
pub const BAROSTAT: Rule = rule!(
    "LML-PHYS-004",
    "Barostat periodicity",
    "physical-consistency",
    ["boundary", "dimension", "fix:npt", "fix:nph"],
    ["fix_nh", "boundary"],
    "Known npt/nph keyword grammar and boundary settings only. A tilt component requires periodicity in its second axis. Two-dimensional iso/aniso omit z. Unknown options or runtime boundaries suppress conclusions."
);
pub const TWO_DIMENSIONAL: Rule = rule!(
    "LML-PHYS-005",
    "Periodic z boundary in two dimensions",
    "physical-consistency",
    ["dimension", "boundary", "create_box", "read_data"],
    ["boundary", "dimension"],
    "Evaluated at box creation with a known dimension and boundary; restart and opaque state are Unknown."
);
pub const THERMO_RESET: Rule = rule!(
    "LML-LINT-001",
    "Thermo style resets modified settings",
    "idiom",
    ["thermo_style", "thermo_modify"],
    ["thermo_style", "thermo_modify"],
    "Reports a reset when a known thermo_modify is followed by thermo_style. Resetting deliberately is valid; no automatic reordering is performed."
);
pub const RANDOM_SEED: Rule = rule!(
    "LML-LINT-002",
    "Positive literal random seed",
    "argument",
    ["velocity:create", "fix:langevin"],
    ["velocity", "fix_langevin"],
    "Only integer literal seeds in velocity create and fix langevin are checked; substitutions and other random generators remain unverified."
);
pub const REGION_COUNT: Rule = rule!(
    "LML-LINT-003",
    "Region combination needs multiple regions",
    "argument",
    ["region:union", "region:intersect"],
    ["region"],
    "Only a literal union/intersect count below two is diagnosed. Region membership, existence and dynamic geometry are not established."
);
pub const VELOCITY_REPLACE: Rule = rule!(
    "LML-LINT-004",
    "Velocity assignment replaces initialized motion",
    "idiom",
    ["velocity:create", "velocity:set"],
    ["velocity"],
    "A numeric velocity set on a group with a preceding thermal create, or create on all, replaces components unless sum yes is explicit. Replacement may be intended. Group changes and opaque commands invalidate this evidence."
);
pub const LANGEVIN_INTEGRATION: Rule = rule!(
    "LML-PHYS-006",
    "Langevin thermostat requires separate integration",
    "physical-consistency",
    ["fix:langevin", "run"],
    ["fix_langevin", "fix_nve"],
    "Only when every active fix has a reviewed role and no known integrator overlaps the Langevin group. Unknown fix styles, group overlap, or opaque effects prevent an absence claim. run 0 is excluded."
);
pub static RULES: &[Rule] = &[
    INTEGRATION,
    THERMOSTATS,
    BAROSTAT,
    TWO_DIMENSIONAL,
    THERMO_RESET,
    RANDOM_SEED,
    REGION_COUNT,
    VELOCITY_REPLACE,
    LANGEVIN_INTEGRATION,
];

#[derive(Clone)]
struct Fix {
    group: String,
    style: String,
    integrates: bool,
    thermostats: bool,
    known: bool,
    axes: Option<Vec<usize>>,
    span: Span,
}

pub(crate) struct Engine {
    fixes: BTreeMap<String, Fix>,
    fix_inventory_known: bool,
    dimension: Option<u8>,
    periodic: [Option<bool>; 3],
    thermo_modified: Option<Span>,
    thermal: BTreeMap<String, Span>,
    pub findings: Vec<Finding>,
}
impl Default for Engine {
    fn default() -> Self {
        Self {
            fixes: BTreeMap::new(),
            fix_inventory_known: true,
            dimension: Some(3),
            periodic: [Some(true); 3],
            thermo_modified: None,
            thermal: BTreeMap::new(),
            findings: vec![],
        }
    }
}

fn base_style(style: &str) -> &str {
    style
        .rsplit_once('/')
        .filter(|(_, suffix)| matches!(*suffix, "gpu" | "intel" | "kk" | "omp" | "opt"))
        .map_or(style, |(base, _)| base)
}

fn overlap(a: &str, b: &str) -> Option<bool> {
    if a == b || a == "all" || b == "all" {
        Some(true)
    } else {
        None
    }
}

fn pressure_axes(args: &[&str], dimension: Option<u8>) -> Option<Vec<usize>> {
    let mut axes = vec![];
    let mut i = 0;
    while i < args.len() {
        let (count, selected): (usize, &[usize]) = match args[i] {
            "temp" => (3, &[]),
            "iso" | "aniso" if dimension == Some(2) => (3, &[0, 1]),
            "iso" | "aniso" if dimension == Some(3) => (3, &[0, 1, 2]),
            "tri" => (3, &[0, 1, 2]),
            "x" => (3, &[0]),
            "y" | "xy" => (3, &[1]),
            "z" | "xz" | "yz" => (3, &[2]),
            "fixedpoint" => (3, &[]),
            "couple" | "tchain" | "pchain" | "mtk" | "tloop" | "ploop" | "nreset" | "drag"
            | "ptemp" | "dilate" | "scalexy" | "scalexz" | "scaleyz" | "flip" | "isochoric"
            | "update" => (1, &[]),
            _ => return None,
        };
        if i + count >= args.len() {
            return None;
        }
        axes.extend_from_slice(selected);
        i += count + 1;
    }
    axes.sort_unstable();
    axes.dedup();
    Some(axes)
}

impl Engine {
    fn weaken(&mut self) {
        self.fixes.clear();
        self.fix_inventory_known = false;
        self.dimension = None;
        self.periodic = [None; 3];
        self.thermo_modified = None;
        self.thermal.clear();
    }
    #[expect(
        clippy::too_many_arguments,
        reason = "A finding needs source context, rule, outcome, evidence and a diagnostic sink"
    )]
    fn report(
        &mut self,
        source: &Source,
        span: Span,
        rule: &Rule,
        outcome: Outcome,
        evidence: Vec<Evidence>,
        message: &str,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let actions = if outcome == Outcome::Violated {
            vec![
                "Review the cited manual constraint and the intended model before changing the input",
            ]
        } else {
            vec![]
        };
        if outcome == Outcome::Violated {
            let mut diagnostic = Diagnostic::new(source, span, Severity::Warning, rule.id, message);
            diagnostic.evidence = evidence.clone();
            diagnostic.suggested_actions = actions.iter().map(|s| (*s).into()).collect();
            diagnostics.push(diagnostic);
        }
        self.findings.push(Finding {
            rule_id: rule.id,
            outcome,
            span: source.located(span),
            evidence,
            suggested_actions: actions,
        });
    }
    pub fn command(
        &mut self,
        source: &Source,
        command: &Command,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let args: Vec<_> = command.arguments.iter().map(|a| a.text.as_str()).collect();
        let Some(name) = args.first().copied() else {
            return;
        };
        let span = command.line.span;
        if !command.unambiguous {
            self.weaken();
            return;
        }
        let evidence = |subject: &str, definition: String, at: Span| Evidence {
            subject: subject.into(),
            definition,
            span: source.located(at),
        };
        match name {
            "clear" => {
                let findings = std::mem::take(&mut self.findings);
                *self = Self::default();
                self.findings = findings;
            }
            "dimension" => {
                self.dimension = args
                    .get(1)
                    .and_then(|s| s.parse().ok())
                    .filter(|n| matches!(n, 2 | 3))
            }
            "boundary" if args.len() == 4 => {
                self.periodic = std::array::from_fn(|i| {
                    let value = args[i + 1];
                    if matches!(value, "p" | "pp") {
                        Some(true)
                    } else if !value.is_empty()
                        && value.len() <= 2
                        && value.chars().all(|c| "fsm".contains(c))
                    {
                        Some(false)
                    } else {
                        None
                    }
                });
            }
            "create_box" | "read_data" => {
                if self.dimension == Some(2) {
                    let outcome = match self.periodic[2] {
                        Some(true) => Outcome::Satisfied,
                        Some(false) => Outcome::Violated,
                        None => Outcome::Unknown,
                    };
                    self.report(
                        source,
                        span,
                        &TWO_DIMENSIONAL,
                        outcome,
                        vec![evidence(
                            "dimension and boundary",
                            format!("dimension 2; z periodicity {:?}", self.periodic[2]),
                            span,
                        )],
                        "Two-dimensional simulations require a periodic z boundary",
                        diagnostics,
                    );
                }
                self.thermal.clear();
                if name == "read_data" && args.contains(&"fix") {
                    self.weaken();
                }
            }
            "fix" if args.len() >= 4 => {
                let style = base_style(args[3]);
                let integrates = matches!(style, "nve" | "nvt" | "npt" | "nph");
                let thermostats = matches!(
                    style,
                    "nvt" | "npt" | "langevin" | "temp/rescale" | "temp/berendsen"
                );
                let known =
                    integrates || thermostats || matches!(style, "momentum" | "shake" | "setforce");
                self.fixes.insert(
                    args[1].into(),
                    Fix {
                        group: args[2].into(),
                        style: style.into(),
                        integrates,
                        thermostats,
                        known,
                        axes: matches!(style, "npt" | "nph")
                            .then(|| pressure_axes(&args[4..], self.dimension))
                            .flatten(),
                        span,
                    },
                );
                if style == "langevin" {
                    self.seed(source, command, 7, diagnostics);
                }
            }
            "unfix" if args.len() == 2 => {
                self.fixes.remove(args[1]);
            }
            "thermo_modify" if args.len() >= 3 => self.thermo_modified = Some(span),
            "thermo_style" => {
                if let Some(previous) = self.thermo_modified.take() {
                    self.report(source, span, &THERMO_RESET, Outcome::Violated,
                        vec![evidence("thermo_modify", "Earlier thermodynamic settings are reset by this thermo_style".into(), previous)],
                        "thermo_style resets earlier thermo_modify settings; reapply any settings you intend to retain", diagnostics);
                }
            }
            "velocity" if args.len() >= 3 => match args[2] {
                "create" if args.len() >= 5 => {
                    self.seed(source, command, 4, diagnostics);
                    self.thermal.clear();
                    if args[3]
                        .parse::<f64>()
                        .is_ok_and(|n| n.is_finite() && n > 0.0)
                    {
                        self.thermal.insert(args[1].into(), span);
                    }
                }
                "set" if args.len() >= 6 => {
                    let options = &args[6..];
                    let understood = options.len() % 2 == 0
                        && options.chunks_exact(2).all(|pair| {
                            matches!(
                                pair[0],
                                "sum"
                                    | "units"
                                    | "mom"
                                    | "rot"
                                    | "dist"
                                    | "temp"
                                    | "bias"
                                    | "loop"
                                    | "rigid"
                            )
                        });
                    let numeric = args[3..6]
                        .iter()
                        .all(|s| *s == "NULL" || s.parse::<f64>().is_ok_and(f64::is_finite));
                    if understood && numeric && args[3..6].iter().any(|s| *s != "NULL") {
                        if let Some(previous) = self
                            .thermal
                            .get(args[1])
                            .or_else(|| self.thermal.get("all"))
                            .copied()
                        {
                            let sum = options
                                .chunks_exact(2)
                                .rfind(|p| p[0] == "sum")
                                .is_some_and(|p| p[1] == "yes");
                            self.report(source, span, &VELOCITY_REPLACE,
                                if sum { Outcome::Satisfied } else { Outcome::Violated },
                                vec![evidence("thermal initialization", format!("velocity {} create preceded this assignment", args[1]), previous)],
                                "velocity set replaces initialized velocity components unless sum yes is specified; review whether replacement is intended", diagnostics);
                        }
                    }
                    self.thermal.clear();
                }
                _ => {
                    self.thermal.clear();
                }
            },
            "region" if args.len() >= 4 && matches!(args[2], "union" | "intersect") => {
                if let Ok(count) = args[3].parse::<i64>() {
                    self.report(
                        source,
                        span,
                        &REGION_COUNT,
                        if count < 2 {
                            Outcome::Violated
                        } else {
                            Outcome::Satisfied
                        },
                        vec![evidence(
                            "region count",
                            format!("{} count = {count}", args[2]),
                            span,
                        )],
                        "A region union or intersection requires at least two regions",
                        diagnostics,
                    );
                }
            }
            "group" | "create_atoms" | "delete_atoms" | "displace_atoms" => {
                self.thermal.clear();
            }
            "run" if args.len() == 2 && args[1].parse::<u64>().is_ok_and(|n| n > 0) => {
                self.evaluate(source, span, diagnostics);
                if self.fixes.values().any(|f| !f.known) {
                    self.thermal.clear();
                }
            }
            "run" if args.len() == 2 && args[1] == "0" => {}
            "units" | "atom_style" | "timestep" | "thermo" | "dump" | "undump" | "dump_modify"
            | "compute" | "uncompute" | "compute_modify" | "fix_modify" | "print" | "variable"
            | "log" | "write_data" | "write_restart" | "pair_style" | "pair_coeff"
            | "pair_modify" | "bond_style" | "bond_coeff" | "angle_style" | "angle_coeff"
            | "dihedral_style" | "dihedral_coeff" | "improper_style" | "improper_coeff"
            | "mass" | "lattice" | "region" | "neighbor" | "neigh_modify" | "comm_modify"
            | "special_bonds" | "kspace_style" | "kspace_modify" | "suffix" | "reset_timestep" => {}
            _ => self.weaken(),
        }
    }
    fn seed(
        &mut self,
        source: &Source,
        command: &Command,
        position: usize,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let Some(arg) = command.arguments.get(position) else {
            return;
        };
        if let Ok(seed) = arg.text.parse::<i64>() {
            self.report(
                source,
                arg.span,
                &RANDOM_SEED,
                if seed > 0 {
                    Outcome::Satisfied
                } else {
                    Outcome::Violated
                },
                vec![Evidence {
                    subject: "random seed".into(),
                    definition: seed.to_string(),
                    span: source.located(arg.span),
                }],
                "This random seed must be a positive integer",
                diagnostics,
            );
        }
    }
    fn evaluate(&mut self, source: &Source, span: Span, diagnostics: &mut Vec<Diagnostic>) {
        let fixes: Vec<_> = self
            .fixes
            .iter()
            .map(|(id, f)| (id.clone(), f.clone()))
            .collect();
        let evidence = |id: &str, fix: &Fix| Evidence {
            subject: format!("fix {id}"),
            definition: format!("{}, group {}", fix.style, fix.group),
            span: source.located(fix.span),
        };
        for (i, (id, fix)) in fixes.iter().enumerate() {
            for (other_id, other) in fixes.iter().skip(i + 1) {
                let outcome = if self.fixes.values().all(|f| f.known)
                    && overlap(&fix.group, &other.group) == Some(true)
                {
                    Outcome::Violated
                } else {
                    Outcome::Unknown
                };
                for (applies, rule, message) in [
                    (
                        fix.integrates && other.integrates,
                        &INTEGRATION,
                        "Known time integrators may update the same atoms more than once per timestep",
                    ),
                    (
                        fix.thermostats && other.thermostats,
                        &THERMOSTATS,
                        "Multiple known fixes control temperature on potentially overlapping atoms",
                    ),
                ] {
                    if applies {
                        self.report(
                            source,
                            span,
                            rule,
                            outcome,
                            vec![evidence(id, fix), evidence(other_id, other)],
                            message,
                            diagnostics,
                        );
                    }
                }
            }
            if matches!(fix.style.as_str(), "npt" | "nph") {
                let outcome = match &fix.axes {
                    Some(axes) if axes.iter().any(|i| self.periodic[*i] == Some(false)) => {
                        Outcome::Violated
                    }
                    Some(axes)
                        if !axes.is_empty()
                            && axes.iter().all(|i| self.periodic[*i] == Some(true)) =>
                    {
                        Outcome::Satisfied
                    }
                    _ => Outcome::Unknown,
                };
                self.report(source, span, &BAROSTAT, outcome, vec![evidence(id, fix), Evidence {
                    subject: "boundary periodicity".into(), definition: format!("{:?}", self.periodic), span: source.located(span),
                }], "A barostatted dimension is nonperiodic; review boundary and npt/nph pressure components", diagnostics);
            }
            if fix.style == "langevin" {
                let mut outcome = if self.fix_inventory_known {
                    Outcome::Violated
                } else {
                    Outcome::Unknown
                };
                for (_, other) in &fixes {
                    if !other.known {
                        outcome = Outcome::Unknown;
                    }
                    if other.integrates {
                        if other.group == fix.group || other.group == "all" {
                            outcome = Outcome::Satisfied;
                            break;
                        }
                        outcome = Outcome::Unknown;
                    }
                }
                self.report(source, span, &LANGEVIN_INTEGRATION, outcome, vec![evidence(id, fix)],
                    "fix langevin modifies forces but does not integrate positions; no overlapping known integrator was found", diagnostics);
            }
        }
    }
}
