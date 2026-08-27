//! Tests for the `End` sentinel combinator.

use biparse::prelude::*;
use rowan::SyntaxKind;

#[test]
fn single_end() {
    let no_input: ParseOutput<(), DefaultParseEnvironment> = end().parse_from_start("").1;
    assert!(matches!(no_input.ast, ParseResult::Success(_)));
    let (state, some_input): (_, ParseOutput<(), DefaultParseEnvironment>) =
        end().parse_from_start("lorem ipsum dolor sit amet");
    assert!(matches!(some_input.ast, ParseResult::Speculative(_)));
    assert_eq!(state.cursor.into_usize(), 0);
}

#[test]
fn just_then_end() {
    let combinator: WithEnv<DefaultParseEnvironment, _> = just(SyntaxKind(0), "a").then(end());

    let not_enough = combinator.parse_from_start("").1;
    let just_right = combinator.parse_from_start("a").1;
    let too_much = combinator.parse_from_start("ab").1;

    assert!(matches!(not_enough.ast, ParseResult::Speculative(_)));
    assert_eq!(just_right.ast, ParseResult::Success(("a", ())));
    assert!(matches!(too_much.ast, ParseResult::Speculative(_)));
}
