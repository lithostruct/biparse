//! Module relating to parser combinators.
#![allow(non_upper_case_globals)]

mod blanket;

mod binary;
mod commit_recover;
mod just;
mod map_into_owned;
mod node_association;
mod primitive;
mod recursive;
mod repetition;

use crate::{
    input::{Input, Pattern},
    parse::ParseEnvironment,
};

pub use self::{
    binary::{Or, Then},
    blanket::{AsParser, WithEnv},
    commit_recover::{Commit, Recover},
    just::Just,
    map_into_owned::{IntoOwned, Map, TryMap},
    node_association::{Associate, Node},
    primitive::{End, Fail, Success},
    recursive::{
        combinator::{Direct, Indirect, Recursive},
        frame::RecursiveState,
    },
    repetition::Many,
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

pub fn just<'src, I: Input + ?Sized, E: ParseEnvironment, P: Pattern<'src, I> + 'src>(
    pattern: P,
) -> AsParser<'src, I, P::Payload, E, Just<P>> {
    AsParser::from(Just::new(pattern))
}
