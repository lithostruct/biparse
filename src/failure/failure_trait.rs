//! Module for failure-related traits, implementations and items.
//!
//! Refer to [`ParseFailure`] for the central trait exported.

use derive_more::From;
use thiserror::Error;

use crate::{
    failure::{Descriptor, ParseHint},
    input::{Input, LineColumn},
};
use std::fmt::Debug;

/// The trait required by parser failure types to implement.
pub trait ParseFailure: Clone {
    /// Constructs the failure value given its location and error information.
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self;
    /// Constructs the failure with a custom payload given its location and error information.
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        payload: E,
    ) -> Self;
    /// Constructs the composite failure at this location from the given failures from distinct alternatices.
    /// Branch failures may contain different locations than the one supplied in this call.
    /// Branch failures are symmetrical but provided in order of occurence.
    /// Maintain iteration order where applicable.
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self;
    /// Constructs the failure with the addition of the supplemental information obtained.
    /// Information supplied is inherently asymmetrical. Retain information of the main failure where applicable.
    fn supplement(self, additional: Self) -> Self;
}

/// The struct that carries information on what kind of failure is encountered by the parser,
/// which is given to [`ParseFailure`] implementors for construction.
pub enum FailureKind<I: Input + ?Sized> {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    ExpectedFound {
        hint: ParseHint,
        difference_at: Option<I::Cursor>,
    },
    /// Unbounded recursion.
    UnboundedRecursion { name: Option<String> },
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    FailPrimitive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[error("parse failure encountered")]
/// Zero-sized failure type.
pub struct ZeroSized;
impl ParseFailure for ZeroSized {
    fn failure_at<I: Input + ?Sized>(_: &I, _: I::Cursor, _: FailureKind<I>) -> Self {
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
    fn supplement(self, _: Self) -> Self {
        ZeroSized
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Error)]
#[error("{0} parse failure encountered")]
/// Failure type containing only positional information.
pub struct PositionOnly(LineColumn);
impl ParseFailure for PositionOnly {
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, _: FailureKind<I>) -> Self {
        PositionOnly(input.to_line_col(cursor))
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        _: E,
    ) -> Self {
        PositionOnly(input.to_line_col(cursor))
    }
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        _: impl IntoIterator<Item = Self>,
    ) -> Self {
        PositionOnly(input.to_line_col(cursor))
    }
    fn supplement(self, _: Self) -> Self {
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
/// Failure type that flattens information into its string representation.
pub struct Simple {
    pub position: LineColumn,
    pub messages: Vec<String>,
}
impl ParseFailure for Simple {
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self {
        let line_col = input.to_line_col(cursor);
        let message: String = match kind {
            FailureKind::ExpectedFound {
                hint,
                difference_at,
            } => match difference_at {
                Some(difference_location) => {
                    let difference_line_col = input.to_line_col(difference_location);
                    let found = input
                        .peek(difference_location)
                        .map_or(Descriptor::EndOfInput, |item| input.describe(&item.token));
                    format!(
                        "{}found: {found} at location {difference_line_col}",
                        hint.to_expectation()
                    )
                }
                None => {
                    let mut text = hint.to_expectation();
                    if text.ends_with('\n') {
                        text.pop();
                    }
                    text
                }
            },
            FailureKind::UnboundedRecursion { name } => {
                format!(
                    "zero-progress recursion detected{}",
                    name.map(|s| format!(" in '{s}'")).unwrap_or("".into())
                )
            }
            FailureKind::FailPrimitive => "'Fail' primitive combinator encountered".to_string(),
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
    fn supplement(mut self, additional: Self) -> Self {
        self.messages.push(additional.to_string());
        self
    }
}

/// Enum used by the [`Rich`] failure type, representing the failure kinds of parsing.
///
/// Variants and fields closely resemble that of [`FailureKind`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum RichKind {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    #[error("{}{}",
        {
            let mut tree = hint.to_expectation();
            if tree.ends_with('\n') {
                tree.pop();
            }
            tree
        },
        if diverges_at.len() == 1 {
            format!("\nfound: {} at {}", diverges_at[0].1, diverges_at[0].0)
        } else { String::new() }
    )]
    ExpectedFound {
        hint: ParseHint,
        diverges_at: Vec<(LineColumn, Descriptor)>,
    },
    /// Zero-progress left recursion.
    #[error("unbounded recursion{}", .name.as_deref().unwrap_or(""))]
    UnboundedRecursion { name: Option<String> },
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    #[error("failure from primitive Fail combinator")]
    FailPrimitive,
    /// Composite failure, which cannot be further reduced.
    ///
    /// This variant is meaningful where the enclosing [`Rich`] value has a non-empty `supplemental` field.
    #[error("composite failure")]
    CompositeFailure,
    /// Custom failure from an [`std::error::Error`] implementor, as its textual representation.
    #[error("{0}")]
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("{position} {kind}{}",
    if !self.supplemental.is_empty() {
        format!("\nsupplemental diagnostics:{}",
            self.supplemental.iter()
                .map(|inner| {
                    let mut inner_str = inner.to_string();
                    inner_str.insert(0, '\n');
                    indent::indent_all_by(4, inner_str)
                })
                .collect::<String>()
        )
    } else { String::new() }
)]
/// Failure type that preserves most information supplied, with automated merging of related failure kinds.
pub struct Rich {
    pub position: LineColumn,
    pub kind: RichKind,
    pub supplemental: Vec<Self>,
}
impl ParseFailure for Rich {
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self {
        let kind = match kind {
            FailureKind::ExpectedFound {
                hint,
                difference_at,
            } => RichKind::ExpectedFound {
                hint,
                diverges_at: difference_at.map_or(Vec::new(), |diff_cursor| {
                    let diff_position = input.to_line_col(diff_cursor);
                    let descriptor = input
                        .peek(diff_cursor)
                        .map_or(Descriptor::EndOfInput, |item| input.describe(&item.token));
                    vec![(diff_position, descriptor)]
                }),
            },
            FailureKind::UnboundedRecursion { name } => RichKind::UnboundedRecursion { name },
            FailureKind::FailPrimitive => RichKind::FailPrimitive,
        };
        Self {
            kind,
            position: input.to_line_col(cursor),
            supplemental: Vec::new(),
        }
    }

    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        payload: E,
    ) -> Self {
        Self {
            position: input.to_line_col(cursor),
            kind: RichKind::Custom(payload.to_string()),
            supplemental: Vec::new(),
        }
    }

    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let position = input.to_line_col(cursor);
        let supplemental: Vec<Self> = branches.into_iter().collect();

