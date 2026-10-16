//! Tests for the `End` sentinel combinator.

use biparse::prelude::*;
use rowan::SyntaxKind;

#[test]
fn single_end() {
    const NO_TEXT : &str = "";
    let no_input: ParseOutput<(), DefaultParseEnvironment> = end().parse(
            NO_TEXT,
            ParseState::<<str as Input>::Cursor, usize>::new(NO_TEXT)
        ).1;
    assert!(matches!(no_input.ast, ParseOutcome::Success(_)));

    const SOME_TEXT : &str = "lorem ipsum dolor sit amet";
    let (state, some_input): (_, ParseOutput<(), DefaultParseEnvironment>) =
        end().parse(
            SOME_TEXT,
            ParseState::<<str as Input>::Cursor, usize>::new(SOME_TEXT)
        );
    assert!(matches!(some_input.ast, ParseOutcome::Speculative(_)));
    assert_eq!(state.cursor.into_usize(), 0);
}

#[test]
fn just_then_end() {
    let combinator: WithEnv<DefaultParseEnvironment, _> =
        just((SyntaxKind(0), "a")).then(end());

    let not_enough = combinator.parse_all("");
    let just_right = combinator.parse_all("a");
    let too_much = combinator.parse_all("ab");

    assert!(not_enough.is_err());
    assert_eq!(just_right.expect("success!").into_ast_only(), ("a", ()));
    assert!(too_much.is_err());
}
