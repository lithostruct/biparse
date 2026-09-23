use std::{borrow::Cow, cell::RefCell, fmt::Debug, ops::Range, rc::Rc};

use derive_more::Into;
use logos::{Lexer, Logos, SpannedIter};
use rowan::SyntaxKind;

use crate::{
    input::{CachedStr, Input, InputItem, LineColumn, Pattern, PatternMatch},
    parse::{Expectation, ParseHint},
};

/// Type alias for an iterator item returned by a [`logos::SpannedIter<'src, T>`].
pub type LexerItem<'src, Token> = (Result<Token, <Token as Logos<'src>>::Error>, Range<usize>);

/// The index newtype for a lexer
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
    pub fn new(source: &'src Token::Source, skipped_kind: impl Into<SyntaxKind>) -> Self
    where
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
    /// # Caller Invariant
    ///
    /// This constructor assumes the caller has asserted that
    /// the Lexer created from this `Token` type produces gapless input coverage.
    /// Biparse may panic during parsing if this invariant is not upheld.
    pub fn assume_contiguous(source: &'src Token::Source) -> Self
    where
        Token: Clone,
        Token::Extras: Default,
    {
        LexerStream {
            inner: Rc::new(RefCell::new(LexerStreamInner::new(source))),
            skipped_kind: None,
            source: CachedStr::new(source),
        }
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
    pub fn new(source: &'src Token::Source) -> Self
    where
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

impl<'src, Token> Input for LexerStream<'src, Token>
where
    Token: Debug + Clone,
    Token: Logos<'src, Source = str>,
{
    type Cursor = LexerIndex;
    type Token = Result<Token, <Token as Logos<'src>>::Error>;

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

impl<'src, T> Pattern<LexerStream<'src, T>> for (SyntaxKind, T)
where
    T: Debug + Clone + Logos<'src, Source = str>,
{
    type Payload<'p>
        = T
    where
        LexerStream<'src, T>: 'p;

    fn matches_at<'p>(
        &self,
        input: &'p LexerStream<'src, T>,
        cursor: <LexerStream<'src, T> as Input>::Cursor,
    ) -> Result<
        PatternMatch<'p, Self::Payload<'p>, <LexerStream<'src, T> as Input>::Cursor>,
        Option<<LexerStream<'src, T> as Input>::Cursor>,
    >
    where
        LexerStream<'src, T>: 'p,
    {
        match input.peek(cursor) {
            Some(item) => item.token.map_or_else(
                |_error| Err(None),
                |token| {
                    Ok(PatternMatch::new(
                        token,
                        vec![(self.0, item.span)],
                        item.next_cursor,
                    ))
                },
            ),
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

impl<'src, T> Pattern<LexerStream<'src, T>> for T
where
    T: Debug + Logos<'src, Source = str> + rowan::Language<Kind = T>,
{
    type Payload<'p>
        = T
    where
        LexerStream<'src, T>: 'p;

    fn matches_at<'p>(
        &self,
        input: &'p LexerStream<'src, T>,
        cursor: <LexerStream<'src, T> as Input>::Cursor,
    ) -> Result<
        PatternMatch<'p, Self::Payload<'p>, <LexerStream<'src, T> as Input>::Cursor>,
        Option<<LexerStream<'src, T> as Input>::Cursor>,
    >
    where
        LexerStream<'src, T>: 'p,
    {
        match input.peek(cursor) {
            Some(item) => item.token.map_or_else(
                |_error| Err(None),
                |token| {
                    Ok(PatternMatch::new(
                        token,
                        vec![(Self::kind_to_raw(*self), item.span)],
                        item.next_cursor,
                    ))
                },
            ),
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
