//! Module relating to parser combinators.
#![allow(non_upper_case_globals)]

mod blanket;

mod binary;
mod just;
mod map_to_owned;
mod node_association;
mod recursive;
mod sentinel;

use crate::{
    input::{Input, Pattern},
    parse::{ParseEnvironment, Parser},
};

pub use self::{
    binary::{Either, Or, Then},
    blanket::{AsParser, WithEnv},
    just::Just,
    map_to_owned::{Map, ToOwned},
    node_association::{Associate, Node},
    recursive::{
        combinator::{Direct, Indirect, Recursive},
        frame::RecursiveState,
    },
    sentinel::{Success, End, Fail},
};

// Constants and functions.

pub fn success<'src, I: Input + ?Sized, E: ParseEnvironment>() -> AsParser<'src, I, (), E, Success> {
    AsParser::from(Success)
}
pub fn fail<'src, I: Input + ?Sized, E: ParseEnvironment>() -> AsParser<'src, I, (), E, Fail> {
    AsParser::from(Fail)
}
pub fn end<'src, I: Input + ?Sized, E: ParseEnvironment>() -> AsParser<'src, I, (), E, End> {
    AsParser::from(End)
}

pub fn just<'src, I: Input + ?Sized, E: ParseEnvironment, P: Pattern<I> + 'src>(
    kind: rowan::SyntaxKind,
    pattern: P,
) -> AsParser<'src, I, P::Payload<'src>, E, Just<P>> {
    AsParser::from(Just::new(kind, pattern))
}

pub fn recursive<'src, I: Input + ?Sized, A, E: ParseEnvironment, F, P>(
    closure: F,
) -> Recursive<Direct<'src, I, A, E>>
where
    F: FnOnce(Recursive<Direct<'src, I, A, E>>) -> P,
    P: Parser<'src, I, A, E> + 'src,
{
    Recursive::recursive(closure)
}
