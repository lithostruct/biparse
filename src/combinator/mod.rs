//! Module for parser combinator types, which implement [`Parser`](crate::parse::Parser).

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
    blanket::{As, With},
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

pub fn success<'src, I: Input + ?Sized, E: ParseEnvironment>() -> As<'src, I, (), E, Success> {
    As::from(Success)
}
pub fn fail<'src, I: Input + ?Sized, E: ParseEnvironment>() -> As<'src, I, (), E, Fail> {
    As::from(Fail)
}
pub fn end<'src, I: Input + ?Sized, E: ParseEnvironment>() -> As<'src, I, (), E, End> {
    As::from(End)
}

pub fn just<'src, I: Input + ?Sized, E: ParseEnvironment, P: Pattern<'src, I> + 'src>(
    pattern: P,
) -> As<'src, I, P::Payload, E, Just<P>> {
    As::from(Just::new(pattern))
}
