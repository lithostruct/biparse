use rowan::{GreenNode, GreenToken};

use crate::{green_interner::GreenInterner, input::Input};


pub struct ParserState<I : Input> {
    pub cursor: I::Cursor,
    pub interner: GreenInterner,
}
pub struct ParserOutput<A, F, Id, C> {
    pub ast: ParserResult<A, F>,
    pub cst: C,
    pub origin: im::HashMap<Id, C>,
    pub errors: im::HashMap<Id, F>,
}
pub enum ParserResult<A, F> {
    Success(A),
    Speculative(F),
    Propagating(F),
}

pub trait Parser<A, F, Id, C> {
    fn parse<I : Input>(&self, input: &I, state: ParserState<I>) -> (ParserState<I>, ParserOutput<A, F, Id, C>);
    fn check<I : Input>(&self, input: &I, state: ParserState<I>) -> (ParserState<I>, ParserOutput<(), F, (), ()>);
}