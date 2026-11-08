pub mod analysis;
pub mod capability;
pub mod codegen;
pub mod diagnostic;
pub mod documentation;
pub mod expr;
pub mod hir;
pub mod lint;
mod lower;
pub mod manual_lint;
pub mod module;
pub mod resources;
pub mod schema;
mod scoped;
pub mod semantics;
pub mod source;
pub mod stage;
pub mod syntax;
pub mod units;

use diagnostic::{Diagnostic, Report};
use source::Source;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    Lammps,
    Lamlet,
}

#[derive(Debug)]
pub struct Compilation {
    pub output: Vec<u8>,
    pub diagnostics: Vec<Diagnostic>,
    pub mappings: Vec<codegen::LineMapping>,
    pub resources: resources::Dependencies,
    pub physical_findings: Vec<semantics::Finding>,
    pub requirements: Vec<capability::Requirement>,
    pub stage: Option<stage::Plan>,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub language: Language,
    pub strict: bool,
    pub max_expanded_lines: usize,
    pub project: Option<resources::Project>,
    pub capabilities: Option<capability::Profile>,
    pub stage: Option<stage::Invocation>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            language: Language::Lammps,
            strict: false,
            max_expanded_lines: 100_000,
            project: None,
            capabilities: None,
            stage: None,
        }
    }
}

impl Compilation {
    pub fn report<'a>(&'a self, source: &'a Source) -> Report<'a> {
        let mut report = Report::new(&source.path, &self.diagnostics);
        report.physical_findings = &self.physical_findings;
        report
    }
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == diagnostic::Severity::Error)
    }
}

pub fn compile(source: &Source, language: Language) -> Compilation {
    compile_with_options(
        source,
        &Options {
            language,
            ..Options::default()
        },
    )
}

pub fn compile_with_options(source: &Source, options: &Options) -> Compilation {
    let mut lowerer = lower::Lowerer::new(source, options);
    match syntax::parse(&source.bytes, options.language == Language::Lamlet) {
        Ok(nodes) => lowerer.run(&nodes),
        Err(e) => lowerer.diagnostics.push(Diagnostic::new(
            source,
            e.span,
            diagnostic::Severity::Error,
            e.code,
            e.message,
        )),
    }
    let failed = lowerer
        .diagnostics
        .iter()
        .any(|d| d.severity == diagnostic::Severity::Error);
    if failed {
        lowerer.resources.dependencies.complete = false;
    }
    Compilation {
        output: if failed { vec![] } else { lowerer.output.bytes },
        diagnostics: lowerer.diagnostics,
        mappings: if failed {
            vec![]
        } else {
            lowerer.output.mappings
        },
        resources: lowerer.resources.dependencies,
        physical_findings: lowerer
            .semantics
            .findings
            .into_iter()
            .chain(lowerer.lint.findings)
            .chain(lowerer.manual_lint.findings)
            .collect(),
        requirements: lowerer.capabilities.requirements,
        stage: lowerer.stage.map(|mut plan| {
            plan.independent_execution &= !failed;
            plan
        }),
    }
}
