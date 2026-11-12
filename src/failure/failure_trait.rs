//! Module for failure-related traits, implementations and items.
//!
//! Refer to [`ParseFailure`] for the central trait exported.

use thiserror::Error;

use crate::{
    combinator::RecursiveId,
    failure::{Descriptor, ParseHint},
    input::{Input, LineColumn},
};
use std::{borrow::Cow, fmt::Debug};

/// The trait required by parser failure types to implement.
pub trait ParseFailure: Clone {
    /// Constructs the failure value given its location and error information.
    /// Information need not be retained.
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
    ///
    /// Note that [`Self::is_unbounded_recursion_at`] invariants must be maintained.
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self;
    /// Constructs the failure with the addition of the supplemental information obtained.
    /// Information supplied is inherently asymmetrical. Retain information of the main failure where applicable.
    fn supplement(self, additional: Self) -> Self;

    /// Construct the failure corresponding to unbounded recursion.
    ///
    /// Unlike [`Self::failure_at`], unbounded recursion information *must* be retained in the failure payload generated,
    /// and be available for checking via [`Self::is_unbounded_recursion_at`].
    /// This is used to determine whether unbounded recursion failure should be promoted as fatal.
    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self;
    /// Checks whether `self` is an unbounded recursion failure
    /// constructed by the corresponding [`Self::unbounded_recursion(input, cursor, info)`](Self::unbounded_recursion).
    ///
    /// Unbounded recursion identification must observe:
    /// - Descriptions are not solely unique. Identify combinator identity by [`RecursiveId`]
    ///   or by applying [`Eq`] onto [`UnboundedRecursion`].
    /// - Composites are only identified as unbounded recursion failures if
    ///   their constituent branches are all unbounded recursions,
    ///   which may originate from different provenances.
    /// - Supplementary information is irrelevant.
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool;
}

/// The struct that carries additional information on what kind of failure is encountered by the parser,
/// which is given to [`ParseFailure`] implementors for construction.
pub enum FailureKind<I: Input + ?Sized> {
    /// Expected a certain pattern, failed to match.
    /// This may correspond to a cursor location in which the input diverges.
    ExpectedFound {
        hint: ParseHint,
        difference_at: Option<I::Cursor>,
    },
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    FailPrimitive { message: Option<Cow<'static, str>> },

    /// Undefined parser. This is always fatal.
    UndefinedParser { name: Option<Cow<'static, str>> },
    /// Dropped parser. This is always fatal.
    DroppedParser { name: Option<Cow<'static, str>> },
    /// Non-growing repetition in [`Many`](crate::combinator::Many). This is always fatal.
    NonGrowingRepetition { inner_hint: ParseHint },
}

/// The struct that carries unbounded recursion failure information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnboundedRecursion {
    pub name: Option<Cow<'static, str>>,
    pub from: RecursiveId,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("{}", if .0.is_empty() { String::from("parse failure") } else {
    String::from("unbounded recursion")
})]
/// Minimal failure type. Other than [`UnboundedRecursion`] information, all failure information is discarded.
pub struct Minimal(Vec<(usize, UnboundedRecursion)>);
impl ParseFailure for Minimal {
    fn failure_at<I: Input + ?Sized>(_: &I, _: I::Cursor, _: FailureKind<I>) -> Self {
        Minimal(Vec::new())
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        _: &I,
        _: I::Cursor,
        _: E,
    ) -> Self {
        Minimal(Vec::new())
    }
    fn composite<I: Input + ?Sized>(
        _: &I,
        _: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let mut recursive_info: Vec<(usize, UnboundedRecursion)> = Vec::new();
        for branch in branches {
            if branch.0.is_empty() {
                return Minimal(Vec::new());
            } else {
                recursive_info.extend(branch.0);
            }
        }
        Minimal(recursive_info)
    }
    fn supplement(self, _: Self) -> Self {
        self
    }

    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self {
        Minimal(vec![(input.byte_offset(cursor), info)])
    }
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool {
        let offset = input.byte_offset(cursor);
        self.0.contains(&(offset, info))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
#[error("{line_col} {}", if .recursive_info.is_empty() { String::from("parse failure") } else {
    String::from("unbounded recursion")
})]
/// Failure type containing only positional information.
pub struct PositionOnly {
    pub line_col: LineColumn,
    recursive_info: Vec<(LineColumn, UnboundedRecursion)>,
}
impl ParseFailure for PositionOnly {
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, _: FailureKind<I>) -> Self {
        PositionOnly {
            line_col: input.to_line_col(cursor),
            recursive_info: Vec::new(),
        }
    }
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
        input: &I,
        cursor: I::Cursor,
        _: E,
    ) -> Self {
        PositionOnly {
            line_col: input.to_line_col(cursor),
            recursive_info: Vec::new(),
        }
    }
    fn composite<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        branches: impl IntoIterator<Item = Self>,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        let mut recursive_info: Vec<(LineColumn, UnboundedRecursion)> = Vec::new();
        for branch in branches {
            if branch.recursive_info.is_empty() {
                return PositionOnly {
                    line_col,
                    recursive_info: Vec::new(),
                };
            } else {
                recursive_info.extend(branch.recursive_info);
            }
        }
        PositionOnly {
            line_col,
            recursive_info,
        }
    }
    fn supplement(self, _: Self) -> Self {
        self
    }

    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        PositionOnly {
            recursive_info: vec![(line_col, info)],
            line_col,
        }
    }
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool {
        let line_col = input.to_line_col(cursor);
        self.recursive_info.contains(&(line_col, info))
    }
}

