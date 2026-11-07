use crate::{
    Options,
    analysis::{self, State},
    codegen::{ExpansionContext, Output},
    diagnostic::{Diagnostic, Severity},
    expr::{self, ExprError, Scope, Type, Value},
    hir,
    source::{Source, Span},
    syntax::{Command, Node},
    units::target_scale,
};

pub(crate) struct Lowerer<'a> {
    source: &'a Source,
    options: &'a Options,
    pub output: Output,
    pub diagnostics: Vec<Diagnostic>,
    scopes: Vec<Scope>,
    context: Vec<ExpansionContext>,
    state: State,
    expanded_lines: usize,
    expansion_steps: usize,
}

impl<'a> Lowerer<'a> {
    pub fn new(source: &'a Source, options: &'a Options) -> Self {
        Self {
            source,
            options,
            output: Output::default(),
            diagnostics: vec![],
            scopes: vec![Scope::new()],
            context: vec![],
            state: State::default(),
            expanded_lines: 0,
            expansion_steps: 0,
        }
    }
    fn error(&self, code: &'static str, message: impl Into<String>, span: Span) -> ExprError {
        ExprError {
            code,
            message: message.into(),
            span,
        }
    }
    pub fn run(&mut self, nodes: &[Node]) {
        if let Err(error) = self.nodes(nodes) {
            self.diagnostics.push(Diagnostic::new(
                self.source,
                error.span,
                Severity::Error,
                error.code,
                error.message,
            ));
        }
    }
    fn step(&mut self, span: Span) -> Result<(), ExprError> {
        if !self.context.is_empty() {
            self.expansion_steps += 1;
            // A separate work bound also catches nested loops with no emitted lines.
            if self.expansion_steps > self.options.max_expanded_lines.saturating_mul(16).max(1024) {
                return Err(self.error(
                    "E_EXPANSION_LIMIT",
                    "Compile-time expansion work limit exceeded",
                    span,
                ));
            }
        }
        Ok(())
    }
    fn nodes(&mut self, nodes: &[Node]) -> Result<(), ExprError> {
        for node in nodes {
            match node {
                Node::Let {
                    name,
                    expression,
                    span,
                } => {
                    self.step(*span)?;
                    if self.scopes.last().unwrap().contains_key(name) {
                        return Err(self.error(
                            "E_DUPLICATE_BINDING",
                            format!("'{name}' is already bound in this scope"),
                            *span,
                        ));
                    }
                    let value = expr::evaluate(expression, &self.scopes)?;
                    self.scopes.last_mut().unwrap().insert(name.clone(), value);
                }
                Node::For {
                    name,
                    items,
                    body,
                    span,
                } => {
                    self.step(*span)?;
                    let mut values = items
                        .iter()
                        .map(|e| expr::evaluate(e, &self.scopes))
                        .collect::<Result<Vec<_>, _>>()?;
                    let mut ty = values.first().map(Value::ty).unwrap_or(Type::Unknown);
                    for value in &values {
                        if matches!(
                            (&ty, value.ty()),
                            (Type::Int, Type::Float) | (Type::Float, Type::Int)
                        ) {
                            ty = Type::Float;
                        } else if value.ty() != ty {
                            return Err(self.error(
                                "E_TYPE_MISMATCH",
                                "Loop lists must be homogeneous",
                                *span,
                            ));
                        }
                    }
                    if ty == Type::Float {
                        for value in &mut values {
                            if let Value::Int(n) = value {
                                *value = Value::Float(*n as f64);
                            }
                        }
                    }
                    for (iteration, value) in values.into_iter().enumerate() {
                        self.scopes.push(Scope::from([(name.clone(), value)]));
                        self.context.push(ExpansionContext {
                            variable: name.clone(),
                            iteration,
                        });
                        self.step(*span)?;
                        let result = self.nodes(body);
                        self.context.pop();
                        self.scopes.pop();
                        result?;
                    }
                }
                Node::Command(command) => {
                    self.step(command.line.span)?;
                    self.command(command)?;
                }
            }
        }
        Ok(())
    }

    fn command(&mut self, command: &Command) -> Result<(), ExprError> {
        let mut replacements = vec![];
        let mut operands = vec![];
        let mut resolved = command.clone();
        for arg in &command.arguments {
            if let Some(interpolation) = &arg.interpolation {
                let value = expr::evaluate(&interpolation.expression, &self.scopes)?;
                operands.push(hir::Operand {
                    span: interpolation.expression.span,
                    value: Some(value.clone()),
                    introduced: true,
                    unchecked: interpolation.unchecked,
                });
                let text = match value {
                    Value::Int(n) => n.to_string(),
                    Value::Float(n) => format_number(n),
                    Value::Bool(b) => b.to_string(),
                    Value::Quantity(n, dimension) => {
                        let style = self.state.units.as_deref().unwrap_or("unknown");
                        let scale = target_scale(style, dimension).ok_or_else(|| self.error("E_UNSUPPORTED_UNITS", format!("Cannot convert {} with active units '{style}'; declare units metal or units real", dimension.name()), interpolation.expression.span))?;
                        let converted = n / scale;
                        if !scale.is_finite()
                            || scale == 0.0
                            || !converted.is_finite()
                            || (n != 0.0 && converted == 0.0)
                        {
                            return Err(self.error(
                                "E_NUMERIC_OVERFLOW",
                                "Unit conversion overflows or underflows the finite numeric range",
                                interpolation.expression.span,
                            ));
                        }
                        format_number(converted)
                    }
                };
                if interpolation.unchecked {
                    self.diagnostics.push(Diagnostic::new(
                        self.source,
                        interpolation.span,
                        Severity::Warning,
                        "W_UNCHECKED_INTERPOLATION",
                        "Explicit @unchecked interpolation bypasses operand verification",
                    ));
                }
                resolved.arguments[operands.len() - 1].text = text.clone();
                replacements.push((interpolation.span, text));
            } else {
                let value = arg
                    .text
                    .parse::<i64>()
                    .map(Value::Int)
                    .ok()
                    .or_else(|| arg.text.parse::<f64>().map(Value::Float).ok());
                operands.push(hir::Operand {
                    span: arg.span,
                    value,
                    introduced: false,
                    unchecked: false,
                });
            }
        }
        let hir = hir::Command {
            syntax: &resolved,
            operands,
        };
        analysis::analyze(
            self.source,
            &hir,
            &mut self.state,
            self.options.strict,
            &mut self.diagnostics,
        );
        let mut bytes = vec![];
        let mut cursor = command.line.span.start;
        for (span, replacement) in replacements {
            bytes.extend_from_slice(&self.source.bytes[cursor..span.start]);
            bytes.extend_from_slice(replacement.as_bytes());
            cursor = span.end;
        }
        bytes.extend_from_slice(&self.source.bytes[cursor..command.line.span.end]);
        if !self.context.is_empty() {
            self.expanded_lines += bytes.split_inclusive(|b| *b == b'\n').count();
            if self.expanded_lines > self.options.max_expanded_lines {
                return Err(self.error(
                    "E_EXPANSION_LIMIT",
                    format!(
                        "Expansion exceeds {} generated lines",
                        self.options.max_expanded_lines
                    ),
                    command.line.span,
                ));
            }
        }
        self.output
            .emit(self.source, command.line.span, &bytes, &self.context);
        Ok(())
    }
}

fn format_number(n: f64) -> String {
    if n == 0.0 { "0".into() } else { n.to_string() }
}
