//! Explicit stage definitions and bound, independently compiled invocations.
use crate::{
    expr::{self, ExprError, Value},
    module::Parameter,
    resources::{Project, ResourceKind, portable, relative, resolved_destination},
    source::{Source, Span},
    syntax::Node,
};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug)]
pub struct Definition {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub body: Vec<Node>,
    pub span: Span,
    pub end: Span,
}

#[derive(Clone, Debug)]
pub struct Invocation {
    pub stage: String,
    pub instance: String,
    pub arguments: BTreeMap<String, Value>,
}

impl Invocation {
    /// CLI expressions are closed values; they cannot capture source bindings.
    pub fn parse(
        stage: &str,
        instance: Option<&str>,
        arguments: &[String],
    ) -> Result<Self, ExprError> {
        let instance = instance.unwrap_or(stage);
        if !crate::syntax::identifier(stage) || !crate::syntax::identifier(instance) {
            return Err(ExprError::new(
                "E_STAGE_ARGUMENT",
                "Stage and instance names must be identifiers",
                Span::default(),
            ));
        }
        let mut bindings = BTreeMap::new();
        for argument in arguments {
            let (name, text) = argument.split_once('=').ok_or_else(|| {
                ExprError::new(
                    "E_STAGE_ARGUMENT",
                    "Expected --arg NAME=EXPR",
                    Span::default(),
                )
            })?;
            let name = name.trim();
            if !crate::syntax::identifier(name) || bindings.contains_key(name) {
                return Err(ExprError::new(
                    "E_STAGE_ARGUMENT",
                    "Invalid or duplicate stage argument",
                    Span::default(),
                ));
            }
            let expression = expr::parse(text, 0)?;
            let value = expr::evaluate(&expression, &[])?;
            bindings.insert(name.into(), value);
        }
        Ok(Self {
            stage: stage.into(),
            instance: instance.into(),
            arguments: bindings,
        })
    }
    pub fn directory(&self, project: &Project) -> Result<PathBuf, ExprError> {
        if !crate::syntax::identifier(&self.instance) {
            return Err(ExprError::new(
                "E_STAGE_ARGUMENT",
                "Invalid stage instance identifier",
                Span::default(),
            ));
        }
        let requested = project.build_directory.join("stages").join(&self.instance);
        let directory = resolved_destination(&requested)
            .map_err(|e| ExprError::new("E_PATH_IO", e.to_string(), Span::default()))?;
        if !directory.starts_with(&project.root)
            || !directory.starts_with(&project.build_directory)
            || directory != requested
        {
            return Err(ExprError::new(
                "E_PATH_ESCAPE",
                "Stage directory escapes the build root or aliases another directory through a symlink",
                Span::default(),
            ));
        }
        Ok(directory)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ParameterInfo {
    pub name: String,
    #[serde(rename = "type")]
    pub ty: String,
    pub direction: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct DefinitionInfo {
    pub name: String,
    pub parameters: Vec<ParameterInfo>,
}
#[derive(Clone, Debug, Serialize)]
pub struct BoundArgument {
    #[serde(rename = "type")]
    pub ty: String,
    pub value: ArgumentValue,
}
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum ArgumentValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Path(String),
}
#[derive(Clone, Debug, Serialize)]
pub struct Plan {
    pub stage: String,
    pub instance: String,
    /// Relative to the project root.
    pub working_directory: String,
    pub arguments: BTreeMap<String, BoundArgument>,
    pub independent_execution: bool,
    pub resumable: bool,
    pub target_profile_selected: bool,
}
impl Plan {
    pub(crate) fn new(
        invocation: &Invocation,
        project: &Project,
        directory: &std::path::Path,
        bindings: &expr::Scope,
    ) -> Self {
        let arguments = bindings
            .iter()
            .map(|(name, value)| {
                let encoded = match value {
                    Value::Int(n) => ArgumentValue::Int(*n),
                    Value::Float(n) | Value::Quantity(n, _) => ArgumentValue::Float(*n),
                    Value::Bool(b) => ArgumentValue::Bool(*b),
                    Value::Resource(value) => ArgumentValue::Path(value.path.clone()),
                };
                (
                    name.clone(),
                    BoundArgument {
                        ty: value.ty().name(),
                        value: encoded,
                    },
                )
            })
            .collect();
        Self {
            stage: invocation.stage.clone(),
            instance: invocation.instance.clone(),
            working_directory: portable(&relative(&project.root, directory)),
            arguments,
            independent_execution: false,
            resumable: false,
            target_profile_selected: false,
        }
    }
}

pub(crate) fn collect(nodes: &[Node]) -> Result<BTreeMap<String, Definition>, ExprError> {
    let mut definitions = BTreeMap::new();
    for node in nodes {
        if let Node::Stage(definition) = node {
            if definitions
                .insert(definition.name.clone(), definition.clone())
                .is_some()
            {
                return Err(ExprError::new(
                    "E_STAGE_DEFINITION",
                    "Duplicate stage name",
                    definition.span,
                ));
            }
            for parameter in &definition.parameters {
                if matches!(parameter.ty, expr::Type::Resource(kind) if !matches!(kind, ResourceKind::InputFile | ResourceKind::PotentialFile | ResourceKind::OutputFile))
                {
                    return Err(ExprError::new(
                        "E_STAGE_DEFINITION",
                        "Stage resources must be InputFile, PotentialFile, or OutputFile",
                        definition.span,
                    ));
                }
            }
        }
    }
    if !definitions.is_empty() {
        for node in nodes {
            if !matches!(node, Node::Stage(_) | Node::Module { .. })
                && !matches!(node, Node::Command(c) if c.arguments.is_empty())
            {
                return Err(ExprError::new(
                    "E_STAGE_DEFINITION",
                    "A stage source contains only stages, modules, and comments; place setup inside each stage",
                    Span::default(),
                ));
            }
        }
    }
    Ok(definitions)
}

pub fn inspect(source: &Source) -> Result<Vec<DefinitionInfo>, ExprError> {
    let nodes = crate::syntax::parse(&source.bytes, true)?;
    crate::module::Registry::collect(&nodes)?;
    Ok(collect(&nodes)?
        .into_values()
        .map(|definition| DefinitionInfo {
            name: definition.name,
            parameters: definition
                .parameters
                .into_iter()
                .map(|p| ParameterInfo {
                    direction: match p.ty {
                        expr::Type::Resource(crate::resources::ResourceKind::OutputFile) => {
                            "output"
                        }
                        expr::Type::Resource(_) => "input",
                        _ => "parameter",
                    },
                    name: p.name,
                    ty: p.ty.name(),
                })
                .collect(),
        })
        .collect())
}
