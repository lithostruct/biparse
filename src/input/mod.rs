mod input_trait;

mod cached_str;
mod composite_patterns;
mod str;

pub use self::{
    composite_patterns::{AllOf, AnyOf},
    input_trait::*,
    str::StrIndex,
};

#[cfg(feature = "regex")]
mod regex;

#[cfg(feature = "logos")]
mod logos;
#[cfg(feature = "logos")]
pub use logos::{LexerIndex, LexerStream, WithText};