// #[derive(Debug, Clone, PartialEq, Eq, Hash, From, Error)]
// #[error("{position}: {}",
//     if messages.len() == 1 {
//         messages[0].clone()
//     } else {
//         "\n\t".to_string() + &messages.join("\n\t")
//     })]
// /// Failure type that flattens information into its string representation.
// pub struct Simple {
//     pub position: LineColumn,
//     pub messages: Vec<String>,
// }
// impl ParseFailure for Simple {
//     fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self {
//         let line_col = input.to_line_col(cursor);
//         let message: String = match kind {
//             FailureKind::ExpectedFound {
//                 hint,
//                 difference_at,
//             } => match difference_at {
//                 Some(difference_location) => {
//                     let difference_line_col = input.to_line_col(difference_location);
//                     let found = input
//                         .peek(difference_location)
//                         .map_or(Descriptor::EndOfInput, |item| input.describe(&item.token));
//                     format!(
//                         "{}found: {found} at location {difference_line_col}",
//                         hint.to_expectation()
//                     )
//                 }
//                 None => {
//                     let mut text = hint.to_expectation();
//                     if text.ends_with('\n') {
//                         text.pop();
//                     }
//                     text
//                 }
//             },
//             FailureKind::FailPrimitive { message } => {
//                 format!("primitive failure{}",
//                     message.as_ref().map_or_else(|| String::new(), |inner| format!(": {inner}")))
//             }
//         };
//         (line_col, vec![message]).into()
//     }
//     fn custom_failure_at<I: Input + ?Sized, E: std::error::Error>(
//         input: &I,
//         cursor: I::Cursor,
//         payload: E,
//     ) -> Self {
//         let line_col = input.to_line_col(cursor);
//         let message = payload.to_string();
//         (line_col, vec![message]).into()
//     }
//     fn composite<I: Input + ?Sized>(
//         input: &I,
//         cursor: I::Cursor,
//         branches: impl IntoIterator<Item = Self>,
//     ) -> Self {
//         let line_col = input.to_line_col(cursor);
//         let mut messages = Vec::new();
//         for mut branch in branches {
//             if branch.position == line_col {
//                 messages.append(&mut branch.messages);
//             } else {
//                 messages.push(branch.to_string());
//             }
//         }
//         Simple {
//             position: line_col,
//             messages,
//         }
//     }
//     fn supplement(mut self, additional: Self) -> Self {
//         self.messages.push(additional.to_string());
//         self
//     }
// }

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
    #[error("unbounded recursion{}", if info.len() == 1 {
        info[1].1.name.as_deref().map_or_else(String::new, |inner| format!(": {inner}"))
    } else { String::from("") })]
    UnboundedRecursion {
        info: Vec<(LineColumn, UnboundedRecursion)>,
    },
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    #[error("primitive failure{}", message.as_ref().map_or_else(String::new, |inner| format!(": {inner}")))]
    FailPrimitive { message: Option<Cow<'static, str>> },

    /// Undefined parser. This is always fatal.
    #[error("undefined parser{}", name.as_ref().map_or_else(String::new, |inner| format!(": {inner}")))]
    UndefinedParser { name: Option<Cow<'static, str>> },

    /// Dropped parser. This is always fatal.
    #[error("dropped parser{}", name.as_ref().map_or_else(String::new, |inner| format!(": {inner}")))]
    DroppedParser { name: Option<Cow<'static, str>> },

    /// Non-growing repetition in [`Many`](crate::combinator::Many). This is always fatal.
    #[error("primitive failure{}", {
        let mut tree = inner_hint.to_expectation();
        if tree.ends_with('\n') {
            tree.pop();
        }
        format!(": {}", tree)
    })]
    NonGrowingRepetition { inner_hint: ParseHint },

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
            FailureKind::FailPrimitive { message } => RichKind::FailPrimitive { message },
            FailureKind::UndefinedParser { name } => RichKind::UndefinedParser { name },
            FailureKind::DroppedParser { name } => RichKind::DroppedParser { name },
            FailureKind::NonGrowingRepetition { inner_hint } => {
                RichKind::NonGrowingRepetition { inner_hint }
            }
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

        // Unbounded recursion failure check.
        let try_fold = supplemental.iter().try_fold(Vec::new(), |mut acc, inner| {
            // Invariant: unbounded recursion composites are labelled as unbounded recursion.
            // This does not inspect deeply-nested structures and relies on
            match &inner.kind {
                RichKind::UnboundedRecursion { info } => {
                    acc.extend_from_slice(info);
                    Ok(acc)
                }
                _ => Err(()),
            }
        });
        if let Ok(info) = try_fold {
            return Rich {
                position,
                kind: RichKind::UnboundedRecursion { info },
                supplemental,
            };
        }

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
        if self != additional {
            self.supplemental.push(additional);
        } // Redundant information identical to principal failure is discarded.
        self
    }

    fn unbounded_recursion<I: Input + ?Sized>(
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> Self {
        let line_col = input.to_line_col(cursor);
        Rich {
            position: line_col,
            kind: RichKind::UnboundedRecursion {
                info: vec![(line_col, info)],
            },
            supplemental: Vec::new(),
        }
    }
    fn is_unbounded_recursion_at<I: Input + ?Sized>(
        &self,
        input: &I,
        cursor: I::Cursor,
        info: UnboundedRecursion,
    ) -> bool {
        if let RichKind::UnboundedRecursion { info: existing } = &self.kind {
            existing.contains(&(input.to_line_col(cursor), info))
        } else {
            false
        }
    }
}
