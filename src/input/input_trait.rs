//! Module for input-related traits, implementations and items.
//!
//! Refer to [`Input`] and [`Pattern`] for the central traits exported.

use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    hash::Hash,
    ops::Range,
};

use rowan::SyntaxKind;

use crate::failure::{Descriptor, ParseHint};

/// The trait describing all input types accepted by the [`Parser`](crate::parse::Parser) trait.
///
/// This trait describes a UTF-8 [`str`]-based input, indexed via a given cursor type (which delineates each token item),
/// and the byte offset corresponding to the underlying [`str`] source.
///
/// Implementors include:
/// - [`str`].
/// - [`LexerStream`](super::logos::LexerStream), which wraps [`logos::Lexer`]
///   for any token type satisfying [`logos::Logos`] and [`Clone`]. Requires default feature `logos`.
/// - [`SkippingStr`](super::skipping_str::SkippingStr), which implicitly skips trivia characters (such as whitespace),
///   thus not requiring them to be explicitly matched on the parser combinator level.
pub trait Input {
    /// The type of the cursor, that allows for external users to refer into this input.
    type Cursor: Copy + Ord + Hash;
    /// The type of a single input in this input.
    type Token<'src>
    where
        Self: 'src;

    /// Produces a cursor that points to the start of the input. Ideally has O(1) time complexity.
    ///
    /// For input containing gaps (see [`Input::skipped_trivia_kind`]),
    /// the starting cursor may point after a skipped leading span.
    fn start(&self) -> Self::Cursor;

    /// Peeks at the token in this input pointed at by a cursor.
    /// [`None`] signifies end-of-input.
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token<'_>, Self::Cursor>>;

    /// Returns the default [`SyntaxKind`] returned for skipped byte spans within the current stream.
    ///
    /// Two distinct modes are supported:
    /// - [`None`]: Input tokens form gapless contiguous input.
    ///   Missing spans are impossible and considered invariant violations.
    /// - [`Some(rowan::SyntaxKind)`](Some): Input containing gaps. Input tokens contain the "main" span,
    ///   followed by a trailing span that is automatically skipped and not emitted as a token.
    ///   It is the responsibility of [`Parser`](crate::parse::Parser) implementors to account for skipped trivia
    ///   to enable lossless CST representation.
    fn skipped_trivia_kind(&self) -> Option<rowan::SyntaxKind>;

    /// Returns the conceptual starting byte offset into the source string that this cursor points to.
    /// In the event the cursor points past the source, return the byte length of the source string.
    ///
    /// The returned value **must** correspond to a UTF-8 character boundary in the source string.
    fn byte_offset(&self, at: Self::Cursor) -> usize;
    /// Produces the string slice over the range specified by the two cursors.
    ///
    /// Range is `start`-inclusive, `end`-exclusive.
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> Cow<'_, str> {
        self.slice_bytes(self.byte_offset(start), self.byte_offset(end))
    }
    /// Produces the string slice over the range specified by the two cursors.
    ///
    /// Range is `start`-inclusive, `end`-exclusive.
    /// `start` and `end` must be at UTF-8 character boundaries and bounded within the source input.
    ///
    /// # Panics
    ///
    /// Panics if `start` and `end` are not at UTF-8 character boundaries,
    /// or if either exceeds the bounds of this source string.
    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str>;

    /// Calculates the one-indexed line-column number that this cursor points towards.
    ///
    /// If the cursor points past the source string length,
    /// return the line-column number corresponding to end-of-input,
    /// typically one column past the final character.
    fn to_line_col(&self, at: Self::Cursor) -> LineColumn {
        self.byte_to_line_col(self.byte_offset(at))
    }
    /// Calculates the one-indexed line-column number that this byte offset points into.
    /// `at_offset` must be at a UTF-8 character boundary.
    ///
    /// If the byte offset exceeds the source string length,
    /// return the line-column number corresponding to end-of-input,
    /// typically one column past the final character.
    ///
    /// # Panics
    ///
    /// Panics if `at_offset` is not at a UTF-8 character boundary.
    fn byte_to_line_col(&self, at_offset: usize) -> LineColumn;

    /// Provides the description of a token in the form of an [`Descriptor`] object.
    ///
    /// Used for diagnostic construction and error reporting.
    fn describe<'src>(&self, token: &Self::Token<'src>) -> Descriptor
    where
        Self: 'src;
}

