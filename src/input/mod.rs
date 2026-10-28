//! Module for [`Input`] trait definition, implementors, and related types.

mod input_trait;

mod cached_str;
mod composite_patterns;
mod skipping_str;
pub mod str;

pub use self::{
    composite_patterns::{AllOf, AnyOf, Matches},
    input_trait::*,
};

#[cfg(feature = "regex")]
mod regex;

#[cfg(feature = "logos")]
pub mod logos;
