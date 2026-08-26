//! Test regarding non-consuming input for recursive combinators.
//! These focus less on correctness (thought that is important too),
//! and moreso on termination of parsing.

use biparse::prelude::*;
use proptest::{arbitrary::any, proptest};

proptest! {
    #[test]
    fn this_or_empty(s in any::<String>()) {
        let combinator: Recursive<dyn Parser<'_, str, (), DefaultParseEnvironment>> =
            Recursive::recursive(
                |this| this.or(success()).map(|_| ())
            );
        let (state, result) = combinator.parse_from_start(&s);
        assert!(matches!(result.ast, ParseResult::Success(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn empty_or_this(s in any::<String>()) {
        let combinator: Recursive<dyn Parser<'_, str, (), DefaultParseEnvironment>> =
            Recursive::recursive(
                |this| success().or(this).map(|_| ())
            );
        let (state, result) = combinator.parse_from_start(&s);
        assert!(matches!(result.ast, ParseResult::Success(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn this_rec(s in any::<String>()) {
        let combinator: Recursive<dyn Parser<'_, str, (), DefaultParseEnvironment>> =
            Recursive::recursive(
                |this| this
            );
        let (state, result) = combinator.parse_from_start(&s);
        assert!(matches!(result.ast, ParseResult::Speculative(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }
}
