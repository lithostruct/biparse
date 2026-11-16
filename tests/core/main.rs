//! Testing of the correctness of the core combinators offered by Biparse ([`biparse::combinator`]).
//! 
//! # Where's [`Map`](biparse::combinator::Map)?
//! 
//! The transformation combinators [`Map`](biparse::combinator::Map) and [`IntoOwned`](biparse::combinator::IntoOwned)
//! are so commonly used within end-to-end integration tests and have such direct implementations
//! that it's kind of pointless to test them individually.
//! 
//! Rust's type system makes it clear to see that their implementations must be correct.
//! 
//! # Where's [`Associate`](biparse::combinator::Associate)?
//! 
//! There are some edge cases surrounding [`Recursive`](biparse::combinator::Recursive),
//! but for the most part it is pretty straightforward as long as the other combinators
//! thread [`ParseState`](biparse::parse::ParseState) correctly.
//! 
//! Most of [`Associate`](biparse::combinator::Associate)'s guarantees rely on CST association,
//! which is tested separately.

mod end;
mod fail;
mod just;
mod success;

mod or;
mod then;

mod recursive;
mod repetition;

mod commit_recover;
