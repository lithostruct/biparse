use derive_more::From;
use thiserror::Error;

use crate::input::{Input, Pattern};
use std::fmt::Debug;

pub trait ParserFailure {
    fn from_provenance<I : Input>(input: &I, cursor: I::Cursor, context: ErrorContext) -> Self;
    fn merge(self, other: Self) -> Self;
}

pub enum ErrorContext {
    FailPrimitive,
    MatchFailure(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From)]
pub struct Simple(String);
impl ParserFailure for Simple {
    fn from_provenance<I : Input>(_: &I, _: I::Cursor, context: ErrorContext) -> Self {
        match context {
            ErrorContext::FailPrimitive => "Fail primitive encountered".to_owned().into(),
            ErrorContext::MatchFailure(pattern) => format!("Could not match on expected pattern '{pattern}'").into()
        }
    }
    fn merge(self, other: Self) -> Self {
        // for now, discard self.
        // this type does not retain enough type information to determine whether an error is important.
        other
    }
}