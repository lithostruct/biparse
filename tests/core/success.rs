//! Tests for the `Success` sentinel combinator.

use biparse::prelude::*;
use proptest::{arbitrary::any, proptest};
use rowan::SyntaxKind;

proptest! {
    #[test]
    fn single_empty(s in any::<String>()) {
        let (state, result): (_, ParseOutput<(), DefaultParseEnvironment>) = success().parse_from_start(s.as_str());
        assert_eq!(state.cursor.into_usize(), 0);
        assert_eq!(result.ast, ParseResult::Success(()));
    }

    #[test]
    fn empty_then_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<&str, DefaultParseEnvironment>)
            = just(SyntaxKind(kind), pattern.clone())
                .parse_from_start(s.as_str());
        let (state2, result2): (_, ParseOutput<&str, DefaultParseEnvironment>)
            = success().then(just(SyntaxKind(kind), pattern)).map(|(_, s)| s)
                .parse_from_start(s.as_str());
        assert_eq!(result1, result2);
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn just_then_empty(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<&str, DefaultParseEnvironment>)
            = just(SyntaxKind(kind), pattern.clone())
                .parse_from_start(s.as_str());
        let (state2, result2): (_, ParseOutput<&str, DefaultParseEnvironment>)
            = just(SyntaxKind(kind), pattern).then(success()).map(|(s, _)| s)
                .parse_from_start(s.as_str());
        assert_eq!(result1, result2);
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn empty_or_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<Either<(), &str>, DefaultParseEnvironment>)
            = success().map(Either::Left)
                .parse_from_start(s.as_str());
        let (state2, result2): (_, ParseOutput<Either<(), &str>, DefaultParseEnvironment>)
            = success().or(just(SyntaxKind(kind), pattern))
                .parse_from_start(s.as_str());
        assert_eq!(result1, result2);
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn just_or_empty(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        // Currently no way of recovering from a failed `just` to get an `Either::Right(())`,
        // so this is more of a stress test than anything.
        let (state, result): (_, ParseOutput<Either<&str, ()>, DefaultParseEnvironment>)
            = just(SyntaxKind(kind), pattern.clone()).or(success())
                .parse_from_start(s.as_str());
        let ParseResult::Success(inner) = result.ast else {panic!()};
        match inner {
            Either::Left(atom) => assert_eq!(state.cursor.into_usize(), atom.len()),
            Either::Right(()) => assert_eq!(state.cursor.into_usize(), 0),
        }
    }
}
