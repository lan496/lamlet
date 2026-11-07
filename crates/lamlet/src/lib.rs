pub mod analysis;
pub mod codegen;
pub mod diagnostic;
pub mod expr;
pub mod hir;
mod lower;
pub mod schema;
pub mod source;
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
}

#[derive(Clone, Debug)]
pub struct Options {
    pub language: Language,
    pub strict: bool,
    pub max_expanded_lines: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            language: Language::Lammps,
            strict: false,
            max_expanded_lines: 100_000,
        }
    }
}

impl Compilation {
    pub fn report<'a>(&'a self, source: &'a Source) -> Report<'a> {
        Report::new(&source.path, &self.diagnostics)
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
    Compilation {
        output: if failed { vec![] } else { lowerer.output.bytes },
        diagnostics: lowerer.diagnostics,
        mappings: if failed {
            vec![]
        } else {
            lowerer.output.mappings
        },
    }
}
