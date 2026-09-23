//! Module relating to parser combinators.
#![allow(non_upper_case_globals)]

mod blanket;

mod binary;
mod just_satisfies;
mod map_into_owned;
mod node_association;
mod recursive;
mod repetition;
mod sentinel;

use crate::{
    input::{Input, InputItem, Pattern},
    parse::ParseEnvironment,
};

pub use self::{
    binary::{Or, Then},
    blanket::{AsParser, WithEnv},
    just_satisfies::{Just, Satisfies},
    map_into_owned::{IntoOwned, Map},
    node_association::{Associate, Node},
    recursive::{
        combinator::{Direct, Indirect, Recursive},
        frame::RecursiveState,
    },
    repetition::Many,
    sentinel::{End, Fail, Success},
};

// Constants and functions.

pub fn success<'src, I: Input + ?Sized, E: ParseEnvironment>() -> AsParser<'src, I, (), E, Success>
{
    AsParser::from(Success)
}
pub fn fail<'src, I: Input + ?Sized, E: ParseEnvironment>() -> AsParser<'src, I, (), E, Fail> {
    AsParser::from(Fail)
}
pub fn end<'src, I: Input + ?Sized, E: ParseEnvironment>() -> AsParser<'src, I, (), E, End> {
    AsParser::from(End)
}

pub fn just<'src, I: Input + ?Sized, E: ParseEnvironment, P: Pattern<I> + 'src>(
    pattern: P,
) -> AsParser<'src, I, P::Payload<'src>, E, Just<P>> {
    AsParser::from(Just::new(pattern))
}
pub fn satisfies<'src, I: Input + ?Sized, E: ParseEnvironment, F, Success, Failure>(
    kind: rowan::SyntaxKind,
    predicate: F,
) -> AsParser<'src, I, Success, E, Satisfies<F, Success, Failure>>
where
    F: Fn(&I, &InputItem<I::Token, I::Cursor>) -> Result<Success, Failure>,
    Failure: std::error::Error,
{
    AsParser::from(Satisfies::new::<I>(kind, predicate))
}
