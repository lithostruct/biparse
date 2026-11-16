//! Module for failure-related traits, implementations and items.
//!
//! Refer to [`ParseFailure`] for the central trait exported.

use crate::{combinator::RecursiveId, failure::ParseHint, input::Input};
use std::{borrow::Cow, fmt::Debug};

/// Trait that [`Parser`](crate::parse::Parser) implementors interface with
/// to construct owned failure payload values,
/// arising from
pub trait ParseFailure: Clone + Debug {
    /// Constructs the failure value given its location and error information.
    /// Information need not be retained.
    fn failure_at<I: Input + ?Sized>(input: &I, cursor: I::Cursor, kind: FailureKind<I>) -> Self;

    /// Constructs the failure with a custom payload given its location and error information.
    fn custom_failure_at<I: Input + ?Sized, E: std::error::Error + 'static>(
        input: &I,
        cursor: I::Cursor,
        payload: E,
    ) -> Self;

    /// Constructs the composite failure at this location from the given failures from distinct alternatives.
    /// Branch failures may contain different locations than the one supplied in this call.
    /// Branch failures are symmetrical but provided in order of occurrence.
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
    /// - Combinator names need not be unique. Identify combinator identity by [`RecursiveId`]
    ///   or by applying [`Eq`] onto the [`UnboundedRecursion`] struct.
    /// - Composites are only identified as unbounded recursion failures if
    ///   their constituent branches are all unbounded recursions,
    ///   which may originate from different provenances.
    ///   - Unbounded recursion composites should return [`true`](bool) if [`Self::is_unbounded_recursion_at`]
    ///     would return true for any of its constituents.
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
        /// The expected input value, as returned by [`Parser::hint`](crate::parse::Parser::hint).
        hint: ParseHint,
        /// The cursor location at which the input diverges from the hint, if applicable.
        difference_at: Option<I::Cursor>,
    },
    /// Failure caused by the [`Fail`](crate::combinator::Fail) primitive combinator.
    FailPrimitive {
        /// Optional message to be transmitted.
        message: Option<Cow<'static, str>>,
    },

    /// Undefined recursive parser. This is always fatal.
    UndefinedParser {
        /// Optional name of the recursive parser.
        name: Option<Cow<'static, str>>,
    },
    /// Dropped recursive parser. This is always fatal.
    DroppedParser {
        /// Optional name of the recursive parser.
        name: Option<Cow<'static, str>>,
    },
    /// Non-growing repetition in [`Repetition`](crate::combinator::Repetition) combinators. This is always fatal.
    NonGrowingRepetition {
        /// The hint of the inner parser, which admits unconditional success without advancing the input.
        inner_hint: ParseHint,
    },

    /// Insufficient repetition in [`Repetition`](crate::combinator::Repetition) combinators.
    /// Typically supplied as supplementary information for a failure.
    InsufficientRepetition {
        /// The minimum number of repetitions expected.
        minimum_repetitions: usize,
        /// The repetition count at which this error is produced.
        in_repetition: usize,
    },
}

/// The struct that carries unbounded recursion failure information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UnboundedRecursion {
    /// Optional name of the recursive parser.
    pub name: Option<Cow<'static, str>>,
    /// The [`RecursiveId`] of the originating recursive parser.
    pub from: RecursiveId,
}
