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
#[derive(Debug)]
pub struct SkippingStr<'source> {
    cached: CachedStr<'source>,
    policy: fn(char) -> bool,
    skipped_kind: SyntaxKind,
    start_cursor: StrIndex,
}
impl<'source> SkippingStr<'source> {
    /// Constructs an input that treats Unicode whitespace as trivia, identified via [`char::is_whitespace`].
    /// Trivia is implicitly skipped after pattern-matching constructs.
    ///
    /// For ASCII-based whitespace, see [`TriviaStr::ascii_whitespace`].
    pub fn whitespace<'root>(source: &'root str, kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'source,
    {
        Self::new(source, kind, char::is_whitespace)
    }
    /// Constructs an input that treats ASCII whitespace as trivia, identified via [`char::is_ascii_whitespace`].
    /// Trivia is implicitly skipped after pattern-matching constructs.
    ///
    /// For ASCII-based whitespace, see [`TriviaStr::whitespace`].
    pub fn ascii_whitespace<'root>(source: &'root str, kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'source,
    {
        // Why are the ASCII-based character methods `fn(&char)` instead of `fn(char)`?
        Self::new(source, kind, |ch| ch.is_ascii_whitespace())
    }

    /// Constructs an input that uses the given policy to determine whether a character is considered trivia.
    /// Trivia is implicitly skipped after pattern-matching constructs.
    ///
    /// For the common cases of Unicode and ASCII whitespace respectively,
    /// you may use [`TriviaStr::whitespace`] and [`TriviaStr::ascii_whitespace`].
    pub fn new<'root>(
        source: &'root str,
        kind: impl Into<SyntaxKind>,
        policy: fn(char) -> bool,
    ) -> Self
    where
        'root: 'source,
    {
        let start_cursor = SkippingStr::starts_at(source, policy);
        SkippingStr {
            cached: CachedStr::new(source),
            skipped_kind: kind.into(),
            policy,
            start_cursor,
        }
    }

    /// View the underlying source string of `self`.
    pub const fn source(&self) -> &str {
        self.cached.source()
    }
    fn starts_at(source: &str, policy: fn(char) -> bool) -> StrIndex {
        for (index, ch) in source.char_indices() {
            if !(policy)(ch) {
                // return first non-trivia character.
                return StrIndex(index);
            }
        }
        // end of input. the entire input is *just* trivia.
        StrIndex(source.len())
    }

    /// Given a compatible [`Pattern`] implementor, match to this input while discarding leading and trailing trivia.
    ///
    /// Intended for use by [`Pattern<SkippedStr<'_>>`] implementors.
    pub fn match_with_skipping<P>(
        &self,
        pattern: &P,
        mut cursor: StrIndex,
    ) -> PatternMatchResult<'_, P, str>
    where
        Self: Input<Cursor = StrIndex, Token = char>,
        P: Pattern<str>,
    {
        // Leading trivia.
        while let Some(inner) = self.peek(cursor) {
            if (self.policy)(inner.token) {
                cursor = inner.next_cursor;
            } else {
                break;
            }
        }

        // Match.
        let result = pattern.matches_at(self.source(), cursor);

        match result {
            Ok(mut matched) => {
                // Trailing trivia.
                while let Some(inner) = self.peek(matched.next_cursor) {
                    if (self.policy)(inner.token) {
                        matched.next_cursor = inner.next_cursor;
                    } else {
                        break;
                    }
                }
                Ok(matched)
            }
            Err(e) => Err(e),
        }
    }
}

impl<'src> From<SkippingStr<'src>> for &'src str {
    fn from(value: SkippingStr<'src>) -> Self {
        value.cached.into()
    }
}

impl<'source> Input for SkippingStr<'source> {
    type Cursor = StrIndex;
    type Token = char;

    /// O(1) with caching. This is done because [`PatternMatch::actualize_leaves`](crate::input::PatternMatch::actualize_leaves)
    /// calls this to check for leading trivia prior to the start cursor.
    fn start(&self) -> Self::Cursor {
        self.start_cursor
    }

    fn skipped_trivia_kind(&self) -> Option<SyntaxKind> {
        Some(self.skipped_kind)
    }

    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token, Self::Cursor>> {
        self.source().peek(at)
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

    fn describe_token(token: &Self::Token) -> String {
        <str as Input>::describe_token(token)
    }
}

impl<'source, K> Pattern<SkippingStr<'source>> for (K, char)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        SkippingStr<'source>: 'src;

    fn matches_at<'src>(
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

impl<'source, K> Pattern<SkippingStr<'source>> for (K, &str)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        SkippingStr<'source>: 'src;

    fn matches_at<'src>(
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

impl<'source, K> Pattern<SkippingStr<'source>> for (K, String)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        SkippingStr<'source>: 'src;

    fn matches_at<'src>(
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
impl<'source, K> Pattern<SkippingStr<'source>> for (K, regex::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        SkippingStr<'source>: 'src;

    fn matches_at<'src>(
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
impl<'source, K> Pattern<SkippingStr<'source>> for (K, regex_lite::Regex)
where
    K: Clone + Into<SyntaxKind>,
{
    type Payload<'src>
        = <Self as Pattern<str>>::Payload<'src>
    where
        SkippingStr<'source>: 'src;

    fn matches_at<'src>(
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
