use rowan::{GreenNode, GreenToken, SyntaxKind};

use crate::{
    cst::{CstOutput, IdSupply},
    green::{GreenElement, GreenInterner},
    input::Input,
    parse::ParseEnvironment,
};
use std::{cell::RefCell, fmt::Debug, hash::Hash, rc::Rc};

/// The parse state to be passed and returned by all parser combinators.
pub struct ParseState<I: Input, S: IdSupply> {
    /// The current cursor position.
    pub cursor: I::Cursor,
    /// A supply of type `S`.
    pub supply: S,
    /// A reference-counted shared interner, allowing for clones to access the same structure.
    ///
    /// # Safety
    ///
    /// This field comes with the usual pitfalls of [`Rc<RefCell<...>>`];
    /// if you hold a mutable borrow and attempt subsequent borrowing, Rust *will* panic.
    ///
    /// Direct usage is discouraged. The interning methods of [`GreenInterner`]
    /// are re-exposed here as [`ParseState::intern_node`] and [`ParseState::intern_token`],
    /// which holds the mutable borrow for as short as possible.
    pub interner: Rc<RefCell<GreenInterner>>,
}
impl<I: Input, S: IdSupply> Debug for ParseState<I, S>
where
    I::Cursor: Debug,
    S: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ParseState");

        debug.field("cursor", &self.cursor);

        // This is a whole thing because we cannot get an immutable view
        // if the interner is currently mutably-borrowed (results in panic naively).
        // This shouldn't be a problem via API usage of this type,
        // but the field is public (for end-users to be able to reconstruct ParseState)
        // so anything goes.
        // I would rather `Debug` not panic in this scenario.
        match self.interner.try_borrow() {
            Ok(borrowed) => debug.field("interner", &borrowed as &GreenInterner),
            Err(_) => debug.field("interner", &"<borrowed>"),
        };

        debug.field("supply", &self.supply).finish()
    }
}
impl<I: Input, S: IdSupply> Clone for ParseState<I, S> {
    fn clone(&self) -> Self {
        ParseState {
            cursor: self.cursor,
            interner: self.interner.clone(),
            supply: self.supply.clone(),
        }
    }
}
impl<I: Input, S: IdSupply + Default> ParseState<I, S> {
    /// Creates a new parse state from this input.
    pub fn new(input: &I) -> Self {
        ParseState {
            cursor: input.start(),
            interner: Rc::new(RefCell::new(GreenInterner::new())),
            supply: S::default(),
        }
    }
    pub fn intern_token(&self, kind: SyntaxKind, text: &str) -> GreenToken {
        let mut interner = self.interner.borrow_mut();
        interner.token(kind, text)
    }
    pub fn intern_node(
        &self,
        kind: SyntaxKind,
        collection: impl IntoIterator<Item = GreenElement>,
    ) -> GreenNode {
        let mut interner = self.interner.borrow_mut();
        interner.node(kind, collection)
    }
}

/// Parse output produced by [`Parser`](crate::parse::Parser) implementors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput<A, E: ParseEnvironment> {
    pub ast: ParseResult<A, E::Failure>,
    pub cst: CstOutput<<E::Supply as IdSupply>::Id>,
}

/// Status of parse result of the AST payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ParseResult<A, F> {
    Success(A),
    Speculative(F),
    Committed(F),
}
impl<A, F> ParseResult<A, F> {
    pub fn into_result(self) -> Result<A, F> {
        match self {
            ParseResult::Success(success) => Ok(success),
            ParseResult::Speculative(failure) | ParseResult::Committed(failure) => Err(failure),
        }
    }
    pub fn map<B>(self, f: impl FnOnce(A) -> B) -> ParseResult<B, F> {
        match self {
            ParseResult::Success(inner) => ParseResult::Success(f(inner)),
            ParseResult::Speculative(failure) => ParseResult::Speculative(failure),
            ParseResult::Committed(failure) => ParseResult::Committed(failure),
        }
    }
    pub fn map_err<F2>(self, f: impl FnOnce(F) -> F2) -> ParseResult<A, F2> {
        match self {
            ParseResult::Success(inner) => ParseResult::Success(inner),
            ParseResult::Speculative(failure) => ParseResult::Speculative(f(failure)),
            ParseResult::Committed(failure) => ParseResult::Committed(f(failure)),
        }
    }
}
