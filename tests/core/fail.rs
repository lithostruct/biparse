//! Tests for the `Fail` sentinel combinator.

use biparse::prelude::*;
use proptest::{arbitrary::any, proptest};
use rowan::SyntaxKind;

proptest! {
    #[test]
    fn single_fail(s in any::<String>()) {
        let (state, result): (_, ParseOutput<(), DefaultParseEnvironment>) = fail()
            .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));

        assert_eq!(state.cursor.into_usize(), 0);
        assert!(matches!(result.ast, ParseOutcome::Speculative(_)));
    }

    #[test]
    fn fail_then_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state, result): (_, ParseOutput<&str, DefaultParseEnvironment>)
            = fail()
                .then(just((SyntaxKind(kind), pattern.clone())))
                .map(|(_, s)| s)
                .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));

        // No input should be consumed.
        assert_eq!(state.cursor.into_usize(), 0);
        assert!(matches!(result.ast, ParseOutcome::Speculative(_)));
    }

    #[test]
    fn just_then_fail(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (_, result): (_, ParseOutput<&str, DefaultParseEnvironment>)
            = just((SyntaxKind(kind), pattern))
                .then(fail()).map(|(s, _)| s)
                .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));

        assert!(matches!(result.ast, ParseOutcome::Speculative(_)));
        // Regression: this assert doesn't make sense and hence removed
        // because the `Just` will consume input if it succeeds regardless of downstream failure.
        // assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn fail_or_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<Either<(), &str>, DefaultParseEnvironment>)
            = just((SyntaxKind(kind), pattern.clone()))
                .map(Either::Right)
                .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));
        let (state2, result2): (_, ParseOutput<Either<(), &str>, DefaultParseEnvironment>)
            = fail()
                .or(just((SyntaxKind(kind), pattern)))
                .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));

        match (result1.ast, result2.ast) {
            (ParseOutcome::Success(val1), ParseOutcome::Success(val2))
                => assert_eq!(val1, val2),
            (ParseOutcome::Speculative(err1), ParseOutcome::Speculative(err2))
                => assert_eq!(err1.position, err2.position),
            (other1, other2) => {
                panic!("Test failed: left and right incompatible\n\tleft: {other1:?}\n\tright: {other2:?}");
            },
        }
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }

    #[test]
    fn just_or_fail(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, result1): (_, ParseOutput<Either<&str, ()>, DefaultParseEnvironment>)
            = just((SyntaxKind(kind), pattern.clone()))
                .map(Either::Left)
                .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));
        let (state2, result2): (_, ParseOutput<Either<&str, ()>, DefaultParseEnvironment>)
            = just((SyntaxKind(kind), pattern))
                .or(fail())
                .parse(s.as_str(), ParseState::<<str as Input>::Cursor, _>::new(s.as_str()));

        match (result1.ast, result2.ast) {
            (ParseOutcome::Success(val1), ParseOutcome::Success(val2))
                => assert_eq!(val1, val2),
            (ParseOutcome::Speculative(err1), ParseOutcome::Speculative(err2))
                => assert_eq!(err1.position, err2.position),
            (other1, other2) => {
                panic!("Test failed: left and right incompatible\n\tleft: {other1:?}\n\tright: {other2:?}");
            }
        }
        assert_eq!(state1.cursor, state2.cursor);
        assert_eq!(state1.supply, state2.supply);
    }
}
