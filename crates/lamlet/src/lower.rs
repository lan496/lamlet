use crate::{
    Options,
    analysis::{self, State},
    codegen::{ExpansionContext, Output},
    diagnostic::{Diagnostic, Severity},
    expr::{self, ExprError, Scope, Type, Value},
    hir,
    resources::{self, ResourceKind},
    scoped::Action,
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
    pub resources: resources::Resolver,
    lifetimes: crate::scoped::Lifetimes,
    pub semantics: crate::semantics::Engine,
    pub lint: crate::lint::Engine,
    pub manual_lint: crate::manual_lint::Engine,
    pub capabilities: crate::capability::Checker,
    modules: crate::module::Registry,
    effects: crate::module::Effects,
    checked_depth: usize,
    call_depth: usize,
    pub stage: Option<crate::stage::Plan>,
    stage_resources: Vec<resources::ResourceValue>,
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
            state: if options.language == crate::Language::Lammps {
                State::legacy()
            } else {
                State::default()
            },
            expanded_lines: 0,
            expansion_steps: 0,
            resources: resources::Resolver::new(options.project.clone(), source),
            lifetimes: crate::scoped::Lifetimes::default(),
            semantics: crate::semantics::Engine::default(),
            lint: crate::lint::Engine::default(),
            manual_lint: crate::manual_lint::Engine::default(),
            capabilities: crate::capability::Checker::new(options.capabilities.clone()),
            modules: crate::module::Registry::default(),
            effects: crate::module::Effects::default(),
            checked_depth: 0,
            call_depth: 0,
            stage: None,
            stage_resources: vec![],
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
        if let Some(profile) = &self.options.capabilities {
            if let Err(error) = profile.validate() {
                self.diagnostics.push(Diagnostic::new(
                    self.source,
                    Span::default(),
                    Severity::Error,
                    "E_TARGET_PROFILE",
                    error,
                ));
                return;
            }
            if let Some(project) = &self.options.project {
                self.diagnostics.extend(crate::capability::check_project(
                    self.source,
                    profile,
                    &project.lammps.requirements,
                ));
            }
        }
        let result = crate::module::Registry::collect(nodes).and_then(|registry| {
            self.modules = registry;
            let stages = crate::stage::collect(nodes)?;
            if let Some(invocation) = &self.options.stage {
                let definition = stages.get(&invocation.stage).ok_or_else(|| self.error("E_STAGE_SELECTION", format!("Unknown stage '{}'", invocation.stage), Span::default()))?;
                self.stage_body(definition, invocation)
            } else if !stages.is_empty() {
                Err(self.error("E_STAGE_SELECTION", "Select an explicit stage and bind its parameters with --stage and --arg; inspect stages lists definitions", Span::default()))
            } else {
                self.nodes(nodes)
            }
        });
        if let Err(error) = result {
            self.diagnostics.push(Diagnostic::new(
                self.source,
                error.span,
                Severity::Error,
                error.code,
                error.message,
            ));
        }
    }
    fn stage_body(
        &mut self,
        definition: &crate::stage::Definition,
        invocation: &crate::stage::Invocation,
    ) -> Result<(), ExprError> {
        let project = self.options.project.as_ref().ok_or_else(|| {
            self.error(
                "E_PROJECT_REQUIRED",
                "Stage compilation requires a project",
                definition.span,
            )
        })?;
        let directory = invocation.directory(project)?;
        if invocation.arguments.len() != definition.parameters.len()
            || invocation
                .arguments
                .keys()
                .any(|name| !definition.parameters.iter().any(|p| p.name == *name))
        {
            return Err(self.error(
                "E_STAGE_ARGUMENT",
                "Bind exactly the declared stage parameters",
                definition.span,
            ));
        }
        let mut bindings = Scope::new();
        for parameter in &definition.parameters {
            let value = crate::module::bind(
                parameter,
                invocation.arguments[&parameter.name].clone(),
                definition.span,
            )?;
            if let Value::Resource(resource) = &value {
                self.stage_resources.push(resource.clone());
            }
            bindings.insert(parameter.name.clone(), value);
        }
        self.resources.stage_directory(directory.clone());
        self.stage = Some(crate::stage::Plan::new(
            invocation, project, &directory, &bindings,
        ));
        for resource in &self.stage_resources {
            self.resources
                .declare(resource, self.source, definition.span, "file", false)?;
        }
        self.scopes = vec![bindings];
        self.checked_depth = 1;
        self.context.push(ExpansionContext {
            variable: format!("@stage {}", invocation.stage),
            iteration: 0,
        });
        self.lifetimes.enter_scope();
        self.nodes(&definition.body)?;
        while let Some(cleanup) = self.lifetimes.cleanup(definition.end) {
            self.release(cleanup)?;
        }
        self.lifetimes.leave_scope();
        for output in &self.resources.dependencies.outputs {
            if !self.resources.output_was_written(&output.path) {
                return Err(self.error(
                    "E_STAGE_OUTPUT",
                    format!(
                        "Declared output '{}' has no generating command",
                        output.path
                    ),
                    definition.span,
                ));
            }
        }
        self.stage.as_mut().unwrap().independent_execution = true;
        self.stage.as_mut().unwrap().target_profile_selected = self.options.capabilities.is_some();
        self.resources.dependencies.complete = true;
        self.resources.dependencies.coverage =
            "declared stage file resources; file contents and runtime environment are not analyzed";
        Ok(())
    }
    fn stage_resource(
        &self,
        resource: &resources::ResourceValue,
        span: Span,
    ) -> Result<(), ExprError> {
        if self.stage.is_some()
            && !self.stage_resources.iter().any(|declared| {
                declared.path == resource.path
                    && (declared.kind.accepts(resource.kind)
                        || resource.kind.accepts(declared.kind))
            })
        {
            return Err(self.error(
                "E_STAGE_RESOURCE",
                "Stage file dependencies must come from declared resource parameters",
                span,
            ));
        }
        Ok(())
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
                    annotation,
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
                    let mut value = expr::evaluate(expression, &self.scopes)?;
                    if let Some(annotation) = annotation {
                        match &mut value {
                            Value::Resource(resource) if annotation.accepts(resource.kind) => {}
                            Value::Resource(resource)
                                if resource.kind == ResourceKind::InputFile
                                    && annotation.is_input_file() =>
                            {
                                resource.kind = *annotation
                            }
                            _ => {
                                return Err(self.error(
                                    "E_TYPE_MISMATCH",
                                    format!(
                                        "Expected {}, got {}",
                                        annotation.name(),
                                        value.ty().name()
                                    ),
                                    expression.span,
                                ));
                            }
                        }
                    }
                    if let Value::Resource(resource) = &value {
                        self.stage_resource(resource, expression.span)?;
                        self.resources.declare(
                            resource,
                            self.source,
                            expression.span,
                            "file",
                            false,
                        )?;
                    }
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
                    self.command(command, Emission::Original)?;
                }
                Node::Scope { body, end } => {
                    self.lifetimes.enter_scope();
                    self.scopes.push(Scope::new());
                    let result = self.nodes(body);
                    self.scopes.pop();
                    result?;
                    while let Some(cleanup) = self.lifetimes.cleanup(*end) {
                        self.release(cleanup)?;
                    }
                    self.lifetimes.leave_scope();
                }
                Node::Managed(command) => {
                    self.step(command.line.span)?;
                    self.command(command, Emission::Acquire)?;
                }
                Node::Drop { id, span } => {
                    self.step(*span)?;
                    let cleanup = self.lifetimes.drop_command(id, *span)?;
                    self.release(cleanup)?;
                }
                Node::Module { span, .. } if self.checked_depth == 0 => {
                    self.step(*span)?;
                }
                Node::Module { span, .. }
                | Node::Function(crate::module::Function { span, .. })
                | Node::Stage(crate::stage::Definition { span, .. }) => {
                    return Err(self.error("E_MODULE_DEFINITION", "Module definitions belong at the source boundary and functions inside modules", *span));
                }
                Node::Call {
                    name,
                    arguments,
                    span,
                } => {
                    self.step(*span)?;
                    if self.call_depth >= 64 {
                        return Err(self.error(
                            "E_MODULE_RECURSION",
                            "Module expansion exceeds 64 call levels",
                            *span,
                        ));
                    }
                    let function = self.modules.functions.get(name).cloned().ok_or_else(|| {
                        self.error(
                            "E_MODULE_CALL",
                            format!("Unknown module function '{name}'"),
                            *span,
                        )
                    })?;
                    if arguments.len() != function.parameters.len() {
                        return Err(self.error(
                            "E_ARGUMENT_COUNT",
                            format!("'{name}' expects {} arguments", function.parameters.len()),
                            *span,
                        ));
                    }
                    let mut parameters = Scope::new();
                    for (parameter, expression) in function.parameters.iter().zip(arguments) {
                        let value = crate::module::bind(
                            parameter,
                            expr::evaluate(expression, &self.scopes)?,
                            expression.span,
                        )?;
                        if let Value::Resource(resource) = &value {
                            self.stage_resource(resource, expression.span)?;
                            self.resources.declare(
                                resource,
                                self.source,
                                expression.span,
                                "file",
                                false,
                            )?;
                        }
                        parameters.insert(parameter.name.clone(), value);
                    }
                    self.effects.require(&function.requires, *span)?;
                    // Calls see only their explicit parameters, never caller-local bindings.
                    let caller_scopes = std::mem::replace(&mut self.scopes, vec![parameters]);
                    self.checked_depth += 1;
                    self.call_depth += 1;
                    self.context.push(ExpansionContext {
                        variable: format!("@call {name}"),
                        iteration: 0,
                    });
                    self.lifetimes.enter_scope();
                    let result = self.nodes(&function.body).and_then(|()| {
                        while let Some(cleanup) = self.lifetimes.cleanup(function.end) {
                            self.release(cleanup)?;
                        }
                        self.effects.require(&function.ensures, *span)
                    });
                    self.lifetimes.leave_scope();
                    self.context.pop();
                    self.checked_depth -= 1;
                    self.call_depth -= 1;
                    self.scopes = caller_scopes;
                    result?;
                    self.state.units = self.effects.units.clone();
                    self.state.pair_style = self.effects.pair_style.clone();
                }
            }
        }
        Ok(())
    }

    fn release(&mut self, cleanup: Command) -> Result<(), ExprError> {
        let text = format!(
            "{} {}\n",
            cleanup.arguments[0].text, cleanup.arguments[1].text
        );
        self.command(&cleanup, Emission::Release(&text))
    }

    fn command(&mut self, command: &Command, emission: Emission<'_>) -> Result<(), ExprError> {
        let mut replacements = vec![];
        let mut operands = vec![];
        let mut resolved = command.clone();
        let pair_style = if self.checked_depth > 0 {
            self.effects.pair_style.as_deref()
        } else {
            self.state.pair_style.as_deref()
        };
        let path_slot = crate::schema::path_operand(command, pair_style);
        if self.stage.is_some() {
            if command
                .arguments
                .first()
                .is_some_and(|a| a.text == "read_data")
                && (command.arguments.len() != 3 || command.arguments[2].text != "nocoeff")
            {
                return Err(self.error("E_UNVERIFIED_EFFECT", "Stage read_data requires nocoeff; declare the model and potential dependencies explicitly", command.line.span));
            }
            if let Some(slot) = path_slot {
                if !command
                    .arguments
                    .get(slot.position)
                    .is_some_and(|arg| arg.interpolation.is_some())
                {
                    return Err(self.error(
                        "E_STAGE_RESOURCE",
                        "Stage file operands must interpolate declared typed resources",
                        command.line.span,
                    ));
                }
            }
        }
        for (position, arg) in command.arguments.iter().enumerate() {
            if let Some(interpolation) = &arg.interpolation {
                if self.stage.is_some() && interpolation.unchecked {
                    return Err(self.error(
                        "E_STAGE_RESOURCE",
                        "Unchecked operands cannot establish an independent stage contract",
                        interpolation.span,
                    ));
                }
                let value = expr::evaluate(&interpolation.expression, &self.scopes)?;
                if let Some(slot) = path_slot.filter(|s| s.position == position) {
                    let compatible = match &value {
                        Value::Resource(resource) => {
                            slot.resource_type.accepts(resource.kind)
                                || slot.input_pattern && resource.kind == ResourceKind::InputPattern
                                || resource.kind == ResourceKind::DynamicPath
                        }
                        _ => false,
                    };
                    if !compatible && !interpolation.unchecked {
                        return Err(self.error(
                            "E_TYPE_MISMATCH",
                            format!(
                                "Expected {} path, got {}",
                                slot.resource_type.name(),
                                value.ty().name()
                            ),
                            interpolation.expression.span,
                        ));
                    }
                }
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
                    Value::Resource(resource) => {
                        self.stage_resource(&resource, interpolation.expression.span)?;
                        if self.stage.is_some()
                            && path_slot.is_none_or(|slot| slot.position != position)
                        {
                            return Err(self.error(
                                "E_STAGE_RESOURCE",
                                "Resource used outside a modeled file operand",
                                interpolation.span,
                            ));
                        }
                        let slot = path_slot.filter(|s| s.position == position);
                        let role = slot.map_or("file", |s| s.kind);
                        if role == "script" && resource.path.ends_with(".lamlet") {
                            return Err(self.error("E_INPUT_SCRIPT", "Compile Lamlet include files separately; automatic transitive compilation is not supported", interpolation.expression.span));
                        }
                        let path = self.resources.declare(
                            &resource,
                            self.source,
                            interpolation.expression.span,
                            role,
                            slot.is_some_and(|s| s.resource_type == ResourceKind::OutputFile)
                                && resource.kind == ResourceKind::OutputFile,
                        )?;
                        if resource.kind == ResourceKind::DynamicPath {
                            self.diagnostics.push(Diagnostic::new(self.source, interpolation.span,
                                if self.options.strict && !interpolation.unchecked { Severity::Error } else { Severity::Warning },
                                if self.options.strict && !interpolation.unchecked { "E_UNVERIFIED_ARGUMENT" } else { "W_DYNAMIC_PATH" },
                                "Runtime path substitutions are not checked for existence, containment, or collisions"));
                        }
                        self.resources.render(
                            &path,
                            resource.kind,
                            interpolation.expression.span,
                        )?
                    }
                    Value::Quantity(n, dimension) => {
                        let style = if self.checked_depth > 0 {
                            self.effects.units.as_deref()
                        } else {
                            self.state.units.as_deref()
                        }
                        .unwrap_or("unknown");
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
                resolved.arguments[operands.len() - 1].text = text
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or(&text)
                    .into();
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
        self.effects.command(&resolved, self.checked_depth > 0)?;
        if self.checked_depth > 0 {
            self.state.units = self.effects.units.clone();
            self.state.pair_style = self.effects.pair_style.clone();
        }
        self.lifetimes.before(
            &resolved,
            emission.action(),
            self.options.strict,
            self.checked_depth > 0,
            self.source,
            &mut self.diagnostics,
        )?;
        analysis::analyze(
            self.source,
            &hir,
            &mut self.state,
            self.options.strict,
            &mut self.diagnostics,
        );
        self.lifetimes
            .after(&resolved, emission.action(), self.checked_depth > 0);
        self.resources
            .after_command(self.source, &resolved, path_slot);
        self.semantics
            .command(self.source, &resolved, &mut self.diagnostics);
        self.lint
            .command(self.source, &resolved, &mut self.diagnostics);
        self.manual_lint
            .command(self.source, &resolved, &mut self.diagnostics);
        self.capabilities
            .command(self.source, &resolved, &mut self.diagnostics);
        let mut bytes = vec![];
        let mut cursor = command.line.span.start;
        for (span, replacement) in replacements {
            bytes.extend_from_slice(&self.source.bytes[cursor..span.start]);
            bytes.extend_from_slice(replacement.as_bytes());
            cursor = span.end;
        }
        bytes.extend_from_slice(&self.source.bytes[cursor..command.line.span.end]);
        match emission {
            Emission::Acquire if !bytes.ends_with(b"\n") => bytes.push(b'\n'),
            Emission::Acquire => {}
            Emission::Release(text) => bytes = text.as_bytes().to_vec(),
            Emission::Original => {}
        }
        if !self.context.is_empty() || !matches!(emission, Emission::Original) {
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

#[derive(Clone, Copy)]
enum Emission<'a> {
    Original,
    Acquire,
    Release(&'a str),
}
impl Emission<'_> {
    fn action(self) -> Action {
        match self {
            Self::Original => Action::Plain,
            Self::Acquire => Action::Acquire,
            Self::Release(_) => Action::Release,
        }
    }
}

fn format_number(n: f64) -> String {
    if n == 0.0 { "0".into() } else { n.to_string() }
}
