use std::{borrow::Cow, cell::RefCell, fmt::Debug, ops::Range, rc::Rc};

use derive_more::Into;
use logos::{Lexer, Logos, SpannedIter};
use rowan::SyntaxKind;

use crate::input::{Input, InputItem, LineCol};

/// Type alias for an iterator item returned by a [`logos::SpannedIter<'src, T>`].
pub type LexerItem<'src, Token> = (Result<Token, <Token as Logos<'src>>::Error>, Range<usize>);

/// The index newtype for a lexer
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Into)]
pub struct LexerIndex(#[into] usize);

/// A caching structure for [`logos::Lexer`], which supports arbitrary indexing.
pub struct LexerStream<'src, Token: Logos<'src>> {
    inner: Rc<RefCell<LexerStreamInner<'src, Token>>>,
    source: &'src str,
    skipped_kind: Option<SyntaxKind>,
}
impl<'src, Token: Logos<'src>> Clone for LexerStream<'src, Token> {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
            source: self.source,
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
            source,
        }
    }

    /// Creates a new `LexerStream` that assumes the lexer produces contiguous output.
    ///
    /// # Invariants
    ///
    /// This constructor assumes the caller has asserted that
    /// the Lexer created from this `Token` type produces gapless input coverage.
    /// Biparse may panic during parsing if this invariant is violated.
    pub fn assume_contiguous(source: &'src Token::Source) -> Self
    where
        Token: Clone,
        Token::Extras: Default,
    {
        LexerStream {
            inner: Rc::new(RefCell::new(LexerStreamInner::new(source))),
            skipped_kind: None,
            source,
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
        if self.cache.len() < index.0 {
            return Some(&self.cache[index.0]);
        }

        // Request items from the lexer to get a new
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
    Token: Debug + Clone + Eq,
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
            None => self.inner.borrow().lexer.source().len(),
        }
    }

    fn slice_bytes(&self, start: usize, end: usize) -> Cow<'_, str> {
        Cow::Borrowed(&self.source[start..end])
    }

    /// Naive formulation for now. O(N) time.
    ///
    /// This structure would be a good candidate for embedding line-start caching though.
    fn byte_to_line_col(&self, at_offset: usize) -> LineCol {
        let source = self.inner.borrow().lexer.source();
        let mut line = 1;
        let mut column = 1;
        for (offset, ch) in source.char_indices() {
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
        LineCol { line, col: column }
    }

    fn describe_token(token: &Self::Token) -> String {
        format!("token '{token:?}'")
    }

    fn skipped_trivia_kind(&self) -> Option<rowan::SyntaxKind> {
        self.skipped_kind
    }
}
