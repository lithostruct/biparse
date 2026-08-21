use derive_more::Into;

/// The trait describing all input types accepted by the [`Parser`](crate::parser::Parser) trait.
pub trait Input {
    /// The type of the cursor, that allows for external users to refer into this input.
    type Cursor: Clone + Eq;

    /// Produces a cursor that points to the start of the input.
    fn start(&self) -> Self::Cursor;
    /// Produces the string slice over the range specified by the two cursors.
    ///
    /// Range is `start`-inclusive, `end`-exclusive.
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> &str;
}

/// The trait describing all pattern types that match onto a specific input type.
pub trait Pattern<I: Input + ?Sized> {
    /// Returns the end position of the cursor, if `self` matches onto the input at the specified cursor.
    fn matches_at(&self, input: &I, cursor: I::Cursor) -> Option<I::Cursor>;
}

/// The cursor type for `str`-based input.
/// The contained value is asserted to be at a UTF-8 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct StrIndex(usize);

impl Input for str {
    type Cursor = StrIndex;

    fn start(&self) -> Self::Cursor {
        StrIndex(0)
    }
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> &str {
        &self[start.0..end.0]
    }
}
impl Pattern<str> for char {
    fn matches_at(
        &self,
        input: &str,
        cursor: <str as Input>::Cursor,
    ) -> Option<<str as Input>::Cursor> {
        input[cursor.0..]
            .starts_with(*self)
            .then_some(StrIndex(cursor.0 + self.len_utf8()))
    }
}
impl Pattern<str> for str {
    fn matches_at(
        &self,
        input: &str,
        cursor: <str as Input>::Cursor,
    ) -> Option<<str as Input>::Cursor> {
        input[cursor.0..]
            .starts_with(self)
            .then_some(StrIndex(cursor.0 + self.len()))
    }
}
