mod input_trait;

mod str;
pub use self::input_trait::*;

#[cfg(feature = "regex")]
mod regex;

#[cfg(feature = "logos")]
mod logos;
#[cfg(feature = "logos")]
pub use logos::{LexerStream, LexerIndex};
