//! Module for wildcard importing of commonly-used structures and traits.

pub use crate::{
    // We can omit most of these, because `ParserExt` includes the intended construction mechanism.
    combinator::{As, Direct, Indirect, Recursive, With, end, fail, just, success},
    cst::{AssociatedGreenCst, AssociatedRedCst},
    failure::{Descriptor, FailureKind, ParseFailure, ParseHint},
    input::{Input, LineColumn, Pattern},
    parse::{
        DefaultParseEnvironment, ParseArtifact, ParseEnvironment, ParseOutcome, ParseOutput,
        ParseState, Parser, ParserExt, ParserIntoOwnedExt, RecursiveState, composite,
    },
};
pub use either::Either;
pub use rowan::SyntaxKind;

#[cfg(feature = "logos")]
pub use crate::input::logos::{LexerStream, WithText};
