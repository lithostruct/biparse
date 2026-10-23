//! Tests for the `End` sentinel combinator.

use biparse::{input::StrIndex, prelude::*};
use num_derive::{FromPrimitive, ToPrimitive};
use rowan::SyntaxKind;

#[test]
fn single_end() {
    const NO_TEXT: &str = "";
    let no_input: ParseOutput<(), DefaultParseEnvironment> = end()
        .parse(
            NO_TEXT,
            ParseState::<<str as Input>::Cursor, usize>::new(NO_TEXT),
        )
        .1;
    assert!(matches!(no_input.ast, ParseOutcome::Success(_)));

    const SOME_TEXT: &str = "lorem ipsum dolor sit amet";
    let (state, some_input): (_, ParseOutput<(), DefaultParseEnvironment>) = end().parse(
        SOME_TEXT,
        ParseState::<<str as Input>::Cursor, usize>::new(SOME_TEXT),
    );
    assert!(matches!(some_input.ast, ParseOutcome::Speculative(_)));
    assert_eq!(state.cursor.into_usize(), 0);
}

#[test]
fn just_then_end() {
    let combinator: WithEnv<DefaultParseEnvironment, _> = just((SyntaxKind(0), "a")).then(end());

    let not_enough = combinator.parse_all("");
    let just_right = combinator.parse_all("a");
    let too_much = combinator.parse_all("ab");

    assert!(not_enough.is_err());
    assert_eq!(just_right.expect("success!").into_ast_only(), ("a", ()));
    assert!(too_much.is_err());
}

/// Specific to non-contiguous input:
/// If a non-empty input produces no meaningful tokens, the CST actualized should still be complete.
/// 
/// This relates to logos-based input and `SkippingStr<'_>`.
#[test]
fn all_trivia_end() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    struct Kind(u16);
    impl rowan::Language for Kind {
        type Kind = Self;
        fn kind_from_raw(raw: SyntaxKind) -> Self::Kind {
            Kind(raw.0)
        }
        fn kind_to_raw(kind: Self::Kind) -> SyntaxKind {
            SyntaxKind(kind.0)
        }
    }
    impl From<Kind> for SyntaxKind {
        fn from(value: Kind) -> Self {
            <Kind as rowan::Language>::kind_to_raw(value)
        }
    }

    let ends : AsParser<'_, SkippingStr<'_>, (), DefaultParseEnvironment, _> = end();
    
    const ALL_TRIVIA: &str = "     ";
    let input = SkippingStr::whitespace(ALL_TRIVIA, Kind(0));
    let (_, result) = ends.parse(&input, ParseState::<StrIndex, usize>::new(&input));

    assert!(result.ast.into_result().is_ok());
    let tree = result.cst.canonize_with(Kind(1)).0;
    assert_eq!(tree.text(), ALL_TRIVIA)
}
