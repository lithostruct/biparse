use crate::{
    cst::{CstOutput, IdSupply},
    green::GreenInterner,
    input::Input,
};
use std::{fmt::Debug, hash::Hash};

pub struct ParseState<I: Input, S: IdSupply> {
    pub cursor: I::Cursor,
    pub interner: GreenInterner,
    pub supply: S,
}
impl<I: Input, S: IdSupply + Default> ParseState<I, S> {
    pub fn new(input: &I) -> Self {
        ParseState {
            cursor: input.start(),
            interner: GreenInterner::new(),
            supply: S::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput<A, F, Id>
where
    Id: Debug + Clone + Eq + Hash,
{
    pub ast: ParseResult<A, F>,
    pub cst: CstOutput<Id>,
}
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
