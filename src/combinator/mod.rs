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

use std::borrow::Cow;

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
        frame::{RecursiveId, RecursiveState},
    },
    repetition::Many,
};

// Constants and functions.

/// Constructs the primitive [`Success`] combinator that unconditionally succeeds on any input.
pub fn success<'src, I: Input + ?Sized, E: ParseEnvironment>() -> As<'src, I, (), E, Success> {
    As::from(Success)
}
/// Constructs the primitive [`Fail`] combinator that unconditionally fails on any input.
pub fn fail<'src, I: Input + ?Sized, A, E: ParseEnvironment>() -> As<'src, I, A, E, Fail> {
    As::from(Fail::new())
}
/// Constructs the primitive [`Fail`] combinator that unconditionally succeeds on any input, with the given message.
pub fn fail_with<'src, I: Input + ?Sized, A, E: ParseEnvironment>(
    msg: impl Into<Cow<'static, str>>,
) -> As<'src, I, A, E, Fail> {
    As::from(Fail::new_with(msg))
}
/// Constructs the primitive [`End`] combinator that succeeds iff end of input is reached.
pub fn end<'src, I: Input + ?Sized, E: ParseEnvironment>() -> As<'src, I, (), E, End> {
    As::from(End)
}

/// Constructs the [`Just`] combinator that matches the current input on some [`Pattern`].
pub fn just<'src, I: Input + ?Sized, E: ParseEnvironment, P: Pattern<'src, I> + 'src>(
    pattern: P,
) -> As<'src, I, P::Payload, E, Just<P>> {
    As::from(Just::new(pattern))
}