/// The result of pattern matching, as used by [`Pattern::matches_at`].
pub type PatternMatchResult<'src, S, I> = Result<
    PatternMatch<<S as Pattern<'src, I>>::Payload, <I as Input>::Cursor>,
    Option<<I as Input>::Cursor>,
>;

/// The trait describing pattern types that match onto a specific [`Input`] type.
///
/// Implementors include, for their respective [`Input`] types:
/// - [`str`], [`SkippingStr`](super::skipping_str::SkippingStr):
///   - [`(K, char)`](char)
///   - [`(K, &str)`](str)
///   - [`(K, String)`](String)
///   - [`(K, regex::Regex)`](regex::Regex) with default feature `regex`.
///   - [`(K, regex_lite::Regex)`](regex_lite::Regex) with default feature `regex`.
/// - [`LexerStream<'_, T>`](super::logos::LexerStream) with default feature `logos`:
///   - `(K, T)`
///   - `T` satisfying [`Eq`]
///   - [`WithText<P>`](super::logos::WithText) where `P` is a compatible pattern.
///
/// where `K` satisfies [`Clone`] and [`Into<rowan::SyntaxKind>`](Into).
///
/// All [`Input`] implementors may use [`Matches`](super::matches::Matches),
/// [`AnyOf`](super::composite_patterns::AnyOf) and [`AllOf`](super::composite_patterns::AllOf)
/// as universal patterns.
pub trait Pattern<'src, I: Input + ?Sized + 'src> {
    /// The type of the payload.
    type Payload;
    /// Checks whether self matches with the input at the specified cursor.
    ///
    /// - [`Ok`]: The match is successful, and the contained cursor is the position right after the match is consumed.
    /// - [`Err`]: The match is unsuccessful.
    ///   The cursor points to the token where the mismatch occurred, such information is available.
    ///
    /// Matching must not backtrack;
    /// The field `next_cursor` must be strictly be [`Ordering::Greater`](std::cmp::Ordering::Greater) than the `input` cursor.
    ///
    /// The cursor and byte spans returned within [`PatternMatch`] *must* be on UTF-8 character boundaries.
    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I>;

    /// Returns the [`ParseHint`] represented by this pattern.
    ///
    /// Unlike in [`Parser`](crate::parse::Parser), this method is not default-implemented
    /// as patterns may have non-trivial information that is useful to error reporting.
    /// Implementors may (but are discouraged from) returning the default value [`ParseHint::Opaque`].
    fn hint(&self) -> ParseHint;
}

/// Information relating to tokens produced from [`Input::peek`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputItem<T, C> {
    /// The token `T` corresponding to some [`Input::Token`].
    pub token: T,
    /// The contiguous span this token takes up.
    pub span: Range<usize>,
    /// The next cursor this input should advance to.
    pub next_cursor: C,
}
impl<T, C> InputItem<T, C> {
    /// Transforms this
    pub fn into_match_info(self, kind: SyntaxKind) -> PatternMatch<T, C> {
        PatternMatch {
            payload: self.token,
            cst_spans: vec![(kind, self.span)],
            next_cursor: self.next_cursor,
        }
    }
}

/// One-indexed line and column number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineColumn {
    /// 1-indexed line number.
    pub line: usize,
    /// 1-indexed column number, corresponding to the number of Unicode scalar values [`char`] in this line.
    pub column: usize,
}
impl Display for LineColumn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.line, self.column)
    }
}

/// Output information regarding [`Pattern::matches_at`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternMatch<P, C> {
    /// The payload type.
    pub payload: P,
    /// A sequence of kinds + spans, corresponding to a CST token to be constructed.
    /// Spans must be in order, and must not overlap.
    pub cst_spans: Vec<(SyntaxKind, Range<usize>)>,
    /// The next cursor returned by the match.
    /// Matching must not backtrack;
    /// `next_cursor` must be strictly be [`Ordering::Greater`](std::cmp::Ordering::Greater) than the input cursor of the match.
    pub next_cursor: C,
}
