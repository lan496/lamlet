//! Lexical ownership and the deliberately small set of known resource effects.
use crate::{
    diagnostic::{Diagnostic, Severity},
    expr::ExprError,
    source::{Source, Span},
    syntax::{Argument, Command, LogicalLine},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Key {
    kind: String,
    id: String,
}
#[derive(Clone, Debug)]
struct Live {
    managed: bool,
    group: String,
    integrates: bool,
    dependencies: BTreeSet<Key>,
    parameters: BTreeMap<String, Key>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Action {
    Plain,
    Acquire,
    Release,
}
#[derive(Default)]
pub(crate) struct Lifetimes {
    active: Vec<Key>,
    scopes: Vec<Vec<Key>>,
    live: BTreeMap<Key, Live>,
    retired: BTreeSet<Key>,
    borrowers: BTreeMap<Key, BTreeSet<Key>>,
}
fn error(code: &'static str, message: impl Into<String>, span: Span) -> ExprError {
    ExprError::new(code, message, span)
}
fn creating(command: &Command) -> Option<Key> {
    let args = &command.arguments;
    let kind = &args.first()?.text;
    if !matches!(kind.as_str(), "fix" | "compute" | "dump") {
        return None;
    }
    Some(Key {
        kind: kind.clone(),
        id: args.get(1)?.text.clone(),
    })
}
fn deleting(command: &Command) -> Option<Key> {
    let args = &command.arguments;
    let kind = match args.first()?.text.as_str() {
        "unfix" => "fix",
        "uncompute" => "compute",
        "undump" => "dump",
        _ => return None,
    };
    Some(Key {
        kind: kind.into(),
        id: args.get(1)?.text.clone(),
    })
}

fn references(command: &Command) -> BTreeSet<Key> {
    let args = &command.arguments;
    let mut refs = BTreeSet::new();
    let Some(first) = args.first() else {
        return refs;
    };
    let target_kind = match first.text.as_str() {
        "fix_modify" => Some("fix"),
        "compute_modify" => Some("compute"),
        "dump_modify" => Some("dump"),
        _ => None,
    };
    if let (Some(kind), Some(id)) = (target_kind, args.get(1).filter(|a| !a.dynamic)) {
        refs.insert(Key {
            kind: kind.into(),
            id: id.text.clone(),
        });
    }
    refs.extend(compute_parameters(command).into_values());
    if !matches!(
        first.text.as_str(),
        "fix" | "compute" | "dump" | "variable" | "thermo_style" | "fix_modify" | "compute_modify"
    ) {
        return refs;
    }
    let start = match first.text.as_str() {
        "fix" | "compute" => 4,
        "dump" => 6,
        "variable" => 3,
        _ => 2,
    };
    for (position, arg) in args.iter().enumerate().skip(start) {
        if arg.interpolation.is_some() || first.text == "dump" && position == 5 {
            continue;
        }
        // References occur in whole attribute tokens and numeric expressions.
        // String-style variables are not numeric expressions.
        if first.text == "variable"
            && !args
                .get(2)
                .is_some_and(|a| matches!(a.text.as_str(), "equal" | "atom" | "vector"))
        {
            continue;
        }
        let bytes = arg.text.as_bytes();
        let mut i = 0;
        while i + 2 < bytes.len() {
            if matches!(bytes[i], b'c' | b'f')
                && bytes[i + 1] == b'_'
                && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric() && bytes[i - 1] != b'_')
            {
                let start = i + 2;
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > start {
                    refs.insert(Key {
                        kind: if bytes[i] == b'c' { "compute" } else { "fix" }.into(),
                        id: arg.text[start..end].into(),
                    });
                }
                i = end;
            } else {
                i += 1;
            }
        }
    }
    refs
}

fn compute_parameters(command: &Command) -> BTreeMap<String, Key> {
    let args = &command.arguments;
    let mut values = BTreeMap::new();
    let mut i = match args.first().map(|a| a.text.as_str()) {
        Some("fix_modify" | "compute_modify") => 2,
        Some("thermo_modify") => 1,
        _ => return values,
    };
    while let (Some(key), Some(value)) = (args.get(i), args.get(i + 1)) {
        match key.text.as_str() {
            "temp" | "press" if !value.dynamic => {
                values.insert(
                    key.text.clone(),
                    Key {
                        kind: "compute".into(),
                        id: value.text.clone(),
                    },
                );
            }
            "energy" | "virial" | "dynamic/dof" | "extra/dof" | "lost" | "norm" | "flush"
            | "line" | "bodyforces" => {}
            _ => break,
        }
        i += 2;
    }
    values
}

impl Lifetimes {
    pub fn before(
        &self,
        command: &Command,
        action: Action,
        strict: bool,
        verified_effects: bool,
        source: &Source,
        diagnostics: &mut Vec<Diagnostic>,
    ) -> Result<(), ExprError> {
        let acquire = action == Action::Acquire;
        let release = action == Action::Release;
        let args = &command.arguments;
        let Some(first) = args.first() else {
            return Ok(());
        };
        let name = first.text.as_str();
        if acquire {
            if self.scopes.is_empty() {
                return Err(error(
                    "E_SCOPE_RESOURCE",
                    "Managed declarations require @scope",
                    command.line.span,
                ));
            }
            let key = creating(command).ok_or_else(|| {
                error(
                    "E_SCOPE_RESOURCE",
                    "Managed declarations support fix, compute, and dump",
                    command.line.span,
                )
            })?;
            let minimum = if key.kind == "dump" { 6 } else { 4 };
            if !command.unambiguous
                || args.len() < minimum
                || args[1].interpolation.is_some()
                || args[1].quoted
                || args[1].text.is_empty()
                || !args[1]
                    .text
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_')
            {
                return Err(error(
                    "E_SCOPE_RESOURCE",
                    "Managed resources require a complete header and a literal alphanumeric/underscore ID",
                    command.line.span,
                ));
            }
            if self.live.contains_key(&key) {
                return Err(error(
                    "E_SCOPE_OWNERSHIP",
                    format!("{} '{}' is already live", key.kind, key.id),
                    args[1].span,
                ));
            }
        }
        if let Some(key) = deleting(command) {
            if !release && self.active.contains(&key) {
                return Err(error(
                    "E_SCOPE_OWNERSHIP",
                    format!(
                        "{} '{}' is managed by @scope and cannot be destroyed manually",
                        key.kind, key.id
                    ),
                    args[1].span,
                ));
            }
            if self.live.iter().any(|(consumer, live)| {
                consumer != &key
                    && (live.dependencies.contains(&key)
                        || live.parameters.values().any(|p| p == &key))
                    && (live.managed || self.active.contains(&key))
            }) || self.active.contains(&key)
                && self.borrowers.values().any(|refs| refs.contains(&key))
            {
                return Err(error(
                    "E_SCOPE_DEPENDENCY",
                    format!(
                        "{} '{}' is still referenced by a live resource",
                        key.kind, key.id
                    ),
                    args[1].span,
                ));
            }
        }
        if let Some(key) = creating(command) {
            if self.active.contains(&key) {
                return Err(error(
                    "E_SCOPE_OWNERSHIP",
                    "Cannot replace a resource owned by @scope",
                    args[1].span,
                ));
            }
            if key.kind == "fix"
                && args
                    .get(3)
                    .is_some_and(|a| matches!(a.text.as_str(), "nve" | "nvt" | "npt"))
            {
                let group = &args[2].text;
                if self.live.iter().any(|(other, live)| {
                    other != &key
                        && live.integrates
                        && &live.group == group
                        && (acquire || live.managed)
                }) {
                    return Err(error(
                        "E_EFFECT_CONFLICT",
                        format!("Multiple known time integrators act on group '{group}'"),
                        args[2].span,
                    ));
                }
            }
        }
        for key in references(command) {
            if self.retired.contains(&key) && !self.live.contains_key(&key) {
                return Err(error(
                    "E_SCOPE_USE_AFTER_FREE",
                    format!("{} '{}' is outside its managed lifetime", key.kind, key.id),
                    command.line.span,
                ));
            }
        }
        if self.scopes.is_empty() {
            return Ok(());
        }
        if matches!(name, "jump" | "quit" | "clear") {
            return Err(error(
                "E_SCOPE_ESCAPE",
                "This command can bypass or invalidate @scope cleanup",
                first.span,
            ));
        }
        if matches!(name, "if" | "run") {
            for arg in args.iter().skip(2).filter(|a| a.quoted) {
                let nodes = crate::syntax::parse(arg.text.as_bytes(), false)?;
                for node in nodes {
                    if let crate::syntax::Node::Command(mut embedded) = node {
                        embedded.line.span = arg.span;
                        for a in &mut embedded.arguments {
                            a.span = arg.span;
                        }
                        // A nested conditional is treated as opaque below. The
                        // direct command is enough to detect explicit exits.
                        self.check_embedded(&embedded)?;
                    }
                }
            }
        }
        let barrier = !command.unambiguous
            || matches!(name, "include" | "if" | "python" | "next")
            || name == "run" && args.len() > 2
            || !verified_effects
                && crate::schema::find(name).is_none()
                && !matches!(
                    name,
                    "fix_modify"
                        | "compute_modify"
                        | "dump_modify"
                        | "thermo_style"
                        | "thermo_modify"
                        | "print"
                        | "label"
                        | "variable"
                );
        if barrier {
            diagnostics.push(Diagnostic::new(
                source,
                command.line.span,
                if strict {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                if strict {
                    "E_SCOPE_UNVERIFIED"
                } else {
                    "W_SCOPE_UNVERIFIED"
                },
                "Opaque commands may invalidate managed resource state or bypass cleanup",
            ));
        }
        Ok(())
    }
    fn check_embedded(&self, command: &Command) -> Result<(), ExprError> {
        let args = &command.arguments;
        if args
            .first()
            .is_some_and(|a| matches!(a.text.as_str(), "jump" | "quit" | "clear"))
        {
            return Err(error(
                "E_SCOPE_ESCAPE",
                "Embedded command can bypass @scope cleanup",
                command.line.span,
            ));
        }
        if deleting(command).is_some_and(|key| self.active.contains(&key))
            || creating(command).is_some_and(|key| self.active.contains(&key))
        {
            return Err(error(
                "E_SCOPE_OWNERSHIP",
                "Embedded command modifies a managed resource",
                command.line.span,
            ));
        }
        if args.first().is_some_and(|a| a.text == "if") {
            // Reject an unanalyzed nested conditional rather than asserting that
            // its quoted commands cannot escape the scope.
            return Err(error(
                "E_SCOPE_ESCAPE",
                "Nested runtime conditionals inside @scope require control-flow analysis",
                command.line.span,
            ));
        }
        Ok(())
    }
    pub fn after(&mut self, command: &Command, action: Action, verified_effects: bool) {
        let acquire = action == Action::Acquire;
        let release = action == Action::Release;
        let args = &command.arguments;
        if !command.unambiguous
            || args.first().is_some_and(|a| {
                matches!(
                    a.text.as_str(),
                    "include" | "jump" | "if" | "python" | "clear"
                ) || !verified_effects
                    && crate::schema::find(&a.text).is_none()
                    && !matches!(
                        a.text.as_str(),
                        "fix"
                            | "compute"
                            | "fix_modify"
                            | "compute_modify"
                            | "dump_modify"
                            | "thermo_style"
                            | "thermo_modify"
                            | "print"
                            | "label"
                            | "variable"
                    )
            })
            || args.first().is_some_and(|a| a.text == "run") && args.len() > 2
        {
            self.live.retain(|_, live| live.managed);
            self.retired.clear();
            self.borrowers.clear();
        }
        if let Some(key) = creating(command) {
            if command.unambiguous {
                let live = Live {
                    managed: acquire,
                    group: args.get(2).map_or("", |a| a.text.as_str()).into(),
                    integrates: key.kind == "fix"
                        && args
                            .get(3)
                            .is_some_and(|a| matches!(a.text.as_str(), "nve" | "nvt" | "npt")),
                    dependencies: references(command),
                    parameters: BTreeMap::new(),
                };
                self.retired.remove(&key);
                self.live.insert(key.clone(), live);
                if acquire {
                    self.scopes.last_mut().unwrap().push(key.clone());
                    self.active.push(key);
                }
            }
        }
        if let Some(key) = deleting(command) {
            self.live.remove(&key);
            if release {
                self.active.retain(|active| active != &key);
                self.retired.insert(key);
            }
        }
        if args
            .first()
            .is_some_and(|a| matches!(a.text.as_str(), "fix_modify" | "compute_modify"))
        {
            let kind = if args[0].text == "fix_modify" {
                "fix"
            } else {
                "compute"
            };
            if let Some(id) = args.get(1) {
                if let Some(live) = self.live.get_mut(&Key {
                    kind: kind.into(),
                    id: id.text.clone(),
                }) {
                    live.parameters.extend(compute_parameters(command));
                }
            }
        }
        if command.unambiguous {
            if args.first().is_some_and(|a| a.text == "variable") {
                if let Some(id) = args.get(1) {
                    let key = Key {
                        kind: "variable".into(),
                        id: id.text.clone(),
                    };
                    let refs = references(command);
                    if refs.is_empty() {
                        self.borrowers.remove(&key);
                    } else {
                        self.borrowers.insert(key, refs);
                    }
                }
            }
            if args.first().is_some_and(|a| a.text == "thermo_style") {
                self.borrowers.retain(|key, _| key.kind != "thermo");
            }
            if args.first().is_some_and(|a| a.text == "thermo_modify") {
                for (parameter, key) in compute_parameters(command) {
                    self.borrowers.insert(
                        Key {
                            kind: "thermo".into(),
                            id: parameter,
                        },
                        BTreeSet::from([key]),
                    );
                }
            }
        }
        if args.first().is_some_and(|a| a.text == "clear") {
            self.live.clear();
        }
    }
    pub fn enter_scope(&mut self) {
        self.scopes.push(vec![]);
    }
    pub fn leave_scope(&mut self) {
        self.scopes.pop();
    }
    pub fn cleanup(&self, span: Span) -> Option<Command> {
        self.scopes
            .last()?
            .iter()
            .rev()
            .find(|key| self.active.contains(key))
            .map(|key| Self::release_command(key, span))
    }
    pub fn drop_command(&self, id: &str, span: Span) -> Result<Command, ExprError> {
        let mut matches = self.active.iter().filter(|key| key.id == id);
        let key = matches.next().ok_or_else(|| {
            error(
                "E_SCOPE_USE_AFTER_FREE",
                format!("'{id}' is not a live managed resource"),
                span,
            )
        })?;
        if matches.next().is_some() {
            return Err(error(
                "E_SCOPE_RESOURCE",
                format!("@drop ID '{id}' is ambiguous across resource kinds"),
                span,
            ));
        }
        Ok(Self::release_command(key, span))
    }
    fn release_command(key: &Key, span: Span) -> Command {
        let name = match key.kind.as_str() {
            "fix" => "unfix",
            "compute" => "uncompute",
            "dump" => "undump",
            _ => unreachable!(),
        };
        Command {
            line: LogicalLine {
                span,
                continuations: vec![],
            },
            unambiguous: true,
            arguments: [name, &key.id]
                .into_iter()
                .map(|text| Argument {
                    span,
                    text: text.into(),
                    quoted: false,
                    dynamic: false,
                    interpolation: None,
                })
                .collect(),
        }
    }
}
