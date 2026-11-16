//! Module for the implementation of [`Input`] on arbitrary [`Logos`] grammars.
//! 
//! Requires default feature [`logos`].

use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    ops::Range,
};

use derive_more::{AsMut, AsRef, Deref, DerefMut, From, Into};
use logos::{Lexer, Logos};
use rowan::SyntaxKind;

use crate::{
    failure::{Descriptor, ParseHint},
    input::{
        AllOf, AnyOf, Input, InputItem, LineColumn, Pattern, PatternMatch, PatternMatchResult,
        cached_str::CachedStr,
    },
};

/// Type alias for an iterator item returned by a [`logos::SpannedIter<'src, T>`].
pub type LexerItem<'src, Token> = (Result<Token, <Token as Logos<'src>>::Error>, Range<usize>);

/// The index newtype for a lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct LexerIndex(#[into] usize);

/// An input structure for [`logos::Lexer`] supporting arbitrary indexing.
/// The lexer stream is eagerly materialized from the specified `Token : Logos<'src>` grammar.
pub struct LexerStream<'src, Token: Logos<'src>> {
    inner: im::Vector<LexerItem<'src, Token>>,
    source: CachedStr<'src>,
    skipped_kind: Option<SyntaxKind>,
    descriptor: fn(&Token) -> String,
}
impl<'src, Token: Logos<'src>> Clone for LexerStream<'src, Token>
where
    Token: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            source: self.source.clone(),
            skipped_kind: self.skipped_kind,
            descriptor: self.descriptor,
        }
    }
}
impl<'src, Token> LexerStream<'src, Token>
where
    Token: Clone + Logos<'src, Source = str>,
    Token::Extras: Default,
{
    /// Creates a new `LexerStream` with a specified [`rowan::SyntaxKind`] for skipped source spans.
    ///
    /// This constructor method requires the token type to implement [`Display`],
    /// to turn token references into string descriptors during diagnostic generation.
    ///
    /// Use [`LexerStream::new_via_debug`] to use the [`Debug`] trait instead,
    /// or [`LexerStream::new_with_descriptor`] for a user-supplied closure.
    pub fn new<'root>(source: &'root str, skipped_kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'src,
        Token: Display,
    {
        LexerStream::new_with_descriptor(source, skipped_kind, ToString::to_string)
    }
    /// Creates a new `LexerStream` with a specified [`rowan::SyntaxKind`] for skipped source spans.
    ///
    /// This constructor method requires the token type to implement [`Debug`],
    /// to turn token references into string descriptors during diagnostic generation.
    ///
    /// Use [`LexerStream::new`] to use the [`Display`] trait instead,
    /// or [`LexerStream::new_with_descriptor`] for a user-supplied closure.
    pub fn new_via_debug<'root>(source: &'root str, skipped_kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'src,
        Token: Debug,
    {
        LexerStream::new_with_descriptor(source, skipped_kind, |token: &Token| format!("{token:?}"))
    }
    /// Creates a new `LexerStream` with a specified [`rowan::SyntaxKind`] for skipped source spans.
    /// A closure is required to turn token references into string descriptors.
    ///
    /// You may use convenience methods [`LexerStream::new`] or [`LexerStream::new_via_debug`]
    /// to delegate to the token type's [`Display`] and [`Debug`] implementations respectively.
    pub fn new_with_descriptor<'root>(
        source: &'root str,
        skipped_kind: impl Into<SyntaxKind>,
        descriptor: fn(&Token) -> String,
    ) -> Self
    where
        'root: 'src,
    {
        LexerStream {
            inner: Lexer::new(source).spanned().collect(),
            skipped_kind: Some(skipped_kind.into()),
            source: CachedStr::new(source),
            descriptor,
        }
    }

    /// Creates a new `LexerStream` that assumes the lexer produces contiguous output.
    ///
    /// # Caller Precondition
    ///
    /// This constructor assumes the caller has asserted that
    /// the Lexer created from this `Token` type produces gapless input coverage.
    /// Biparse may panic during parsing if this invariant is not upheld.
    pub fn assume_contiguous<'root>(
        source: &'root Token::Source,
        descriptor: fn(&Token) -> String,
    ) -> Self
    where
        'root: 'src,
    {
        LexerStream {
            inner: Lexer::new(source).spanned().collect(),
            skipped_kind: None,
            source: CachedStr::new(source),
            descriptor,
        }
    }

    /// Returns the underlying source string.
    pub fn source(&self) -> &str {
        &self.source
    }
}

impl<'source, Token> Input for LexerStream<'source, Token>
where
    Token: Clone + Logos<'source, Source = str>,
{
    type Cursor = LexerIndex;
    type Token<'src>
        = Result<&'src Token, &'src <Token as Logos<'source>>::Error>
    where
        Self: 'src;

    fn start(&self) -> Self::Cursor {
        LexerIndex(0)
    }

    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token<'_>, Self::Cursor>> {
        self.inner.get(at.0).map(|inner| InputItem {
            token: inner.0.as_ref(),
            span: inner.1.clone(),
            next_cursor: LexerIndex(at.0 + 1),
        })
    }

    fn byte_offset(&self, at: Self::Cursor) -> usize {
        match self.peek(at) {
            Some(item) => item.span.start,
            // end of input.
            None => self.source.len(),
        }
    }

    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self.source[start..end])
    }

    fn byte_to_line_col(&self, at_offset: usize) -> LineColumn {
        self.source.byte_to_line_col(at_offset)
    }

    fn describe<'src>(&self, token: &Self::Token<'src>) -> Descriptor
    where
        Self: 'src,
    {
        match token.as_deref() {
            Ok(inner) => Descriptor::Pattern {
                kind: "token".into(),
                kind_plural: "tokens".into(),
                data: (self.descriptor)(inner),
            },
            Err(e) => Descriptor::Pattern {
                kind: "lex error".into(),
                kind_plural: "lex errors".into(),
                data: format!("{e:?}"),
            },
        }
    }

    fn skipped_trivia_kind(&self) -> Option<rowan::SyntaxKind> {
        self.skipped_kind
    }

    fn slice(&self, start: Self::Cursor, end: Self::Cursor) -> Cow<'_, str> {
        self.slice_bytes(self.byte_offset(start), self.byte_offset(end))
    }

    fn to_line_col(&self, at: Self::Cursor) -> LineColumn {
        self.byte_to_line_col(self.byte_offset(at))
    }
}

