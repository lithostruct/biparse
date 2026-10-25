use std::borrow::Cow;

use rowan::SyntaxKind;

use crate::{
    input::{CachedStr, Input, InputItem, LineColumn, Pattern, PatternMatchResult, StrIndex},
    parse::ParseHint,
};

/// A [`str`]-based [`Input`] implementor that implicitly skips trivia at the character level after pattern-matching constructs.
///
/// This is typically used to discard whitespace for whitespace-insensitive grammars.
///
/// # Notes on Matching on Trivia Characters and [`Pattern`] Atomicity
///
/// Configured trivia is skipped before and after each atomic [`Pattern`] match.
/// Cursors therefore identify the next non-trivia character, or end of input.
/// Atomic patterns may consume trivia characters as part of their match,
/// but cannot match leading trivia before the current cursor.
/// Non-atomic patterns (notably [`AllOf`](crate::input::AllOf)) consumes trivia between each successful element matched.
///
/// Prefer non-skipping ([`str`], [`CachedStr<'source>`])
/// or configurable ([`LexerStream`](crate::input::logos::LexerStream)) input
/// when trivia must be matched explicitly.
///
/// # Note to [`Pattern`] Implementors:
///
/// Due to trait coherence, [`Pattern<str>`] implementors do not blanket implement [`Pattern<SkippingStr<'_>>`].
/// Implementors should use the following snippet to defer to the common implementation where applicable:
/// ```ignore
/// // Given an existing implementation of T : Pattern<str> where (...)
/// impl<'source> Pattern<SkippingStr<'source>> for T
/// where
///     // (...)
/// {
///     type Payload<'src> = <Self as Pattern<str>>::Payload<'src>
///     where
///         SkippingStr<'source>: 'src;
///
///     fn matches_at<'src>(
///         &self,
///         input: &'src SkippingStr<'source>,
///         cursor: <SkippingStr<'source> as Input>::Cursor,
///     ) -> PatternMatchResult<'src, Self, SkippingStr<'source>> {
///         input.match_with_skipping(self, cursor)
///     }
///     fn describe(&self) -> String {
///         <Self as Pattern<str>>::describe(self)
///     }
///     fn hint(&self) -> ParseHint {
///         <Self as Pattern<str>>::hint(self)
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SkippingStr<'source> {
    cached: CachedStr<'source>,
    predicate: fn(char) -> bool,
    skipped_kind: SyntaxKind,
    start_cursor: StrIndex,
}
impl<'source> SkippingStr<'source> {
    /// Constructs an input that treats Unicode whitespace as trivia, identified via [`char::is_whitespace`].
    /// Trivia is implicitly skipped after pattern-matching constructs.
    ///
    /// For ASCII-based whitespace, see [`SkippingStr::ascii_whitespace`].
    pub fn whitespace<'root>(source: &'root str, kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'source,
    {
        Self::new(source, kind, char::is_whitespace)
    }
    /// Constructs an input that treats ASCII whitespace as trivia, identified via [`char::is_ascii_whitespace`].
    /// Trivia is implicitly skipped after pattern-matching constructs.
    ///
    /// For ASCII-based whitespace, see [`SkippingStr::whitespace`].
    pub fn ascii_whitespace<'root>(source: &'root str, kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'source,
    {
        // Why are the ASCII-based character methods `fn(&char)` instead of `fn(char)`?
        Self::new(source, kind, |ch| ch.is_ascii_whitespace())
    }

