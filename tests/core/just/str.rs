//! Test regarding `Just` combinator for pattern matching on `str`-based inputs.

use biparse::prelude::*;
use proptest::{arbitrary::any, prop_oneof, proptest, strategy::Strategy};
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
    let (state, result) = combinator.parse(
        &s,
        ParseState::<<str as Input>::Cursor, usize>::new(s.as_str())
    );
    match result.ast {
        ParseOutcome::Success(matched) => {
            assert_eq!(matched, "a");
            assert_eq!(state.cursor.into_usize(), "a".len());
            Some(matched.to_owned())
        }
        ParseOutcome::Speculative(_) => {
            assert_eq!(state.cursor.into_usize(), 0);
            None
        }
        ParseOutcome::Committed(_) => unreachable!(),
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
    let (state, result) = combinator.parse(
        &s,
        ParseState::<<str as Input>::Cursor, usize>::new(s.as_str())
    );
    match result.ast {
        ParseOutcome::Success(matched) => {
            assert_eq!(matched, "a");
            assert_eq!(state.cursor.into_usize(), "a".len());
            Some(matched.to_owned())
        }
        ParseOutcome::Speculative(_) => {
            assert_eq!(state.cursor.into_usize(), 0);
            None
        }
        ParseOutcome::Committed(_) => unreachable!(),
    }
}

proptest! {
    #[test]
    fn str_a_stress(s in some_amount_of_a()) {
        str_a(s);
    }
}

// Non-panicking assertions.
proptest! {
    /// Tests that well-formed `Just` character patterns are non-panicking.
    #[test]
    fn arbitrary_char_pattern(pat in any::<char>(), s in any::<String>()) {
        let combinator =
            just::<'_, str, DefaultParseEnvironment, _>((SyntaxKind(0), pat))
                .to_owned();
        let (state, result) = combinator.parse(
            &s,
            ParseState::<<str as Input>::Cursor, usize>::new(s.as_str())
        );
        match result.ast {
            ParseOutcome::Success(matched) => {
                assert_eq!(matched, pat.to_string());
                assert_eq!(state.cursor.into_usize(), pat.len_utf8());
            }
            ParseOutcome::Speculative(_) => {
                assert_eq!(state.cursor.into_usize(), 0);
            }
            ParseOutcome::Committed(_) => unreachable!(),
        }
    }
    /// Tests that well-formed `Just` string patterns are non-panicking.
    #[test]
    fn arbitrary_string_pattern(pat in any::<String>(), s in any::<String>()) {
        let combinator =
            just::<'_, str, DefaultParseEnvironment, _>((SyntaxKind(0), pat.clone()))
                .to_owned();
        let (state, result) = combinator.parse(
            &s,
            ParseState::<<str as Input>::Cursor, usize>::new(s.as_str())
        );
        match result.ast {
            ParseOutcome::Success(matched) => {
                assert_eq!(matched, pat);
                assert_eq!(state.cursor.into_usize(), pat.len());
            }
            ParseOutcome::Speculative(_) => {
                assert_eq!(state.cursor.into_usize(), 0);
            }
            ParseOutcome::Committed(_) => unreachable!(),
        }
    }
}
