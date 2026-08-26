mod input_trait;
mod str;
#[cfg(feature = "regex")]
mod regex;

pub use self::{
    input_trait::*,
};