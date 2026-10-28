//! Module for [`ParseFailure`] trait definition, implementors, and related types.

mod failure_trait;
mod hint_descriptor;

pub use {
    failure_trait::*,
    hint_descriptor::{Descriptor, ParseHint},
};
