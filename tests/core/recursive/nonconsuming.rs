//! Test regarding non-consuming input for recursive combinators.
//! These focus less on correctness (thought that is important too),
//! and more on termination of parsing.
//!
//! In other words, the failure case is timeout or spinning.

use biparse::prelude::*;
use proptest::{arbitrary::any, proptest};

proptest! {
    #[test]
    fn this_or_empty(s in any::<String>()) {
        let combinator: Recursive<dyn Parser<'_, str, (), DefaultParseEnvironment>> =
            Recursive::new(
                |this| this.or(success()).map(|_| ())
            );
        let (state, output) = combinator.parse(
            &s,
            ParseState::<str, DefaultParseEnvironment>::new(s.as_str())
        );
        assert!(matches!(output.ast, ParseOutcome::Success(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn empty_or_this(s in any::<String>()) {
        let combinator: Recursive<dyn Parser<'_, str, (), DefaultParseEnvironment>> =
            Recursive::new(
                |this| success().or(this).map(|_| ())
            );
        let (state, output) = combinator.parse(
            &s,
            ParseState::<str, DefaultParseEnvironment>::new(s.as_str())
        );
        assert!(matches!(output.ast, ParseOutcome::Success(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn this_rec(s in any::<String>()) {
        let combinator: Recursive<dyn Parser<'_, str, (), DefaultParseEnvironment>> =
            Recursive::new(
                |this| this
            );

        let (state, output) = combinator.parse(
            &s,
            ParseState::<str, DefaultParseEnvironment>::new(s.as_str())
        );
        assert!(matches!(output.ast, ParseOutcome::Fatal(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn mut_rec(s in any::<String>()) {
        type Rec<'p, 'a> = Recursive::<Indirect<'p, 'a, str, (), DefaultParseEnvironment>>;
        let mut rec_a : Rec<'_, '_> = Recursive::declare_as("rec_a");
        let mut rec_b : Rec<'_, '_> = Recursive::declare_as("rec_b");
        rec_a.define(rec_b.clone());
        rec_b.define(rec_a.weaken());

        let (state, output) = rec_a.parse(
            &s,
            ParseState::<str, DefaultParseEnvironment>::new(s.as_str())
        );
        assert!(matches!(output.ast, ParseOutcome::Fatal(_)));
        assert_eq!(state.cursor.into_usize(), 0);
    }
}
