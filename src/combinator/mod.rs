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
    blanket::WithEnv,
    just::Just,
    map_to_owned::{Map, ToOwned},
    node_association::{Node, Associate},
    recursive::{Recursive, RecursiveStack},
    sentinel::{Empty, End, Fail},
};

// Constants and functions.

pub const empty: Empty = Empty;
pub const fail: Fail = Fail;
pub const end: End = End;

pub fn just<I: Input + ?Sized, P: Pattern<I>>(kind: rowan::SyntaxKind, pattern: P) -> Just<P> {
    Just::new(kind, pattern)
}

pub fn recursive<'src, F, I: Input + ?Sized, A, E: ParseEnvironment>(closure: F) -> Recursive<F>
where
    Recursive<F>: Parser<'src, I, A, E>,
{
    Recursive::new(closure)
}
