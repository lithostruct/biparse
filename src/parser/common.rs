use rowan::{GreenNode, GreenToken};

use crate::{
    green::green_interner::{GreenElement, GreenInterner},
    input::Input,
    parser::cst_cardinality::{CstOutput, Zero},
};
use std::hash::Hash;

pub struct ParserState<I: Input> {
    pub cursor: I::Cursor,
    pub(crate) interner: GreenInterner,
}
impl<I: Input> ParserState<I> {
    pub fn new(input: &I) -> Self {
        ParserState {
            cursor: input.start(),
            interner: GreenInterner::new(),
        }
    }
}

pub struct ParserOutput<A, F, Id, C>
where
    Id: Eq + Hash,
    C: CstOutput,
{
    pub ast: ParserResult<A, F>,
    pub cst: C,
    pub origin: Association<Id, C>,
}
pub enum ParserResult<A, F> {
    Success(A),
    Speculative(F),
    Propagating(F),
}

/// Struct controlling the association between an Id type and a C containing GreenNodes.
#[derive(Debug, Clone)]
pub struct Association<Id, C>
where
    Id: Eq + Hash,
    C: CstOutput,
{
    pub(crate) inner: im::HashMap<Id, <C as CstOutput>::PathType>,
}
impl<Id, C> Default for Association<Id, C>
where
    Id: Eq + Hash,
    C: CstOutput,
{
    fn default() -> Self {
        Association {
            inner: im::HashMap::new(),
        }
    }
}
impl<Id, C> Association<Id, C>
where
    Id: Eq + Hash,
    C: CstOutput,
{
    pub fn new() -> Self {
        Association::default()
    }
}

pub trait Parser<A, F, Id, C>
where
    Id: Eq + Hash,
    C: CstOutput,
{
    fn parse<I: Input>(
        &self,
        input: &I,
        state: ParserState<I>,
    ) -> (ParserState<I>, ParserOutput<A, F, Id, C>);
    fn check<I: Input>(
        &self,
        input: &I,
        state: ParserState<I>,
    ) -> (ParserState<I>, ParserOutput<(), F, Id, Zero<GreenElement>>);
}
