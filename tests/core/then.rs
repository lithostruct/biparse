//! Notes:
//!
//! `then` is such a fundamental operator in Biparse that plenty of tests have been done in other files.
//! This mostly targets the combinations of potential parse outcomes in the first or second payload.

use biparse::prelude::*;

#[test]
fn double_success() {
    let combinator = success::<_, DefaultParseEnvironment>().then(success());
    combinator.parse_all("").expect("Success!");
}

#[test]
fn success_then_speculative() {
    todo!()
}

#[test]
fn success_then_committed() {
    todo!()
}

#[test]
fn success_then_fatal() {
    todo!()
}
