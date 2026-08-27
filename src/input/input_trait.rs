//! Module for input-related traits, implementations and items.
//!
//! Refer to [`Input`] and [`Pattern`] for the central traits exported.

use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    hash::Hash,
    marker::PhantomData,
    ops::Range,
};

/// The trait describing all input types accepted by the [`Parser`](crate::parse::Parser) trait.
pub trait Input {
    /// The type of the cursor, that allows for external users to refer into this input.
    type Cursor: Copy + Ord + Hash;
    /// The type of a single input in this input.
    type Token: Clone + Eq;

    /// Produces a cursor that points to the start of the input.
    fn start(&self) -> Self::Cursor;

    /// Peeks at the token in this input pointed at by a cursor.
    /// [`None`] signifies end-of-input.
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token, Self::Cursor>>;

    /// Produces the string slice over the range specified by the two cursors.
    ///
    /// Range is `start`-inclusive, `end`-exclusive.
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> Cow<'_, str> {
        self.slice_bytes(self.byte_offset(start), self.byte_offset(end))
    }

    /// Returns the conceptual byte offset into the source string that this cursor points to.
    ///
    /// The returned value must be at an UTF-8 boundary.
    fn byte_offset(&self, at: Self::Cursor) -> usize;
    /// Produces the string slice over the range specified by the two cursors.
    ///
    /// Range is `start`-inclusive, `end`-exclusive. `start` and `end` must be at UTF-8 character boundaries.
    ///
    /// # Panics
    ///
    /// Panics if `start` and `end` are not at UTF-8 character boundaries.
    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str>;

    /// Calculates the one-indexed line-column number that this cursor points towards.
    fn to_line_col(&self, at: Self::Cursor) -> LineCol;
    /// Displays the description of a token.
    fn describe_token(token: &Self::Token) -> String;
}

/// The trait describing all pattern types that match onto a specific input type.
pub trait Pattern<I: Input + ?Sized> {
    /// The type of the payload.
    type Payload<'src>
    where
        I: 'src;
    /// Checks whether self matches with the input at the specified cursor.
    ///
    /// - [`Ok`]: The match is successful, and the cursor is the position right after the match is consumed.
    /// - [`Err`]: The match is unsuccessful.
    ///   The cursor points to the token where the mismatch occured, if any other than a generic "failure to match".
    /// 
    /// The cursor and byte spans returned within `PatternMatch` *must* be on UTF-8 character boundary.
    fn matches_at<'src>(
        &self,
        input: &'src I,
        cursor: I::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, I>, Option<I::Cursor>>;

    /// Writes a description of what is expected by `self` as a string.
    ///
    /// This is used for error reporting:
    /// Assume a typical format of "expected \<description\>, got \<token\>".
    fn describe(&self) -> String;
}

/// Information relating to tokens from [`Input::peek`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputItem<T, C> {
    pub token: T,
    pub span: Range<usize>,
    pub next_cursor: C,
}

/// One-indexed line and column number.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineCol {
    pub line: usize,
    pub col: usize,
}
impl Display for LineCol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.line, self.col)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PatternMatch<'src, P, I: Input + ?Sized> {
    pub payload: P,
    pub spans: Vec<Range<usize>>,
    pub next_cursor: I::Cursor,
    _marker: PhantomData<fn(&'src P)>,
}
impl<'src, P, I: Input + ?Sized> PatternMatch<'src, P, I> {
    pub fn new(payload: P, spans: Vec<Range<usize>>, next_cursor: I::Cursor) -> Self {
        PatternMatch {
            payload,
            spans,
            next_cursor,
            _marker: PhantomData,
        }
    }
}