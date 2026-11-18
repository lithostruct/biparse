//! Module for testing of [`Or`](biparse::combinator::Or) combinators.
//!
//! # Notes:
//! `or` is such a fundamental operator in Biparse that plenty of tests have been done in other files.
//! This mostly targets the combinations of potential parse outcomes in the first or second payload.

use biparse::prelude::*;

#[test]
fn double_speculate() {
    let combinator = fail::<str, (), DefaultParseEnvironment>().or(fail::<_, (), _>());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Speculative(_)))
}

#[test]
fn first_success() {
    let combinator = success::<str, DefaultParseEnvironment>().or(fail::<_, (), _>().fatal());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Success(_)))
}

#[test]
fn speculate_or_succeed() {
    let combinator = fail::<str, (), DefaultParseEnvironment>().or(success());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Success(_)))
}

#[test]
fn first_committed() {
    let combinator = fail::<str, (), DefaultParseEnvironment>()
        .commit()
        .or(success());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Committed(_)))
}

#[test]
fn speculate_then_committed() {
    let combinator = fail::<str, (), DefaultParseEnvironment>().or(fail().commit());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Committed(_)))
}

#[test]
fn first_fatal() {
    let combinator = fail::<str, (), DefaultParseEnvironment>()
        .fatal()
        .or(success());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Fatal(_)))
}

#[test]
fn speculate_then_fatal() {
    let combinator = fail::<str, (), DefaultParseEnvironment>().or(fail().fatal());
    let (_, output) = combinator.parse("", ParseState::<str, DefaultParseEnvironment>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Fatal(_)))
}
