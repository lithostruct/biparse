//! Module for failure-related traits, implementations and items.
//!
//! Refer to [`ParseFailure`] for the central trait exported.

use derive_more::From;
use thiserror::Error;

use crate::input::{Input, LineCol};
use std::{cmp::Ordering, fmt::Debug};

/// The trait required by parser failure types to implement.
pub trait ParseFailure: Clone {
    fn failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        kind: FailureKind<I, E>,
    ) -> Self;
    fn merge(self, other: Self) -> Self;
}

/// The struct that carries information on what kind of failure is encountered by the parser,
/// which is given to [`ParseFailure`] implementors for construction.
pub enum FailureKind<I: Input + ?Sized, E: std::error::Error> {
    ExpectedFound {
        expected: String,
        found: Option<I::Token>,
        difference_at: I::Cursor,
    },
    ExpectedEnd {
        found: I::Token,
    },
    LeftRecursion {
        description: Option<String>,
        diagnostic_name: String,
    },
    FailPrimitive,
    Custom {
        error: E,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[error("Parser error encountered.")]
pub struct ZeroSized;
impl ParseFailure for ZeroSized {
    fn failure_at<I: Input + ?Sized, E: std::error::Error>(
        _: &I,
        _: I::Cursor,
        _: FailureKind<I, E>,
    ) -> Self {
        ZeroSized
    }
    fn merge(self, _: Self) -> Self {
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, From, Error)]
#[error("{position}: {}",
    if messages.len() == 1 {
        messages[0].clone()
    } else {
        "\n\t".to_string() + &messages.join("\n\t")
    })]
pub struct Simple {
    pub position: LineCol,
    pub messages: Vec<String>,
}
impl ParseFailure for Simple {
    fn failure_at<I: Input + ?Sized, E: std::error::Error>(
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
                    location.line, location.col
                )
            }
            FailureKind::ExpectedEnd { found } => {
                format!("expected end-of-input, found {found:?}",)
            }
            FailureKind::LeftRecursion {
                description,
                diagnostic_name: closure_typename,
            } => {
                format!(
                    "Zero-progress left recursion detected at '{}'",
                    description.unwrap_or(closure_typename)
                )
            }
            FailureKind::FailPrimitive => "'Fail' primitive combinator encountered".to_string(),
            FailureKind::Custom { error } => error.to_string(),
        };
        (line_col, vec![message]).into()
    }
    fn merge(mut self, mut other: Self) -> Self {
        // Retain the most advanced error position, for now.
        match self.position.cmp(&other.position) {
            Ordering::Less => other,
            Ordering::Equal => {
                self.messages.append(&mut other.messages);
                self
            }
            Ordering::Greater => self,
        }
    }
}
