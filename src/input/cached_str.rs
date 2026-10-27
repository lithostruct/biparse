use std::borrow::Cow;

use derive_more::{AsRef, Deref};
use rowan::SyntaxKind;

use crate::{
    failure::{Descriptor, ParseHint},
    input::{Input, InputItem, LineColumn, Pattern, PatternMatchResult, StrIndex},
};

/// A string structure that caches line start information of its source.
/// Line starts are defined as the character following a newline sequence of
/// either LF `\n` or CRLF `\r\n` only.
///
/// This performs the non-essential optimization for calculating line starts,
/// thus taking `O(N)` time on construction and `O(lg N)` on subsequent lookup.
/// Hence, this input type might be beneficial for wide speculative parsing branches compared to [`str`].
///
/// # Note on [`Pattern`] Implementors
///
/// Due to trait coherence, [`Pattern<str>`] implementors do not blanket implement [`Pattern<CachedStr<'_>>`].
/// Implementors should use the following snippet to defer to the inner [`str`] implementation:
/// ```ignore
/// // Given an existing implementation of T : Pattern<str> where (...)
/// impl<'source> Pattern<CachedStr<'source>> for T
/// where
///     // (...)
/// {
///     type Payload<'src>
///         = <Self as Pattern<str>>::Payload<'src>
///     where
///         CachedStr<'source>: 'src;
///
///     fn matches_at<'src>(
///         &self,
///         input: &'src CachedStr<'source>,
///         cursor: <CachedStr<'source> as Input>::Cursor,
///     ) -> super::PatternMatchResult<'src, Self, CachedStr<'source>> {
///         self.matches_at(input.source(), cursor)
///     }
///     fn describe(&self) -> String {
///         <Self as Pattern<str>>::describe(self)
///     }
///     fn hint(&self) -> ParseHint {
///         <Self as Pattern<str>>::hint(self)
///     }
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref, AsRef)]
pub struct CachedStr<'source> {
    #[deref]
    #[as_ref]
    source: &'source str,
    linestarts: Vec<usize>,
}
impl<'source> CachedStr<'source> {
    /// Constructs a string with cached line start positions.
    pub fn new(source: &'source str) -> Self {
        let mut linestarts = vec![0];
        let mut current_offset = 0;

        for line in source.lines() {
            // text.lines() strips both \n and \r\n, so we must manually
            // calculate the full skipped stride including the newline characters.
            current_offset += line.len();

            // Peek into the remaining string to see if it used CRLF or LF
            let remaining = &source[current_offset..];
            if remaining.starts_with("\r\n") {
                current_offset += 2;
            } else if remaining.starts_with('\n') {
                current_offset += 1;
            } else {
                // end-of-input. Do not push a linestart, break.
                break;
            }

            // push the offset as the linestart offset of the next line.
            linestarts.push(current_offset);
        }

        CachedStr { source, linestarts }
    }
    /// View the underlying source string of `self`.
    pub const fn source(&self) -> &str {
        self.source
    }
    /// Uses cached linestart information to calculate the line and column of a byte offset in `O(lg N)` average time.
    pub fn to_line_col(&self, at_offset: usize) -> LineColumn {
        let line_index = self
            .linestarts
            .partition_point(|&start| start <= at_offset)
            .saturating_sub(1);
        let linestart = self.linestarts[line_index];

        let mut column_index = 0;
        for (char_offset, _) in self.source[linestart..].char_indices() {
            if char_offset >= at_offset {
                break;
            }
            column_index += 1;
        }
        LineColumn {
            line: line_index + 1,
            column: column_index + 1,
        }
    }
}
impl<'source> From<&'source str> for CachedStr<'source> {
    fn from(value: &'source str) -> Self {
        CachedStr::new(value)
    }
}
impl<'source> From<CachedStr<'source>> for &'source str {
    fn from(value: CachedStr<'source>) -> Self {
        value.source
    }
}

impl<'source> Input for CachedStr<'source> {
    type Cursor = StrIndex;
    type Token<'src>
        = char
    where
        Self: 'src;

    fn byte_to_line_col(&self, at_offset: usize) -> LineColumn {
        self.to_line_col(at_offset)
    }

    // Delegate to `str`.

    fn start(&self) -> Self::Cursor {
        self.source.start()
    }
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token<'_>, Self::Cursor>> {
        self.source.peek(at)
    }
    fn skipped_trivia_kind(&self) -> Option<SyntaxKind> {
        self.source.skipped_trivia_kind()
    }
    fn byte_offset(&self, at: Self::Cursor) -> usize {
        self.source.byte_offset(at)
    }
    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str> {
        self.source.slice_bytes(start, end)
    }
    fn describe<'src>(&self, token: &Self::Token<'src>) -> Descriptor
    where
        Self: 'src,
    {
        self.source().describe(token)
    }
}

// Pattern delegation to inner `str`.

/// Single matching on `str` for `char`.
impl<'src, 'source, K> Pattern<'src, CachedStr<'source>> for (K, char)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        CachedStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src CachedStr<'source>,
        cursor: <CachedStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'source>> {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

/// Contigious matching on [`str`] for [`&str`].
impl<'src, 'source, K> Pattern<'src, CachedStr<'source>> for (K, &str)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        CachedStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src CachedStr<'source>,
        cursor: <CachedStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'source>> {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

/// Contiguous matching on [`str`] for [`String`], deferring to the inner [`&str`] implementation.
impl<'src, 'source, K> Pattern<'src, CachedStr<'source>> for (K, String)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        CachedStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src CachedStr<'source>,
        cursor: <CachedStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'source>> {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

#[cfg(feature = "regex")]
/// `regex` support.
impl<'src, 'source, K> Pattern<'src, CachedStr<'source>> for (K, regex::Regex)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        CachedStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src CachedStr<'source>,
        cursor: <CachedStr<'source> as Input>::Cursor,
    ) -> super::PatternMatchResult<'src, Self, CachedStr<'source>> {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

#[cfg(feature = "regex")]
/// `regex_lite` support.
impl<'src, 'source, K> Pattern<'src, CachedStr<'source>> for (K, regex_lite::Regex)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        CachedStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src CachedStr<'source>,
        cursor: <CachedStr<'source> as Input>::Cursor,
    ) -> super::PatternMatchResult<'src, Self, CachedStr<'source>> {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}
