use std::{borrow::Cow, cell::RefCell, fmt::Debug, ops::Range, rc::Rc};

use derive_more::{AsMut, AsRef, Deref, DerefMut, From, Into};
use logos::{Lexer, Logos, SpannedIter};
use rowan::SyntaxKind;

use crate::{
    input::{
        AllOf, AnyOf, Input, InputItem, LineColumn, Pattern, PatternMatch, PatternMatchResult,
        cached_str::CachedStr,
    },
    parse::{Expectation, ParseHint},
};

/// Type alias for an iterator item returned by a [`logos::SpannedIter<'src, T>`].
pub type LexerItem<'src, Token> = (Result<Token, <Token as Logos<'src>>::Error>, Range<usize>);

/// The index newtype for a lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct LexerIndex(#[into] usize);

/// A caching structure for [`logos::Lexer`], which supports arbitrary indexing.
pub struct LexerStream<'src, Token: Logos<'src>> {
    inner: Rc<RefCell<LexerStreamInner<'src, Token>>>,
    source: CachedStr<'src>,
    skipped_kind: Option<SyntaxKind>,
}
impl<'src, Token: Logos<'src>> Clone for LexerStream<'src, Token> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            source: self.source.clone(),
            skipped_kind: self.skipped_kind,
        }
    }
}
impl<'src, Token> LexerStream<'src, Token>
where
    Token: Logos<'src, Source = str>,
{
    /// Creates a new `LexerStream` with a specified [`rowan::SyntaxKind`] for skipped source spans.
    pub fn new<'root>(source: &'root Token::Source, skipped_kind: impl Into<SyntaxKind>) -> Self
    where
        'root: 'src,
        Token: Clone,
        Token::Extras: Default,
    {
        LexerStream {
            inner: Rc::new(RefCell::new(LexerStreamInner::new(source))),
            skipped_kind: Some(skipped_kind.into()),
            source: CachedStr::new(source),
        }
    }

    /// Creates a new `LexerStream` that assumes the lexer produces contiguous output.
    ///
    /// # Caller Precondition
    ///
    /// This constructor assumes the caller has asserted that
    /// the Lexer created from this `Token` type produces gapless input coverage.
    /// Biparse may panic during parsing if this invariant is not upheld.
    pub fn assume_contiguous<'root>(source: &'root Token::Source) -> Self
    where
        'root: 'src,
        Token: Clone,
        Token::Extras: Default,
    {
        LexerStream {
            inner: Rc::new(RefCell::new(LexerStreamInner::new(source))),
            skipped_kind: None,
            source: CachedStr::new(source),
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

struct LexerStreamInner<'src, Token: Logos<'src>> {
    lexer: SpannedIter<'src, Token>,
    cache: im::Vector<LexerItem<'src, Token>>,
}
impl<'src, Token: Logos<'src>> Clone for LexerStreamInner<'src, Token>
where
    Token: Clone,
    Token::Extras: Clone,
{
    fn clone(&self) -> Self {
        Self {
            lexer: self.lexer.clone(),
            cache: self.cache.clone(),
        }
    }
}
impl<'src, Token: Logos<'src>> LexerStreamInner<'src, Token> {
    pub fn new<'root>(source: &'root Token::Source) -> Self
    where
        'root: 'src,
        Token: Clone,
        Token::Extras: Default,
    {
        LexerStreamInner {
            lexer: Lexer::new(source).spanned(),
            cache: im::Vector::new(),
        }
    }

    pub fn get(&mut self, index: LexerIndex) -> Option<&LexerItem<'src, Token>>
    where
        Token: Clone,
    {
        // Already exists in cache.
        if index.0 < self.cache.len() {
            return Some(&self.cache[index.0]);
        }

        // Request items from the lexer.
        // This should never panic due to the guard above; requested_len > 0.
        let requested_len = index.0 - self.cache.len() + 1;
        let requested = self
            .lexer
            .by_ref()
            .take(requested_len)
            .collect::<im::Vector<_>>();
        self.cache.append(requested);

        // This handles out-of-bounds or lexer exhaustion btw.
        self.cache.get(index.0)
    }
}

