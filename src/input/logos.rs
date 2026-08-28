use std::{cell::RefCell, iter::Peekable, ops::Range, rc::Rc};

use logos::{Lexer, Logos, SpannedIter};

/// Type alias for an iterator item returned by a [`logos::SpannedIter<'src, T>`].
pub type LexerItem<'src, Token> = (Result<Token, <Token as Logos<'src>>::Error>, Range<usize>);

/// A caching stream structure for [`logos::Lexer`] supporting indexing.
pub struct LexerStream<'src, Token: Logos<'src>> {
    lexer: Peekable<SpannedIter<'src, Token>>,
    cache: Rc<RefCell<im::Vector<LexerItem<'src, Token>>>>,
}
impl<'src, Token: Logos<'src>> Clone for LexerStream<'src, Token>
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

pub struct LexerIndex(usize);
