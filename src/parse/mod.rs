//! Crate containing traits and items relating to the central `Parser` trait.

mod parser;

mod environment;
mod io;

pub use self::{environment::*, io::*, parser::*};
