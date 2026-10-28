use biparse::prelude::*;

// Assert that success is unaffected,
// and speculative failure is promoted to committed failure.
#[test]
fn commit_semantics() {
    assert!(success::<str, DefaultParseEnvironment>().commit().parse_all("").is_ok());
    assert!(fail::<str, DefaultParseEnvironment>().commit().parse_all("").is_err());

    let combinator = fail::<str, DefaultParseEnvironment>().commit();
    let (_, output) = combinator.parse("", ParseState::<str, _>::new(""));
    assert!(matches!(output.ast, ParseOutcome::Committed(_)));
}

#[test]
fn commit_within_then() {
    todo!()
}
#[test]
fn commit_within_or() {
    todo!()
}
#[test]
fn commit_within_recursive() {
    todo!()
}
