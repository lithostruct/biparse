use derive_more::Into;
use std::fmt::Debug;

/// The trait describing all input types accepted by the [`Parser`](crate::parser::Parser) trait.
pub trait Input {
    /// The type of the cursor, that allows for external users to refer into this input.
    type Cursor: Copy + Eq;
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
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> &str;
    /// Calculates the one-indexed line-column number that this cursor points towards.
    fn to_line_col(&self, at: Self::Cursor) -> LineCol;
}

/// The trait describing all pattern types that match onto a specific input type.
pub trait Pattern<I: Input + ?Sized> {
    /// Checks whether self matches with the input at the specified cursor.
    ///
    /// - [`Ok`]: The match is successful, and the cursor is the position right after the match is consumed.
    /// - [`Err`]: The match is unsuccessful. The cursor points to the token where the mismatch occured.
    fn matches_at(&self, input: &I, cursor: I::Cursor) -> Result<I::Cursor, I::Cursor>;
    /// Writes a description of what is expected by `self` as a string.
    ///
    /// This is used for error reporting:
    /// Assume a typical format of "expected [DESCRIPTION], got token [INPUT TOKEN]".
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
    pub column: usize,
}

/// The cursor type for `str`-based input.
/// The contained value is asserted to be at a UTF-8 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct StrIndex(usize);

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
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> &str {
        &self[start.0..end.0]
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
        LineCol { line, column }
    }
}
impl Pattern<str> for char {
    fn matches_at(
        &self,
        input: &str,
        cursor: <str as Input>::Cursor,
    ) -> Result<<str as Input>::Cursor, <str as Input>::Cursor> {
        input[cursor.0..]
            .starts_with(*self)
            .then_some(StrIndex(cursor.0 + self.len_utf8()))
            .ok_or(cursor)
    }
    fn describe(&self) -> String {
        format!("character '{self}'")
    }
}
impl Pattern<str> for str {
    fn matches_at(
        &self,
        input: &str,
        cursor: <str as Input>::Cursor,
    ) -> Result<<str as Input>::Cursor, <str as Input>::Cursor> {
        input[cursor.0..]
            .starts_with(self)
            .then_some(StrIndex(cursor.0 + self.len()))
            .ok_or_else(|| {
                // Iterate character-by-character to find the mismatch.
                let mut cursor = cursor;
                for ch in self.chars() {
                    match ch.matches_at(input, cursor) {
                        Ok(new_position) => {
                            cursor = new_position;
                            continue;
                        }
                        Err(erron_cursor) => return erron_cursor,
                    }
                }
                unreachable!("The whole pattern cannot match!")
            })
    }
    fn describe(&self) -> String {
        format!("string \"{self:?}\"")
    }
}
