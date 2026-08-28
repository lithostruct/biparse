//! Module for failure-related traits, implementations and items.
//!
//! Refer to [`ParseFailure`] for the central trait exported.

use derive_more::From;
use thiserror::Error;

use crate::input::{Input, LineCol};
use std::fmt::Debug;

/// The trait required by parser failure types to implement.
pub trait ParseFailure: Clone {
    /// Constructs the failure value given its location and error information.
    fn failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        kind: FailureKind<I, E>,
    ) -> Self;
    /// Constructs the composite failure at this location from the given failures from distinct branches.
    /// Branch failures may contain different locations than the one supplied in this call.
    /// Branch failures are provided in order of occurence. Maintain iteration order where applicable.
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self;
}

/// The struct that carries information on what kind of failure is encountered by the parser,
/// which is given to [`ParseFailure`] implementors for construction.
pub enum FailureKind<I: Input + ?Sized, E: std::error::Error> {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    ExpectedFound {
        expected: String,
        difference_at: Option<I::Cursor>,
    },
    /// Expected the end of the input, found an additional token.
    ExpectedEnd { found: I::Token },
    /// Zero-progress left recursion.
    ZeroProgressRecursion { name: Option<String> },
    /// Failure caused by the [`Fail`] primitive combinator.
    FailPrimitive,
    /// A custom error.
    Custom { error: E },
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
    fn composite<I: Input + ?Sized>(
        _: &I,
        _: I::Cursor,
        _: impl IntoIterator<Item = Self>,
    ) -> Self {
        ZeroSized
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
                difference_at,
            } => match difference_at {
                Some(difference_location) => {
                    let difference_line_col = input.to_line_col(difference_location);
                    let found = input
                        .peek(difference_location)
                        .map_or("end of input".into(), |item| I::describe_token(&item.token));
                    format!("expected {expected}, found {found} at location {difference_line_col}")
                }
                None => format!("expected {expected}"),
            },
            FailureKind::ExpectedEnd { found } => {
                format!("expected end-of-input, found {}", I::describe_token(&found))
            }
            FailureKind::ZeroProgressRecursion { name } => {
                format!(
                    "zero-progress recursion detected{}",
                    name.map(|s| format!(" in '{s}'")).unwrap_or("".into())
                )
            }
            FailureKind::FailPrimitive => "'Fail' primitive combinator encountered".to_string(),
            FailureKind::Custom { error } => error.to_string(),
        };
        (line_col, vec![message]).into()
    }
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        let mut messages = Vec::new();
        for mut branch in branches {
            if branch.position == line_col {
                messages.append(&mut branch.messages);
            } else {
                messages.push(branch.to_string());
            }
        }
        Simple {
            position: line_col,
            messages,
        }
    }
}
