pub mod diagnostic;
pub mod source;
pub mod syntax;

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

pub fn compile(source: &Source, _language: Language) -> Compilation {
    let _lines = syntax::logical_lines(&source.bytes);
    Compilation {
        output: source.bytes.clone(),
        diagnostics: vec![],
    }
}
