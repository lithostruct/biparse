use derive_more::Deref;
use rowan::SyntaxKind;

use crate::{
    input::{Input, InputItem, LineColumn, Pattern, PatternMatchResult},
    parse::ParseHint,
};

/// A string structure that caches line start information of its source.
/// Line starts are defined as the character following a newline sequence of
/// either LF `\n` or CRLF `\r\n` only.
///
/// This allows for line-column number conversion of O(lg N) time rather than O(N),
/// which may be useful for large input values and wide, speculative parsing,
/// where line-column calculation becomes significant.
///
/// # Note to [`Pattern<str>`] Implementors
///
/// [`Pattern<str>`] implementors are not blanket implemented as [`Pattern<CachedStr<'_>>`] due to trait coherence.
///
/// Hence, compatible [`str`]-based patterns should also be implemented for [`CachedStr`]
/// by deferring to the inner [`CachedStr::source()`] string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deref)]
pub struct CachedStr<'a> {
    #[deref]
    source: &'a str,
    linestarts: Vec<usize>,
}
impl<'a> CachedStr<'a> {
    /// Returns the source this cached string contains.
    pub fn source(&self) -> &str {
        self.source
    }
    /// Constructs a string with cached line start positions.
    pub fn new(source: &'a str) -> Self {
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
}
impl<'a> From<&'a str> for CachedStr<'a> {
    fn from(value: &'a str) -> Self {
        CachedStr::new(value)
    }
}
impl<'a> From<CachedStr<'a>> for &'a str {
    fn from(value: CachedStr<'a>) -> Self {
        value.source
    }
}

impl<'a> Input for CachedStr<'a> {
    type Cursor = <str as Input>::Cursor;
    type Token = <str as Input>::Token;

    fn byte_to_line_col(&self, at_offset: usize) -> LineColumn {
        let at_offset = at_offset.min(self.source.len());
        let line_index = self.linestarts.partition_point(|&start| start <= at_offset) - 1;
        let line_start = self.linestarts[line_index];
        let column_index = self.source()[line_start..at_offset].chars().count();
        LineColumn {
            line: line_index + 1,
            column: column_index + 1,
        }
    }

    // All other capabilities of CachedStr delegate to the underlying `str`.
    fn start(&self) -> Self::Cursor {
        self.source.start()
    }
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token, Self::Cursor>> {
        self.source.peek(at)
    }
    fn skipped_trivia_kind(&self) -> Option<rowan::SyntaxKind> {
        self.source.skipped_trivia_kind()
    }
    fn byte_offset(&self, at: Self::Cursor) -> usize {
        self.source.byte_offset(at)
    }
    fn slice_bytes(&self, start: usize, end: usize) -> std::borrow::Cow<'_, str> {
        self.source.slice_bytes(start, end)
    }
    fn describe_token(token: &Self::Token) -> String {
        <str as Input>::describe_token(token)
    }
}

// Trait re-implementations for `str`-based patterns.

impl<'a, K> Pattern<CachedStr<'a>> for (K, char)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        CachedStr<'a>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src CachedStr<'a>,
        cursor: <CachedStr<'a> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'a>>
    where
        CachedStr<'a>: 'src,
    {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}
impl<'a, K> Pattern<CachedStr<'a>> for (K, &str)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        CachedStr<'a>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src CachedStr<'a>,
        cursor: <CachedStr<'a> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'a>>
    where
        CachedStr<'a>: 'src,
    {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}
impl<'a, K> Pattern<CachedStr<'a>> for (K, String)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        CachedStr<'a>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src CachedStr<'a>,
        cursor: <CachedStr<'a> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'a>>
    where
        CachedStr<'a>: 'src,
    {
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
impl<'a, K> Pattern<CachedStr<'a>> for (K, regex::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        CachedStr<'a>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src CachedStr<'a>,
        cursor: <CachedStr<'a> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'a>>
    where
        CachedStr<'a>: 'src,
    {
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
impl<'a, K> Pattern<CachedStr<'a>> for (K, regex_lite::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        CachedStr<'a>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src CachedStr<'a>,
        cursor: <CachedStr<'a> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, CachedStr<'a>>
    where
        CachedStr<'a>: 'src,
    {
        self.matches_at(input.source(), cursor)
    }

    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}
