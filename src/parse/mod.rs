//! Module for parsing-related traits, implementations and items.
//!
//! Refer to [`Parser`] for the central trait exported.

mod parser_trait;

mod diagnostics;
mod environment;
mod io;

pub use self::{diagnostics::*, environment::*, io::*, parser_trait::*};
