use std::borrow::Cow;

use crate::input::{Input, InputItem, LineCol, Pattern, PatternMatch};

/// The cursor type for `str`-based input.
/// The contained value is asserted to be at a UTF-8 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrIndex(pub(super) usize);
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
                span: (at.0)..(end_cursor.0),
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

    fn describe_token(token: &Self::Token) -> String {
        format!("character '{token}'")
    }
}

impl Pattern<str> for char {
    type Payload<'src> = &'src str;
    fn matches_at<'src>(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, Option<<str as Input>::Cursor>> {
        if cursor.0 >= input.len() {
            return Err(Some(cursor));
        }
        match input.peek(cursor) {
            Some(item) => {
                if item.token == *self {
                    Ok(PatternMatch::new(
                        &input[item.span.clone()],
                        vec![item.span],
                        item.next_cursor,
                    ))
                } else {
                    Err(Some(cursor))
                }
            }
            None => Err(Some(cursor)),
        }
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
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, Option<<str as Input>::Cursor>> {
        if cursor.0 >= input.len() {
            return Err(Some(cursor));
        }
        let new_byte_offset = cursor.0 + self.len();
        if input[cursor.0..].starts_with(self) {
            // This is indeed intentional.
            // It doesn't make much sense to separate these into character-by-character spans
            // when the whole pattern matches contiguously.
            // That's just extra work for the `Just` combinator.
            #[allow(clippy::single_range_in_vec_init)]
            Ok(PatternMatch::new(
                &input[cursor.0..new_byte_offset],
                vec![(cursor.0)..(new_byte_offset)],
                StrIndex(new_byte_offset),
            ))
        } else {
            // Iterate character-by-character to find the mismatch.
            let mut cursor = cursor;
            for ch in self.chars() {
                match ch.matches_at(input, cursor) {
                    Ok(new_position) => {
                        cursor = new_position.next_cursor;
                        continue;
                    }
                    Err(error_cursor) => {
                        return Err(Some(error_cursor.expect(
                            "<char as Pattern<I>> always has a meaningful error cursor!",
                        )));
                    }
                }
            }
            unreachable!("The whole pattern cannot match!")
        }
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
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, Option<<str as Input>::Cursor>> {
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
    ) -> Result<PatternMatch<'src, Self::Payload<'src>, str>, Option<<str as Input>::Cursor>> {
        (*self).matches_at(input, cursor)
    }
    fn describe(&self) -> String {
        (*self).describe()
    }
}
