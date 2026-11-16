//! Module for testing of [`Then`](biparse::combinator::Then) combinators.
//! 
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
fn first_speculate() {
    let combinator = fail::<_, !, DefaultParseEnvironment>()
        .then(success());
    let (_, output) = combinator.parse("",
        ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Speculative(_)))
}

#[test]
fn success_then_speculative() {
    let combinator = success::<_, DefaultParseEnvironment>()
        .then(fail::<_, !, _>());
    let (_, output) = combinator.parse("",
        ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Speculative(_)))
}

#[test]
fn first_committed() {
    let combinator = fail::<_, !, DefaultParseEnvironment>().commit()
        .then(success());
    let (_, output) = combinator.parse("",
        ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Committed(_)))
}

#[test]
fn success_then_committed() {
    let combinator = success::<_, DefaultParseEnvironment>()
        .then(fail::<_, !, _>().commit());
    let (_, output) = combinator.parse("",
        ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Committed(_)))
}

#[test]
fn first_fatal() {
    let combinator = fail::<_, !, DefaultParseEnvironment>().fatal()
        .then(success());
    let (_, output) = combinator.parse("",
        ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Fatal(_)))
}

#[test]
fn success_then_fatal() {
    let combinator = success::<_, DefaultParseEnvironment>()
        .then(fail::<_, !, _>().fatal());
    let (_, output) = combinator.parse("",
        ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Fatal(_)))
}