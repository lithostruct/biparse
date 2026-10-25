//! Module for failure-related traits, implementations and items.
//!
//! Refer to [`ParseFailure`] for the central trait exported.

use derive_more::From;
use thiserror::Error;

use crate::{
    input::{Input, LineColumn},
    parse::ParseHint,
};
use std::fmt::Debug;

/// The trait required by parser failure types to implement.
pub trait ParseFailure: Clone {
    /// Constructs the failure value given its location and error information.
    fn failure_at<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        kind: ParseFailureKind<I>,
    ) -> Self;
    /// Constructs the failure with a custom payload given its location and error information.
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        payload: E,
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
pub enum ParseFailureKind<I: Input + ?Sized> {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    ExpectedFound {
        hint: ParseHint,
        difference_at: Option<I::Cursor>,
    },
    /// Zero-progress left recursion.
    ZeroProgressRecursion { name: Option<String> },
    /// Failure caused by the [`Fail`] primitive combinator.
    FailPrimitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[error("Parser error encountered.")]
pub struct ZeroSized;
impl ParseFailure for ZeroSized {
    fn failure_at<I: Input + ?Sized>(_: &I, _: I::Cursor, _: ParseFailureKind<I>) -> Self {
        ZeroSized
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        _: &I,
        _: I::Cursor,
        _: E,
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
    pub position: LineColumn,
    pub messages: Vec<String>,
}
impl ParseFailure for Simple {
    fn failure_at<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        kind: ParseFailureKind<I>,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        let message: String = match kind {
            ParseFailureKind::ExpectedFound {
                hint,
                difference_at,
            } => match difference_at {
                Some(difference_location) => {
                    let difference_line_col = input.to_line_col(difference_location);
                    let found = input
                        .peek(difference_location)
                        .map_or("end of input".into(), |item| {
                            input.describe_token(&item.token)
                        });
                    format!("expected {hint:?}, found {found} at location {difference_line_col}")
                }
                None => format!("expected {hint:?}"),
            },
            ParseFailureKind::ZeroProgressRecursion { name } => {
                format!(
                    "zero-progress recursion detected{}",
                    name.map(|s| format!(" in '{s}'")).unwrap_or("".into())
                )
            }
            ParseFailureKind::FailPrimitive => {
                "'Fail' primitive combinator encountered".to_string()
            }
        };
        (line_col, vec![message]).into()
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        payload: E,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        let message = payload.to_string();
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
