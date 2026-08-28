//! Test regarding `Just` combinator for pattern matching.

use biparse::prelude::*;
use proptest::{prop_oneof, proptest, strategy::Strategy};
use rowan::SyntaxKind;

fn some_amount_of_a() -> impl Strategy<Value = String> {
    prop_oneof![
        "[^a]{0, 100}",
        (0..50_usize).prop_map(|num: usize| "a".repeat(num)),
    ]
}

fn char_a(s: String) -> Option<String> {
    let combinator: AsParser<'_, str, &str, DefaultParseEnvironment, _> =
        just((SyntaxKind(0), 'a'));
    let (state, result) = combinator.parse_from_start(&s);
    match result.ast {
        ParseResult::Success(matched) => {
            assert_eq!(matched, "a");
            assert_eq!(state.cursor.into_usize(), "a".len());
            assert_eq!(result.cst.green.len(), 1);
            Some(matched.to_owned())
        }
        ParseResult::Speculative(_) => {
            assert_eq!(state.cursor.into_usize(), 0);
            assert_eq!(result.cst.green.len(), 0);
            None
        }
        ParseResult::Committed(_) => unreachable!(),
    }
}
proptest! {
    #[test]
    fn char_a_stress(s in some_amount_of_a()) {
        char_a(s);
    }
}

fn str_a(s: String) -> Option<String> {
    let combinator: AsParser<'_, str, &str, DefaultParseEnvironment, _> =
        just((SyntaxKind(0), "a"));
    let (state, result) = combinator.parse_from_start(&s);
    match result.ast {
        ParseResult::Success(matched) => {
            assert_eq!(matched, "a");
            assert_eq!(state.cursor.into_usize(), "a".len());
            assert_eq!(result.cst.green.len(), 1);
            Some(matched.to_owned())
        }
        ParseResult::Speculative(_) => {
            assert_eq!(state.cursor.into_usize(), 0);
            assert_eq!(result.cst.green.len(), 0);
            None
        }
        ParseResult::Committed(_) => unreachable!(),
    }
}
proptest! {
    #[test]
    fn str_a_stress(s in some_amount_of_a()) {
        str_a(s);
    }
}