impl<'source, Token> Input for LexerStream<'source, Token>
where
    Token: Debug + Clone + 'static + Logos<'source, Source = str>,
{
    type Cursor = LexerIndex;
    type Token = Result<Token, <Token as Logos<'source>>::Error>;

    fn start(&self) -> Self::Cursor {
        LexerIndex(0)
    }

    fn peek(&self, at: Self::Cursor) -> Option<InputItem<Self::Token, Self::Cursor>> {
        self.inner
            .borrow_mut()
            .get(at)
            .cloned()
            .map(|inner| InputItem {
                token: inner.0,
                span: inner.1,
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

    fn describe_token(token: &Self::Token) -> String {
        format!("token '{token:?}'")
    }

    fn skipped_trivia_kind(&self) -> Option<rowan::SyntaxKind> {
        self.skipped_kind
    }
}

impl<'source, K, T> Pattern<LexerStream<'source, T>> for (K, T)
where
    K: Clone + Into<SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'static,
{
    type Payload<'src>
        = T
    where
        LexerStream<'source, T>: Input + 'src;

    fn matches_at<'src>(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        match input.peek(cursor) {
            Some(item) => {
                if item.token.as_ref().is_ok_and(|token| *token == self.1) {
                    Ok(PatternMatch::new(
                        item.token.expect("item.token is not Err!"),
                        vec![(self.0.clone().into(), item.span)],
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
        format!("token {:?}", self.1)
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Expectation::Pattern {
            kind: "token".into(),
            kind_plural: "tokens".into(),
            data: format!("{:?}", self.1),
        })
    }
}

impl<'source, T> Pattern<LexerStream<'source, T>> for T
where
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<SyntaxKind> + 'static,
{
    type Payload<'src>
        = T
    where
        LexerStream<'source, T>: Input + 'src;

    fn matches_at<'src>(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        match input.peek(cursor) {
            Some(item) => {
                if item.token.as_ref().is_ok_and(|token| token == self) {
                    Ok(PatternMatch::new(
                        item.token.expect("item.token is not Err!"),
                        vec![(self.clone().into(), item.span)],
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
        format!("token {self:?}")
    }
    fn hint(&self) -> ParseHint {
        ParseHint::Expectation(Expectation::Pattern {
            kind: "token".into(),
            kind_plural: "tokens".into(),
            data: format!("{self:?}"),
        })
    }
}

/// In addition to `Logos` token matching, attach the text associated with the match.
///
/// Supported types include:
/// - (K, T) where K : Into<SyntaxKind>, T : Pattern<LexerStream<...>>
/// - T where T : Pattern<LexerStream<...>> + Into<SyntaxKind>
/// - AllOf<P> where P : Pattern<...>
/// - AnyOf<P> where P : Pattern<...>
///
/// Notably, `str`-based patterns are *not* supported, as they return text as their payloads.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, From, Deref, DerefMut, AsRef, AsMut,
)]
pub struct WithText<P>(#[from] pub P);

impl<'source, K, T> Pattern<LexerStream<'source, T>> for WithText<(K, T)>
where
    K: Clone + Into<SyntaxKind>,
    T: Debug + Clone + Eq + Logos<'source, Source = str> + 'static,
{
    type Payload<'src>
        = (T, Cow<'src, str>)
    where
        LexerStream<'source, T>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        self.0.matches_at(input, cursor).map(|matched| {
            // INVARIANT: `matched.cst_spans` is a singleton vector.
            let range = &matched.cst_spans.first().unwrap().1;
            PatternMatch::new(
                (matched.payload, input.slice_bytes(range.start, range.end)),
                matched.cst_spans,
                matched.next_cursor,
            )
        })
    }

    fn describe(&self) -> String {
        self.0.describe()
    }
    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}

impl<'source, T> Pattern<LexerStream<'source, T>> for WithText<T>
where
    T: Debug + Clone + Eq + Logos<'source, Source = str> + Into<SyntaxKind> + 'static,
{
    type Payload<'src>
        = (T, Cow<'src, str>)
    where
        LexerStream<'source, T>: 'src;

    fn matches_at<'src>(
        &self,
        input: &'src LexerStream<'source, T>,
        cursor: <LexerStream<'source, T> as Input>::Cursor,
    ) -> PatternMatchResult<'src, Self, LexerStream<'source, T>> {
        self.0.matches_at(input, cursor).map(|matched| {
            // INVARIANT: `matched.cst_spans` is a singleton vector.
            let range = &matched.cst_spans.first().unwrap().1;
            PatternMatch::new(
                (matched.payload, input.slice_bytes(range.start, range.end)),
                matched.cst_spans,
                matched.next_cursor,
            )
        })
    }

    fn describe(&self) -> String {
        self.0.describe()
    }
    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}

impl<I: Input + ?Sized, P> Pattern<I> for WithText<AllOf<P>>
where
    P: Pattern<I>,
{
    type Payload<'src>
        = (P::Payload<'src>, Vec<Cow<'src, str>>)
    where
        I: 'src;

    fn matches_at<'src>(
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
            PatternMatch::new(
                (matched.payload, texts),
                matched.cst_spans,
                matched.next_cursor,
            )
        })
    }

    fn describe(&self) -> String {
        self.0.describe()
    }
    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}

impl<I: Input + ?Sized, P> Pattern<I> for WithText<AnyOf<P>>
where
    P: Pattern<I>,
{
    type Payload<'src>
        = (P::Payload<'src>, Vec<Cow<'src, str>>)
    where
        I: 'src;

    fn matches_at<'src>(
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
            PatternMatch::new(
                (matched.payload, texts),
                matched.cst_spans,
                matched.next_cursor,
            )
        })
    }

    fn describe(&self) -> String {
        self.0.describe()
    }
    fn hint(&self) -> ParseHint {
        self.0.hint()
    }
}