        // Attempt unification of the common expected-found speculative error.
        let all_expected_found = supplemental.clone().into_iter().try_fold(
            (Vec::new(), Vec::new(), Vec::new()),
            |(mut hints, mut divergence, mut supplemental), mut inner| match inner.kind {
                RichKind::ExpectedFound {
                    hint,
                    mut diverges_at,
                } if inner.position == position => {
                    hints.push(hint);
                    divergence.append(&mut diverges_at);
                    supplemental.append(&mut inner.supplemental);
                    Ok((hints, divergence, supplemental))
                }
                _ => Err(()),
            },
        );
        if let Ok((hints, diverges_at, supplemental)) = all_expected_found {
            return Self {
                position,
                kind: RichKind::ExpectedFound {
                    hint: ParseHint::choice(hints),
                    diverges_at,
                },
                supplemental,
            };
        }

        // Composite failure flat-mapping.
        let supplemental = supplemental
            .into_iter()
            .flat_map(|elem| {
                if elem.kind == RichKind::CompositeFailure {
                    // Position information of inner composite lost.
                    // I consider this acceptable.
                    elem.supplemental
                } else {
                    vec![elem]
                }
            })
            .collect();

        Self {
            kind: RichKind::CompositeFailure,
            position,
            supplemental,
        }
    }
    fn supplement(mut self, additional: Self) -> Self {
        self.supplemental.push(additional);
        self
    }
}
