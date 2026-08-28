mod input_trait;

mod str;

#[cfg(feature = "logos")]
mod logos;
#[cfg(feature = "regex")]
mod regex;

pub use self::input_trait::*;
