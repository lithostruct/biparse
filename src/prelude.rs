//! Module for wildcard importing of commonly-used structures and traits.

pub use crate::{
    // We can omit most of these, because `ParserExt` includes the intended construction mechanism.
    combinator::{AsParser, Direct, Indirect, Recursive, WithEnv, end, fail, just, success},
    cst::{AssociatedGreenCst, AssociatedRedCst},
    failure::{Descriptor, FailureKind, ParseFailure, ParseHint},
    input::{Input, LineColumn, Pattern},
    parse::{
        DefaultParseEnvironment, ParseArtifact, ParseEnvironment, ParseOutcome, ParseOutput,
        ParseState, Parser, ParserExt, ParserOwnedExt, RecursiveState, composite,
    },
};
pub use either::Either;

#[cfg(feature = "logos")]
pub use crate::input::logos::{LexerStream, WithText};
