//! Module for parsing-related traits, implementations and items.
//!
//! Refer to [`Parser`] for the central trait exported.

mod parser_trait;

pub mod environment;
mod io;

pub use self::{
    io::*,
    parser_trait::{derived, *},
};
