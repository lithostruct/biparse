use rowan::{GreenNode, GreenToken};

use crate::{green_interner::GreenInterner, input::Input};


pub struct ParserState<I : Input> {
    cursor: I::Cursor,
    interner: GreenInterner,
}
pub struct ParserOutput<A, F, Id, C> {
    ast: ParserResult<A, F>,
    cst: C,
    origin: im::HashMap<Id, C>,
    errors: im::HashMap<Id, F>,
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