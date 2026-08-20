
/// The trait describing all input types accepted by the [`Parser`](crate::parser::Parser) trait.
pub trait Input {
    /// The type of a singular token from this input.
    type Token;
    /// The type of the cursor, that allows for external users to refer into this input.
    type Cursor: Clone + Eq;

    /// Produces the next token in the input from the cursor.
    fn next(&self, cursor: Self::Cursor) -> Option<(Self::Cursor, Self::Token)>;
    /// Produces the string slice over the range specified by the two cursors.
    /// 
    /// Range is `start`-inclusive, `end`-exclusive.
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> &str;
}

/// The trait describing all pattern types that match onto a specific input type.
pub trait Pattern<I : Input + ?Sized> {
    /// The result of matching.
    type Matched<'src> where I : 'src;
    fn matches_at<'src>(&self, input: &'src I, cursor: I::Cursor) -> Option<(I::Cursor, Self::Matched<'src>)>;
}

impl Input for str {
    type Token = char;
    type Cursor = usize;
    
    fn next(&self, cursor: Self::Cursor) -> Option<(Self::Cursor, Self::Token)> {
        self[cursor..].char_indices().next()
    }
    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> &str {
        &self[start..end]
    }
}
impl Pattern<str> for char {
    type Matched<'src> = char;
    fn matches_at<'src>(&self, input: &'src str, cursor: <str as Input>::Cursor) -> Option<(<str as Input>::Cursor, Self::Matched<'src>)> {
        input.next(cursor).filter(|(_, inner)| inner == self)
    }
}
impl Pattern<str> for str {
    type Matched<'src> = &'src str;
    fn matches_at<'src>(&self, input: &'src str, cursor: <str as Input>::Cursor) -> Option<(<str as Input>::Cursor, Self::Matched<'src>)> {
        input[cursor..].starts_with(self).then_some((cursor + self.len(), &input[cursor..(cursor+self.len())]))
    }
}