    /// Constructs an input that uses the given predicate to determine whether a character is considered trivia.
    /// Trivia is implicitly skipped after pattern-matching constructs.
    ///
    /// For the common cases of Unicode and ASCII whitespace respectively,
    /// you may use [`SkippingStr::whitespace`] and [`SkippingStr::ascii_whitespace`].
    ///
    /// # Caller Precondition
    /// - The predicate function provided must be deterministic and stateless.
    ///
    /// Failure to abide will result in non-deterministic parsing, particularly in combination with speculative parsing branches.
    pub fn new<'root>(
        source: &'root str,
        kind: impl Into<SyntaxKind>,
        predicate: fn(char) -> bool,
    ) -> Self
    where
        'root: 'source,
    {
        // This is only used on initialization and is conceptually similar to `self.next_non_trivia_character`,
        // so here it is without pulling it into a separate function.
        let start_cursor = {
            // Final value for if the subsequent for-loop never breaks (all-trivia input).
            let mut cursor = StrIndex(source.len());
            for (index, ch) in source.char_indices() {
                if !(predicate)(ch) {
                    // return first non-trivia character.
                    cursor = StrIndex(index);
                    break;
                }
            }
            cursor
        };

        SkippingStr {
            cached: CachedStr::new(source),
            skipped_kind: kind.into(),
            predicate,
            start_cursor,
        }
    }

    /// View the underlying source string of `self`.
    pub const fn source(&self) -> &str {
        self.cached.source()
    }

    /// Given a compatible [`Pattern`] implementor, match to this input while discarding leading and trailing trivia.
    ///
    /// Intended for use by [`Pattern<SkippedStr<'_>>`] implementors. Direct use is not recommended.
    pub fn match_with_skipping<'src, P>(
        &'src self,
        pattern: &P,
        mut cursor: StrIndex,
    ) -> PatternMatchResult<'src, P, str>
    where
        Self: Input<Cursor = StrIndex, Token<'src> = char>,
        P: Pattern<'src, str>,
    {
        // Leading trivia.
        cursor = self.next_non_trivia_index(cursor);

        // Match.
        let result = pattern.matches_at(self.source(), cursor);

        match result {
            Ok(mut matched) => {
                // Trailing trivia.
                matched.next_cursor = self.next_non_trivia_index(matched.next_cursor);
                Ok(matched)
            }
            Err(e) => Err(e),
        }
    }

    /// Finds the next non-trivia index at or after the current index.
    fn next_non_trivia_index(&self, curr: StrIndex) -> StrIndex {
        let suboffset = curr.into_usize();
        let chars = self.source()[suboffset..].char_indices();
        for (char_offset, char) in chars {
            if !(self.predicate)(char) {
                return StrIndex(suboffset + char_offset);
            }
        }
        StrIndex(self.source().len())
    }
}

impl<'src> From<SkippingStr<'src>> for &'src str {
    fn from(value: SkippingStr<'src>) -> Self {
        value.cached.into()
    }
}

/// As an [`Input`] implementor, [`SkippingStr`] aggressively strips leading and trailing whitespace.
impl<'source> Input for SkippingStr<'source> {
    type Cursor = StrIndex;
    type Token<'src>
        = char
    where
        Self: 'src;

    /// O(1) with caching.
    /// This is done because [`PatternMatch::actualize_leaves`](crate::input::PatternMatch::actualize_leaves)
    /// calls this to check for leading trivia prior to the start cursor.
    fn start(&self) -> Self::Cursor {
        self.start_cursor
    }
    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token<'_>, Self::Cursor>> {
        self.source().peek(at).map(|mut inner| {
            inner.next_cursor = self.next_non_trivia_index(inner.next_cursor);
            inner
        })
    }

    fn skipped_trivia_kind(&self) -> Option<SyntaxKind> {
        Some(self.skipped_kind)
    }

    fn byte_offset(&self, at: Self::Cursor) -> usize {
        self.source().byte_offset(at)
    }
    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str> {
        self.source().slice_bytes(start, end)
    }
    fn byte_to_line_col(&self, at_offset: usize) -> LineColumn {
        self.cached.to_line_col(at_offset)
    }

    fn describe_token<'src>(&self, token: &Self::Token<'src>) -> String
    where
        Self: 'src,
    {
        self.source().describe_token(token)
    }
}

impl<'src, 'source, K> Pattern<'src, SkippingStr<'source>> for (K, char)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        SkippingStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src SkippingStr<'source>,
        cursor: <SkippingStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, SkippingStr<'source>> {
        input.match_with_skipping(self, cursor)
    }
    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

impl<'src, 'source, K> Pattern<'src, SkippingStr<'source>> for (K, &str)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        SkippingStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src SkippingStr<'source>,
        cursor: <SkippingStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, SkippingStr<'source>> {
        input.match_with_skipping(self, cursor)
    }
    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

impl<'src, 'source, K> Pattern<'src, SkippingStr<'source>> for (K, String)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        SkippingStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src SkippingStr<'source>,
        cursor: <SkippingStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, SkippingStr<'source>> {
        input.match_with_skipping(self, cursor)
    }
    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

#[cfg(feature = "regex")]
impl<'src, 'source, K> Pattern<'src, SkippingStr<'source>> for (K, regex::Regex)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        SkippingStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src SkippingStr<'source>,
        cursor: <SkippingStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, SkippingStr<'source>> {
        input.match_with_skipping(self, cursor)
    }
    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}

#[cfg(feature = "regex")]
impl<'src, 'source, K> Pattern<'src, SkippingStr<'source>> for (K, regex_lite::Regex)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
{
    type Payload
        = <Self as Pattern<'src, str>>::Payload
    where
        SkippingStr<'source>: 'src;

    fn matches_at(
        &self,
        input: &'src SkippingStr<'source>,
        cursor: <SkippingStr<'source> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, SkippingStr<'source>> {
        input.match_with_skipping(self, cursor)
    }
    fn describe(&self) -> String {
        <Self as Pattern<str>>::describe(self)
    }
    fn hint(&self) -> ParseHint {
        <Self as Pattern<str>>::hint(self)
    }
}
