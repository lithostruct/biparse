use derive_more::From;
use thiserror::Error;

use crate::input::{Input, LineCol};
use std::fmt::Debug;

pub trait ParseFailure {
    fn failure_at<I: Input, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        kind: FailureKind<I, E>,
    ) -> Self;
    fn merge(self, other: Self) -> Self;
}

pub enum FailureKind<I: Input, E: std::error::Error> {
    ExpectedFound {
        expected: String,
        found: Option<I::Token>,
        difference_at: I::Cursor,
    },
    LeftRecursion,
    FailPrimitive,
    Custom {
        error: E,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Error)]
#[error("({}, {}): {message}", position.line, position.column)]
pub struct Simple {
    position: LineCol,
    message: String,
}
impl ParseFailure for Simple {
    fn failure_at<I: Input, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        kind: FailureKind<I, E>,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        let message: String = match kind {
            FailureKind::ExpectedFound {
                expected,
                found,
                difference_at,
            } => {
                let location = input.to_line_col(difference_at);
                format!(
                    "expected {expected}, found {found:?} at ({}, {})",
                    location.line, location.column
                )
            }
            FailureKind::LeftRecursion => "Zero-progress left recursion detected".to_string(),
            FailureKind::FailPrimitive => "'Fail' primitive combinator encountered".to_string(),
            FailureKind::Custom { error } => error.to_string(),
        };
        (line_col, message).into()
    }
    fn merge(self, other: Self) -> Self {
        // for now, discard self.
        // this type does not retain enough type information to determine whether an error is important.
        other
    }
}
