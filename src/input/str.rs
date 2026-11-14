//! Module containing data structures relating to [`str`]-based [`Input`] implementations.

use std::borrow::Cow;

use derive_more::Into;
use rowan::SyntaxKind;

use crate::{
    failure::{Descriptor, ParseHint},
    input::{Input, InputItem, LineColumn, Pattern, PatternMatch, PatternMatchResult},
};

pub use super::{cached_str::CachedStr, skipping_str::SkippingStr};

/// The cursor type for `str`-based input, which wraps a byyte offset into the source string.
/// The contained value is asserted to be at a UTF-8 boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct StrIndex(#[into] pub(super) usize);
impl StrIndex {
    pub fn into_usize(self) -> usize {
        self.into()
    }
}

impl Input for str {
    type Cursor = StrIndex;
    type Token<'src> = char;

    fn start(&self) -> Self::Cursor {
        StrIndex(0)
    }
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token<'_>, Self::Cursor>> {
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

    /// As the raw [`str`] representation does not cache linebreak positions,
    /// this function needs to traverse the whole string (at worst-case),
    /// resulting in time complexity `O(N)`.`
    fn byte_to_line_col(&self, at_offset: usize) -> LineColumn {
        let at_offset = at_offset.min(self.len());
        assert!(
            self.is_char_boundary(at_offset),
            "byte offset is not a UTF-8 character boundary"
        );

        let mut line = 1;
        let mut column = 1;
        for (offset, ch) in self.char_indices() {
            if offset >= at_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        LineColumn { line, column }
    }

    fn describe<'src>(&self, token: &Self::Token<'src>) -> Descriptor
    where
        Self: 'src,
    {
        Descriptor::Pattern {
            kind: "character".to_string(),
            kind_plural: "characters".to_string(),
            data: format!("'{token}'"),
        }
    }
    fn skipped_trivia_kind(&self) -> Option<rowan::SyntaxKind> {
        None
    }
}

/// Single matching on `str` for `char`.
impl<'src, K> Pattern<'src, str> for (K, char)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload = &'src str;
    fn matches_at(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, str> {
        if cursor.0 >= input.len() {
            return Err(Some(cursor));
        }
        match input.peek(cursor) {
            Some(item) => {
                if item.token == self.1 {
                    Ok(PatternMatch {
                        payload: &input[item.span.clone()],
                        cst_spans: vec![(self.0.clone().into(), item.span)],
                        next_cursor: item.next_cursor,
                    })
                } else {
                    Err(Some(cursor))
                }
            }
            None => Err(Some(cursor)),
        }
    }

    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Descriptor::Pattern {
            kind: "character".to_string(),
            kind_plural: "characters".to_string(),
            data: format!("'{}'", self.1),
        })
    }
}

/// Contiguous matching on [`str`] for [`&str`].
impl<'src, K> Pattern<'src, str> for (K, &str)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload = &'src str;
    fn matches_at(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, str>
where {
        if cursor.0 >= input.len() {
            return Err(Some(cursor));
        }
        let new_byte_offset = cursor.0 + self.1.len();
        if input[cursor.0..].starts_with(self.1) {
            let str_span = (cursor.0)..(new_byte_offset);
            // This is indeed intentional.
            // It doesn't make much sense to separate these into character-by-character spans
            // when the whole pattern matches contiguously.
            // That's just extra work for the `Just` combinator.
            #[allow(clippy::single_range_in_vec_init)]
            Ok(PatternMatch {
                payload: &input[cursor.0..new_byte_offset],
                cst_spans: vec![(self.0.clone().into(), str_span)],
                next_cursor: StrIndex(new_byte_offset),
            })
        } else {
            // Iterate character-by-character to find the mismatch.
            let mut cursor = cursor;
            let kind = self.0.clone().into();
            for ch in self.1.chars() {
                match (kind, ch).matches_at(input, cursor) {
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

    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Descriptor::Pattern {
            kind: "string".to_string(),
            kind_plural: "strings".to_string(),
            data: format!("\"{:?}\"", self.1),
        })
    }
}

/// Contiguous matching on [`str`] for [`String`], deferring to the inner [`&str`] implementation.
impl<'src, K> Pattern<'src, str> for (K, String)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload = &'src str;
    fn matches_at(
        &self,
        input: &'src str,
        cursor: <str as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, str>
where {
        (self.0.clone(), self.1.as_str()).matches_at(input, cursor)
    }

    fn hint(&self) -> ParseHint {
        <(K, &str) as Pattern<str>>::hint(&(self.0.clone(), self.1.as_str()))
    }
}
