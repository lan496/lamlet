//! Same-source modules and conservative, sequential state contracts.
use crate::{
    analysis::Fact,
    expr::{ExprError, Type, Value},
    resources::ResourceKind,
    source::Span,
    syntax::{Command, Node},
    units::Dimension,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum StateFact {
    BoxDefined,
    AtomsDefined,
    ModelDefined,
    IntegratorConfigured,
}
impl StateFact {
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "BoxDefined" => Self::BoxDefined,
            "AtomsDefined" => Self::AtomsDefined,
            "ModelDefined" => Self::ModelDefined,
            "IntegratorConfigured" => Self::IntegratorConfigured,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub requires: Vec<StateFact>,
    pub ensures: Vec<StateFact>,
    pub body: Vec<Node>,
    pub span: Span,
    pub end: Span,
}

pub fn parameter_type(text: &str) -> Option<Type> {
    Some(match text {
        "Int" => Type::Int,
        "Float" => Type::Float,
        "Bool" => Type::Bool,
        "Time" => Type::Quantity(Dimension::TIME),
        "Length" => Type::Quantity(Dimension::LENGTH),
        "Temperature" => Type::Quantity(Dimension::TEMPERATURE),
        "Pressure" => Type::Quantity(Dimension::PRESSURE),
        "Energy" => Type::Quantity(Dimension::ENERGY),
        _ => Type::Resource(ResourceKind::parse(text)?),
    })
}

pub(crate) fn bind(
    parameter: &Parameter,
    mut value: Value,
    span: Span,
) -> Result<Value, ExprError> {
    let actual = value.ty();
    match (&parameter.ty, &mut value) {
        (Type::Float, Value::Int(n)) => value = Value::Float(*n as f64),
        (Type::Resource(expected), Value::Resource(resource)) => {
            if resource.kind == ResourceKind::InputFile && expected.is_input_file() {
                resource.kind = *expected;
            }
            if !expected.accepts(resource.kind) {
                return Err(type_error(parameter, &value, span));
            }
        }
        (expected, _) if *expected != actual => return Err(type_error(parameter, &value, span)),
        _ => {}
    }
    Ok(value)
}
fn type_error(parameter: &Parameter, value: &Value, span: Span) -> ExprError {
    ExprError::new(
        "E_TYPE_MISMATCH",
        format!(
            "Parameter '{}' expects {}, got {}",
            parameter.name,
            parameter.ty.name(),
            value.ty().name()
        ),
        span,
    )
}

#[derive(Default)]
pub(crate) struct Registry {
    pub functions: BTreeMap<String, Function>,
}
impl Registry {
    pub fn collect(nodes: &[Node]) -> Result<Self, ExprError> {
        let mut registry = Self::default();
        let mut modules = BTreeSet::new();
        for node in nodes {
            if let Node::Module { name, body, span } = node {
                if !modules.insert(name) {
                    return Err(ExprError::new(
                        "E_MODULE_DEFINITION",
                        "Duplicate module name",
                        *span,
                    ));
                }
                for node in body {
                    match node {
                        Node::Function(function) => {
                            let name = format!("{name}.{}", function.name);
                            if registry.functions.insert(name, function.clone()).is_some() {
                                return Err(ExprError::new(
                                    "E_MODULE_DEFINITION",
                                    "Duplicate function name",
                                    function.span,
                                ));
                            }
                        }
                        Node::Command(command) if command.arguments.is_empty() => {}
                        _ => {
                            return Err(ExprError::new(
                                "E_MODULE_DEFINITION",
                                "A module contains only @fn declarations and comments",
                                *span,
                            ));
                        }
                    }
                }
            }
        }
        // Validate the entire call graph, including unused definitions.
        for name in registry.functions.keys() {
            registry.visit(name, &mut vec![], &mut BTreeSet::new())?;
        }
        for node in nodes
            .iter()
            .filter(|node| !matches!(node, Node::Module { .. }))
        {
            registry.visit_body(
                std::slice::from_ref(node),
                &mut vec![],
                &mut BTreeSet::new(),
            )?;
        }
        Ok(registry)
    }
    fn visit(
        &self,
        name: &str,
        stack: &mut Vec<String>,
        done: &mut BTreeSet<String>,
    ) -> Result<(), ExprError> {
        let function = &self.functions[name];
        if stack.iter().any(|n| n == name) || stack.len() >= 64 {
            return Err(ExprError::new(
                "E_MODULE_RECURSION",
                "Module calls must be acyclic and at most 64 levels deep",
                function.span,
            ));
        }
        if !done.insert(name.into()) {
            return Ok(());
        }
        stack.push(name.into());
        self.visit_body(&function.body, stack, done)?;
        stack.pop();
        Ok(())
    }
    fn visit_body(
        &self,
        nodes: &[Node],
        stack: &mut Vec<String>,
        done: &mut BTreeSet<String>,
    ) -> Result<(), ExprError> {
        for node in nodes {
            match node {
                Node::Call { name, span, .. } => {
                    if !self.functions.contains_key(name) {
                        return Err(ExprError::new(
                            "E_MODULE_CALL",
                            format!("Unknown module function '{name}'"),
                            *span,
                        ));
                    }
                    self.visit(name, stack, done)?;
                }
                Node::Scope { body, .. } | Node::For { body, .. } => {
                    self.visit_body(body, stack, done)?
                }
                Node::Module { span, .. } | Node::Function(Function { span, .. }) => {
                    return Err(ExprError::new(
                        "E_MODULE_DEFINITION",
                        "Definitions must appear at their module boundary",
                        *span,
                    ));
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// Facts describe command effects, not file contents or physical completeness.
pub(crate) struct Effects {
    facts: BTreeMap<StateFact, Fact>,
    pair_style: Option<String>,
    integrators: BTreeSet<String>,
    pub units: Option<String>,
}
impl Default for Effects {
    fn default() -> Self {
        Self {
            facts: BTreeMap::new(),
            pair_style: None,
            integrators: BTreeSet::new(),
            units: Some("lj".into()),
        }
    }
}
impl Effects {
    pub fn require(&self, facts: &[StateFact], span: Span) -> Result<(), ExprError> {
        for fact in facts {
            let actual = self.facts.get(fact).copied().unwrap_or(Fact::Absent);
            if actual != Fact::Present {
                return Err(ExprError::new(
                    "E_STATE_CONTRACT",
                    format!("Required fact {fact:?} is {actual:?}"),
                    span,
                ));
            }
        }
        Ok(())
    }
    fn set(&mut self, fact: StateFact, value: Fact) {
        self.facts.insert(fact, value);
    }
    fn weaken(&mut self) {
        for fact in [
            StateFact::BoxDefined,
            StateFact::AtomsDefined,
            StateFact::ModelDefined,
            StateFact::IntegratorConfigured,
        ] {
            self.set(fact, Fact::Unknown);
        }
        self.pair_style = None;
        self.integrators.clear();
        self.units = None;
    }
    pub fn command(&mut self, command: &Command, checked: bool) -> Result<(), ExprError> {
        use StateFact::*;
        let args = &command.arguments;
        let Some(first) = args.first() else {
            return Ok(());
        };
        let name = first.text.as_str();
        let literal = |i: usize| args.get(i).map(|a| a.text.as_str()).unwrap_or("");
        let supported = command.unambiguous
            && !args.iter().any(|a| a.dynamic)
            && match name {
                "units" | "atom_style" | "dimension" | "boundary" | "mass" | "timestep"
                | "neighbor" | "neigh_modify" | "thermo" | "thermo_style" | "thermo_modify"
                | "fix_modify" | "compute_modify" | "unfix" | "uncompute" | "undump" => true,
                "region" => matches!(literal(2), "block" | "sphere"),
                "lattice" => matches!(literal(1), "sc" | "bcc" | "fcc" | "diamond"),
                "create_box" => args.len() == 3,
                "create_atoms" => matches!(literal(2), "box" | "region" | "single"),
                "read_data" | "write_data" | "write_restart" => args.len() == 2,
                "pair_style" => matches!(literal(1), "lj/cut" | "eam" | "eam/alloy"),
                "pair_coeff" => self.pair_style.is_some(),
                "fix" => matches!(literal(3), "nve" | "nvt" | "npt" | "langevin"),
                "compute" => matches!(literal(3), "temp" | "temp/com"),
                "dump" => literal(3) == "custom",
                "velocity" => matches!(literal(2), "create" | "set" | "zero" | "scale"),
                "run" => args.len() == 2,
                "minimize" => args.len() == 5,
                "clear" => true,
                _ => false,
            };
        if !supported {
            self.weaken();
            return if checked {
                Err(ExprError::new(
                    "E_UNVERIFIED_EFFECT",
                    format!("Cannot verify the state effects of '{name}' at this boundary"),
                    command.line.span,
                ))
            } else {
                Ok(())
            };
        }
        if checked {
            match name {
                "create_atoms" | "mass" | "fix" | "compute" | "dump" | "velocity"
                | "pair_coeff" => self.require(&[BoxDefined], command.line.span)?,
                "run" | "minimize" => {
                    self.require(&[AtomsDefined, ModelDefined], command.line.span)?;
                    if name == "run" && literal(1) != "0" {
                        self.require(&[IntegratorConfigured], command.line.span)?;
                    }
                }
                _ => {}
            }
        }
        match name {
            "units" => self.units = Some(literal(1).into()),
            "clear" => *self = Self::default(),
            "create_box" => {
                self.set(BoxDefined, Fact::Present);
                self.set(AtomsDefined, Fact::Absent);
            }
            "create_atoms" => self.set(AtomsDefined, Fact::Present),
            "read_data" => {
                self.set(BoxDefined, Fact::Present);
                self.set(AtomsDefined, Fact::Present);
            }
            "pair_style" => {
                self.pair_style = Some(literal(1).into());
                self.set(ModelDefined, Fact::Absent);
            }
            "pair_coeff" if literal(1) == "*" && literal(2) == "*" => {
                self.set(ModelDefined, Fact::Present)
            }
            "fix" => {
                self.integrators.remove(literal(1));
                if literal(2) == "all" && matches!(literal(3), "nve" | "nvt" | "npt") {
                    self.integrators.insert(literal(1).into());
                }
                self.set(
                    IntegratorConfigured,
                    if self.integrators.is_empty() {
                        Fact::Absent
                    } else {
                        Fact::Present
                    },
                );
            }
            "unfix" => {
                self.integrators.remove(literal(1));
                self.set(
                    IntegratorConfigured,
                    if self.integrators.is_empty() {
                        Fact::Absent
                    } else {
                        Fact::Present
                    },
                );
            }
            _ => {}
        }
        Ok(())
    }
}
