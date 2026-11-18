//! Tests for the [`Fail`](biparse::combinator::Fail) primitive combinator.
//!
//! # Note on Never Type
//!
//! While the never type [`!`](primitive@never) is indeed the "canonical" type that
//! [`Fail`](biparse::combinator::Fail) outputs,
//! the tests here (and everywhere `fail` is used) will default to unit [`()`](primitive@unit).
//!
//! This is mostly for backwards compatibility with pre-1.100 versions.

use biparse::prelude::*;
use proptest::{arbitrary::any, proptest};
use rowan::SyntaxKind;

proptest! {
    #[test]
    fn single_fail(s in any::<String>()) {
        let (state, output): (_, ParseOutput<(), environment::Rich>) = fail()
            .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        assert_eq!(state.cursor.into_usize(), 0);
        assert!(matches!(output.ast, ParseOutcome::Speculative(_)));
    }

    #[test]
    fn fail_then_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state, output): (_, ParseOutput<&str, environment::Rich>)
            = fail()
                .then(just((SyntaxKind(kind), pattern.clone())))
                .map(|((), s)| s)
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        // No input should be consumed.
        assert_eq!(state.cursor.into_usize(), 0);
        assert!(matches!(output.ast, ParseOutcome::Speculative(_)));
    }

    #[test]
    fn just_then_fail(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (_, output): (_, ParseOutput<&str, environment::Rich>)
            = just((SyntaxKind(kind), pattern))
                .then(fail()).map(|(s, ())| s)
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        assert!(matches!(output.ast, ParseOutcome::Speculative(_)));
        // Regression: this assert doesn't make sense and hence removed
        // because the `Just` will consume input if it succeeds regardless of downstream failure.
        // assert_eq!(state.cursor.into_usize(), 0);
    }

    #[test]
    fn fail_or_just(kind in any::<u16>(), pattern in ".{0,20}", s in any::<String>()) {
        let (state1, output1): (_, ParseOutput<&str, environment::Rich>)
            = just((SyntaxKind(kind), pattern.clone()))
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        let (state2, output2): (_, ParseOutput<&str, environment::Rich>)
            = fail()
                .or(just((SyntaxKind(kind), pattern)))
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

        match (output1.ast, output2.ast) {
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
        let (state1, result1): (_, ParseOutput<Either<&str, ()>, environment::Rich>)
            = just((SyntaxKind(kind), pattern.clone()))
                .map(Either::Left)
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));
        let (state2, result2): (_, ParseOutput<Either<&str, ()>, environment::Rich>)
            = just((SyntaxKind(kind), pattern))
                .either_or(fail())
                .parse(s.as_str(), ParseState::<str, environment::Rich>::new(s.as_str()));

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