impl<'src, 'source, K, T> Pattern<'src, LexerStream<'source, T>> for (K, T)
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'src,
{
    type Payload
        = &'src T
    where
        LexerStream<'source, T>: Input + 'src;

    fn matches_at(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        match input.peek(cursor) {
            Some(item) => {
                if item.token.is_ok_and(|token| *token == self.1) {
                    Ok(PatternMatch {
                        payload: item.token.expect("item.token is not Err!"),
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
            kind: "token".into(),
            kind_plural: "tokens".into(),
            data: format!("{:?}", self.1),
        })
    }
}

impl<'src, 'source, T> Pattern<'src, LexerStream<'source, T>> for T
where
    'source: 'src,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<SyntaxKind> + 'src,
{
    type Payload
        = &'src T
    where
        LexerStream<'source, T>: Input + 'src;

    fn matches_at(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        match input.peek(cursor) {
            Some(item) => {
                if item.token.is_ok_and(|token| token == self) {
                    Ok(PatternMatch {
                        payload: item.token.expect("item.token is not Err!"),
                        cst_spans: vec![(self.clone().into(), item.span)],
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
            kind: "token".into(),
            kind_plural: "tokens".into(),
            data: format!("{self:?}"),
        })
    }
}

/// In addition to `Logos` token matching, attach the text associated with the match.
///
/// Supported types include:
/// - `(K, T) where K : Into<SyntaxKind>, T : Pattern<LexerStream<...>>`
/// - `T where T : Pattern<LexerStream<...>> + Into<SyntaxKind>`
/// - `AllOf<P> where P : Pattern<...>`
/// - `AnyOf<P> where P : Pattern<...>`
///
/// Notably, `str`-based patterns are *not* supported, as they return text as their payloads.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Deref, DerefMut, AsRef, AsMut,
)]
pub struct WithText<P>(#[from] pub P);

impl<'src, 'source, K, T> Pattern<'src, LexerStream<'source, T>> for WithText<(K, T)>
where
    'source: 'src,
    K: Clone + Into<SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'static,
{
    type Payload
        = (&'src T, Cow<'src, str>)
    where
        LexerStream<'source, T>: 'src;

    fn matches_at(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        self.0.matches_at(input, cursor).map(|matched| {
            // INVARIANT: `matched.cst_spans` is a singleton vector.
            let range = &matched.cst_spans.first().unwrap().1;
            PatternMatch {
                payload: (matched.payload, input.slice_bytes(range.start, range.end)),
                cst_spans: matched.cst_spans,
                next_cursor: matched.next_cursor,
            }
        })
    }

    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}

impl<'src, 'source, T> Pattern<'src, LexerStream<'source, T>> for WithText<T>
where
    'source: 'src,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<SyntaxKind> + 'static,
{
    type Payload
        = (&'src T, Cow<'src, str>)
    where
        LexerStream<'source, T>: 'src;

    fn matches_at(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        self.0.matches_at(input, cursor).map(|matched| {
            // INVARIANT: `matched.cst_spans` is a singleton vector.
            let range = &matched.cst_spans.first().unwrap().1;
            PatternMatch {
                payload: (matched.payload, input.slice_bytes(range.start, range.end)),
                cst_spans: matched.cst_spans,
                next_cursor: matched.next_cursor,
            }
        })
    }

    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}

impl<'src, I: Input + ?Sized, P> Pattern<'src, I> for WithText<AllOf<P>>
where
    I: 'src,
    P: Pattern<'src, I>,
{
    type Payload
        = (P::Payload, Vec<Cow<'src, str>>)
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        self.0.matches_at(input, cursor).map(|matched| {
            let texts: Vec<Cow<'src, str>> = matched
                .cst_spans
                .iter()
                .map(|elem| input.slice_bytes(elem.1.start, elem.1.end))
                .collect();
            PatternMatch {
                payload: (matched.payload, texts),
                cst_spans: matched.cst_spans,
                next_cursor: matched.next_cursor,
            }
        })
    }

    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}

impl<'src, I: Input + ?Sized, P> Pattern<'src, I> for WithText<AnyOf<P>>
where
    I: 'src,
    P: Pattern<'src, I>,
{
    type Payload
        = (P::Payload, Vec<Cow<'src, str>>)
    where
        I: 'src;

    fn matches_at(
        &self,
        input: &'src I,
        cursor: <I as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, I> {
        self.0.matches_at(input, cursor).map(|matched| {
            let texts: Vec<Cow<'src, str>> = matched
                .cst_spans
                .iter()
                .map(|elem| input.slice_bytes(elem.1.start, elem.1.end))
                .collect();
            PatternMatch {
                payload: (matched.payload, texts),
                cst_spans: matched.cst_spans,
                next_cursor: matched.next_cursor,
            }
        })
    }

    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}
