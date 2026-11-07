//! Evaluated operands retain their source-language types until code generation.
use crate::{expr::Value, source::Span, syntax};

#[derive(Clone, Debug)]
pub struct Operand {
    pub span: Span,
    pub value: Option<Value>,
    pub introduced: bool,
    pub unchecked: bool,
}

#[derive(Debug)]
pub struct Command<'a> {
    pub syntax: &'a syntax::Command,
    pub operands: Vec<Operand>,
}
