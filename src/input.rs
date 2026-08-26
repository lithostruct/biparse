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
    type Cursor: Copy + Eq + Hash;
    /// The type of a single input in this input.
    type Token: Debug + Clone + Eq;

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
    /// - [`Err`]: The match is unsuccessful. The cursor points to the token where the mismatch occured.
    fn matches_at<'src>(
        &self,
        input: &'src I,
        cursor: I::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, I>, I::Cursor>;

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
    pub span: (C, C),
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
    pub advanced_cursor: I::Cursor,
    _marker: PhantomData<fn(&'src P)>,
}
impl<'src, P, I: Input + ?Sized> PatternMatch<'src, P, I> {
    pub fn new(payload: P, spans: Vec<Range<usize>>, advanced_cursor: I::Cursor) -> Self {
        PatternMatch {
            payload,
            spans,
            advanced_cursor,
            _marker: PhantomData,
        }
    }
}

/// The cursor type for `str`-based input.
/// The contained value is asserted to be at a UTF-8 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrIndex(usize);
impl From<StrIndex> for usize {
    fn from(val: StrIndex) -> Self {
        val.0
    }
}
impl StrIndex {
    pub fn into_usize(self) -> usize {
        self.into()
    }
}

impl Input for str {
    type Cursor = StrIndex;
    type Token = char;

    fn start(&self) -> Self::Cursor {
        StrIndex(0)
    }
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token, Self::Cursor>> {
        self[at.0..].chars().next().map(|inner| {
            let end_cursor = StrIndex(at.0 + inner.len_utf8());
            InputItem {
                token: inner,
                span: (at, end_cursor),
                next_cursor: end_cursor,
            }
        })
    }

    fn byte_offset(&self, at: Self::Cursor) -> usize {
        at.0
    }
    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self[start..end])
    }

    fn to_line_col(&self, at: Self::Cursor) -> LineCol {
        // This is done with no caching, so time-complexity is O(N).
        let mut line = 1;
        let mut column = 1;
        for (offset, ch) in self.char_indices() {
            if offset >= at.0 {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        LineCol { line, col: column }
    }
}

impl Pattern<str> for char {
    type Payload<'src> = &'src str;
    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, <str as Input>::Cursor> {
        let new_byte_offset = cursor.0 + self.len_utf8();
        input[cursor.0..]
            .starts_with(*self)
            .then_some(PatternMatch::new(
                &input[cursor.0..new_byte_offset],
                vec![(cursor.0)..(new_byte_offset)],
                StrIndex(new_byte_offset),
            ))
            .ok_or(cursor)
    }
    fn describe(&self) -> String {
        format!("character '{self}'")
    }
}
impl Pattern<str> for str {
    type Payload<'src> = &'src str;
    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, <str as Input>::Cursor> {
        let new_byte_offset = cursor.0 + self.len();
        input[cursor.0..]
            .starts_with(self)
            .then_some(PatternMatch::new(
                &input[cursor.0..new_byte_offset],
                vec![(cursor.0)..(new_byte_offset)],
                StrIndex(new_byte_offset),
            ))
            .ok_or_else(|| {
                // Iterate character-by-character to find the mismatch.
                let mut cursor = cursor;
                for ch in self.chars() {
                    match ch.matches_at(input, cursor) {
                        Ok(new_position) => {
                            cursor = new_position.advanced_cursor;
                            continue;
                        }
                        Err(error_cursor) => return error_cursor,
                    }
                }
                unreachable!("The whole pattern cannot match!")
            })
    }
    fn describe(&self) -> String {
        format!("string \"{self:?}\"")
    }
}
impl Pattern<str> for String {
    type Payload<'src> = &'src str;
    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, <str as Input>::Cursor> {
        self.as_str().matches_at(input, cursor)
    }
    fn describe(&self) -> String {
        self.as_str().describe()
    }
}
impl Pattern<str> for &str {
    type Payload<'src> = &'src str;
    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, <str as Input>::Cursor> {
        (*self).matches_at(input, cursor)
    }
    fn describe(&self) -> String {
        (*self).describe()
    }
}
